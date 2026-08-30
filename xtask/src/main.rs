//! PeekOS build & run orchestrator.
//!
//! Runs on the host (your Mac or Windows box), not in the OS. Everything you do
//! with PeekOS goes through `cargo xtask <command>`, so the build pipeline is
//! plain readable Rust rather than a Makefile.
//!
//!   cargo xtask build         compile the kernel for bare metal
//!   cargo xtask run [--gui]   build a bootable image and boot it in QEMU
//!   cargo xtask test          boot headless, assert it came up, then exit

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

/// The kernel is always built for this bare-metal target, never the host.
const KERNEL_TARGET: &str = "x86_64-unknown-none";

/// QEMU's `isa-debug-exit` device turns a guest port write of `v` into a
/// process exit code of `(v << 1) | 1`. The kernel writes `0x10` on a clean
/// boot (see `kernel/src/qemu.rs`), so we expect exactly this.
const QEMU_EXIT_ON_CLEAN_BOOT: i32 = (0x10 << 1) | 1; // 33

/// A headless boot that takes longer than this is considered hung.
const TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Marker the kernel prints to serial just before asking QEMU to exit.
const BOOT_OK_MARKER: &str = "[test] boot ok";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("build") => match build_kernel(&[]) {
            Ok(elf) => {
                report_elf(&elf);
                ExitCode::SUCCESS
            }
            Err(()) => ExitCode::FAILURE,
        },
        // `--gui` opens QEMU's window (you see the bootloader draw to the
        // framebuffer); without it QEMU runs headless and only the serial
        // console comes back to this terminal.
        Some("run") => run(args.iter().any(|a| a == "--gui")),
        Some("test") => test(),
        Some(other) => usage(&format!("unknown command: {other}")),
        None => usage("no command given"),
    }
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("xtask: {problem}");
    eprintln!("usage: cargo xtask build");
    eprintln!("       cargo xtask run [--gui]");
    eprintln!("       cargo xtask test");
    ExitCode::FAILURE
}

/// Compile the kernel crate for `x86_64-unknown-none`, with any extra cargo
/// args (e.g. `--features ...`). Returns the path to the ELF.
fn build_kernel(extra: &[&str]) -> Result<PathBuf, ()> {
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["build", "--package", "kernel", "--target", KERNEL_TARGET]);
    cmd.args(extra);

    if !cmd.status().expect("failed to run cargo").success() {
        return Err(());
    }

    let elf = kernel_elf_path();
    if !elf.exists() {
        eprintln!("cargo reported success but {} is missing", elf.display());
        return Err(());
    }
    Ok(elf)
}

/// Build a BIOS disk image from `kernel` and return its path.
fn build_image(kernel: &Path) -> Result<PathBuf, ()> {
    let image = target_dir().join("peekos").join("peekos-bios.img");
    std::fs::create_dir_all(image.parent().unwrap()).expect("create image dir");

    println!("xtask: building BIOS disk image -> {}", image.display());
    if let Err(e) = bootloader::BiosBoot::new(kernel).create_disk_image(&image) {
        eprintln!("xtask: failed to build disk image: {e:?}");
        return Err(());
    }
    Ok(image)
}

/// QEMU on Windows is happier with forward slashes in path arguments.
fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Build a BIOS disk image from the kernel and boot it in QEMU.
fn run(gui: bool) -> ExitCode {
    let Ok(kernel) = build_kernel(&[]) else {
        return ExitCode::FAILURE;
    };
    let Ok(image) = build_image(&kernel) else {
        return ExitCode::FAILURE;
    };

    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.args([
        "-drive",
        &format!("format=raw,file={}", slashed(&image)),
        "-serial",
        "stdio",
        "-no-reboot",
    ]);
    if gui {
        println!("xtask: booting QEMU in a window - serial also mirrored below, Ctrl+C to quit\n");
    } else {
        qemu.args(["-display", "none"]);
        println!("xtask: booting QEMU headless - serial output below, Ctrl+C to quit\n");
    }

    match qemu.status() {
        Ok(_) => ExitCode::SUCCESS, // QEMU's exit code isn't meaningful to us here
        Err(e) => {
            eprintln!("xtask: could not launch qemu-system-x86_64: {e}");
            eprintln!("       is QEMU installed and on PATH?");
            ExitCode::FAILURE
        }
    }
}

/// Boot the kernel headless with the `boot-test` feature, which makes it write
/// a success code to QEMU's `isa-debug-exit` device once it is up. Pass only if
/// QEMU exits with that exact code *and* the boot marker reached the serial log.
fn test() -> ExitCode {
    let Ok(kernel) = build_kernel(&["--features", "boot-test"]) else {
        return ExitCode::FAILURE;
    };
    let Ok(image) = build_image(&kernel) else {
        return ExitCode::FAILURE;
    };

    let serial_log = target_dir().join("peekos").join("test-serial.txt");
    let _ = std::fs::remove_file(&serial_log);

    println!(
        "xtask: boot test - QEMU headless, {}s timeout\n",
        TEST_TIMEOUT.as_secs()
    );
    let spawned = Command::new("qemu-system-x86_64")
        .args([
            "-drive",
            &format!("format=raw,file={}", slashed(&image)),
            "-chardev",
            &format!("file,id=serial0,path={}", slashed(&serial_log)),
            "-serial",
            "chardev:serial0",
            "-display",
            "none",
            "-device",
            "isa-debug-exit,iobase=0xf4,iosize=0x04",
            "-no-reboot",
        ])
        .stdout(Stdio::null())
        .spawn();

    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            eprintln!("xtask: could not launch qemu-system-x86_64: {e}");
            eprintln!("       is QEMU installed and on PATH?");
            return ExitCode::FAILURE;
        }
    };

    let status = match child.wait_timeout(TEST_TIMEOUT).expect("wait on qemu") {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            dump_serial(&serial_log);
            eprintln!(
                "\nxtask: FAIL - kernel did not exit within {}s (hung?)",
                TEST_TIMEOUT.as_secs()
            );
            return ExitCode::FAILURE;
        }
    };

    let serial = std::fs::read_to_string(&serial_log).unwrap_or_default();
    let code = status.code();
    let marker_seen = serial.contains(BOOT_OK_MARKER);

    dump_serial(&serial_log);
    println!(
        "\nxtask: qemu exit code {code:?} (want {QEMU_EXIT_ON_CLEAN_BOOT}), \
         boot marker {}",
        if marker_seen { "seen" } else { "MISSING" }
    );

    if code == Some(QEMU_EXIT_ON_CLEAN_BOOT) && marker_seen {
        println!("xtask: PASS");
        ExitCode::SUCCESS
    } else {
        eprintln!("xtask: FAIL");
        ExitCode::FAILURE
    }
}

fn dump_serial(path: &Path) {
    if let Ok(s) = std::fs::read_to_string(path) {
        println!("--- serial ---\n{}--- end serial ---", s);
    }
}

fn report_elf(elf: &Path) {
    if let Ok(m) = std::fs::metadata(elf) {
        println!("\nkernel: {} ({} bytes)", elf.display(), m.len());
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ always has a parent")
        .to_path_buf()
}

fn target_dir() -> PathBuf {
    workspace_root().join("target")
}

fn kernel_elf_path() -> PathBuf {
    target_dir().join(KERNEL_TARGET).join("debug").join("kernel")
}
