# PeekOS — design

## What this is

PeekOS is a small operating system for x86-64, written in Rust, built to be
understood. During development it runs only under emulation (QEMU), never on
real hardware.

It is a learning project first. The goal is not to ship a product — it is to
build, by hand and in small steps, enough of a real OS to understand how one
works, and to end up with an artifact small and coherent enough to hold in
your head.

## The "why": the Glass Box

Most operating systems are opaque. They are enormous, and their internal state
— what the scheduler is about to do, what each page of physical memory is for,
why a particular interrupt fired — is invisible without specialized tooling, if
it can be seen at all.

PeekOS takes the opposite stance. **Every subsystem is built to be observed from
inside the running system.** The scheduler can show you its run queue. The frame
allocator can list every physical frame it has handed out. The page tables are
walkable from the shell. Interrupts are counted and inspectable. The system does
not hide its mechanism — it exhibits it.

The name is from `PEEK`, the BASIC primitive that read a byte straight out of
memory. Same spirit, decades later: look directly at the machine.

### The design rule

> No subsystem is "done" until you can observe its live state from the shell.

We apply this to every feature. Add a heap allocator, add a `peek heap` command
in the same breath. If you cannot see it, it is not finished. This keeps the
system legible as it grows, and it makes the OS its own best teaching tool.

## Scope

We build in small steps and keep building. There is no fixed feature ceiling —
the project goes as far as it stays interesting and instructive.

Deliberately **out of scope**, at least for a long time:

- Running on real hardware (QEMU only)
- SMP / multicore
- Networking
- USB
- A graphical desktop / windowing system
- ACPI power management
- Multi-user security

Any one of these could sink the project. If one becomes the interesting thing
later, that is a deliberate decision made at the time — not a default we drifted
into.

## Platform & toolchain decisions

| Decision | Choice | Why |
|---|---|---|
| Architecture | x86-64 | The most documented path; leaves the door open to real hardware later. |
| Bootloader | [`bootloader`](https://crates.io/crates/bootloader) crate (0.11.x) | Pure Rust. Builds the bootable disk image through Cargo alone — no GRUB, no `xorriso`, no C toolchain — so the repo builds identically on macOS and Windows. Hands the kernel a memory map and a framebuffer. |
| Emulator | QEMU | Standard, safe, and exposes a GDB stub for breakpoint-debugging the kernel. |
| Kernel target | `x86_64-unknown-none` | Built-in bare-metal target: red zone disabled, SSE off, soft-float — already the right settings for a kernel, so no custom target JSON. |
| Toolchain | pinned nightly | Needed soon for the `x86-interrupt` calling convention. Pinned by date in `rust-toolchain.toml` so every machine and CI match. |
| Build orchestration | `xtask` (a Rust program) | `cargo xtask build` / `run` / `test` behave the same on macOS and Windows. No Makefiles, no shell scripts. The pipeline is readable Rust, which fits the Glass Box. |

## Repository layout

```
peekos/
├── kernel/              the OS itself — #![no_std], boots on x86_64-unknown-none
│   └── src/main.rs
├── xtask/               build & run orchestrator (runs on the host)
│   └── src/main.rs
├── docs/design.md       this document
├── rust-toolchain.toml  pinned nightly + components + target
├── .cargo/config.toml   the `cargo xtask` alias
└── Cargo.toml           workspace; default-members = ["xtask"]
```

## Roadmap

Rough sequence. Each step is small enough to review in one pull request and
ends with something you can see.

1. **Scaffold** — workspace, pinned toolchain, this doc. Kernel compiles for
   bare metal and defines its entry point. *(done)*
2. **Boot & print** — `bootloader` wired into xtask, panic handler, serial
   output. `cargo xtask run` boots PeekOS in QEMU and prints a banner. *(done)*
3. **Dev loop** — a self-terminating `cargo xtask test`; CI runs it headless on
   every pull request.
4. **The `peek>` shell** — read serial input, a command loop. The lens for
   everything after.
5. **CPU exceptions** — an interrupt descriptor table; `peek irq` shows counts.
6. **Hardware interrupts** — timer and keyboard.
7. **Memory** — read the bootloader's page tables; a frame allocator
   (`peek frames`); a heap (`peek heap`), after which `Box` / `Vec` work.
8. **Multitasking** — cooperative tasks first; `peek tasks`.
9. **onward** — userspace, a filesystem, … decided when we get there.

## Working notes

- Development happens on macOS and Windows against the same repo. The pinned
  toolchain and the xtask orchestrator are what keep the two in sync.
- `Cargo.lock` is committed — this is a binary project, so exact dependency
  versions are part of the build.
