# PeekOS

A small, legible operating system for x86-64, written in Rust, developed
entirely under QEMU. A learning project — see [docs/design.md](docs/design.md)
for what it is and why.

## Requirements

- Rust — the pinned nightly in `rust-toolchain.toml` installs on first build
- [QEMU](https://www.qemu.org/) with `qemu-system-x86_64` on `PATH`
- Windows only: the MSVC C++ build tools (to build `xtask`)

## Usage

    cargo xtask build     # compile the kernel for x86_64-unknown-none

More commands (`run`, `test`) arrive as the project grows.

## Layout

| Path | What |
|------|------|
| `kernel/` | the OS itself (`#![no_std]`, boots on `x86_64-unknown-none`) |
| `xtask/`  | build & run orchestrator, runs on the host |
| `docs/`   | design notes |
