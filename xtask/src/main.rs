//! PeekOS build & run orchestrator.
//!
//! Runs on the host (your Mac or Windows box), not in the OS. Everything you do
//! with PeekOS goes through `cargo xtask <command>`, so the build pipeline is
//! plain readable Rust rather than a Makefile.
//!
//!   cargo xtask build    compile the kernel for bare metal
//!   cargo xtask run      build a bootable image and boot it in QEMU

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// The kernel is always built for this bare-metal target, never the host.
const KERNEL_TARGET: &str = "x86_64-unknown-none";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("build") => match build_kernel() {
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
        Some(other) => usage(&format!("unknown command: {other}")),
        None => usage("no command given"),
    }
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("xtask: {problem}");
    eprintln!("usage: cargo xtask build");
    eprintln!("       cargo xtask run [--gui]");
    ExitCode::FAILURE
}

/// Compile the kernel crate for `x86_64-unknown-none`. Returns the ELF path.
fn build_kernel() -> Result<PathBuf, ()> {
    let status = Command::new(env!("CARGO"))
        .args(["build", "--package", "kernel", "--target", KERNEL_TARGET])
        .status()
        .expect("failed to run cargo");

    if !status.success() {
        return Err(());
    }

    let elf = kernel_elf_path();
    if !elf.exists() {
        eprintln!("cargo reported success but {} is missing", elf.display());
        return Err(());
    }
    Ok(elf)
}

/// Build a BIOS disk image from the kernel and boot it in QEMU.
fn run(gui: bool) -> ExitCode {
    let Ok(kernel) = build_kernel() else {
        return ExitCode::FAILURE;
    };

    let image = target_dir().join("peekos").join("peekos-bios.img");
    std::fs::create_dir_all(image.parent().unwrap()).expect("create image dir");

    println!("xtask: building BIOS disk image -> {}", image.display());
    if let Err(e) = bootloader::BiosBoot::new(&kernel).create_disk_image(&image) {
        eprintln!("xtask: failed to build disk image: {e:?}");
        return ExitCode::FAILURE;
    }

    // QEMU on Windows is happier with forward slashes inside -drive.
    let image_arg = format!("format=raw,file={}", image.to_string_lossy().replace('\\', "/"));

    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.args(["-drive", &image_arg, "-serial", "stdio", "-no-reboot"]);
    if gui {
        println!("xtask: booting QEMU in a window - serial also mirrored below, Ctrl+C to quit\n");
    } else {
        // Headless: the kernel only speaks over serial right now, and the
        // framebuffer is a black rectangle once our kernel takes over anyway.
        qemu.args(["-display", "none"]);
        println!("xtask: booting QEMU headless - serial output below, Ctrl+C to quit\n");
    }

    match qemu.status() {
        Ok(_) => ExitCode::SUCCESS, // QEMU's exit code isn't meaningful to us yet
        Err(e) => {
            eprintln!("xtask: could not launch qemu-system-x86_64: {e}");
            eprintln!("       is QEMU installed and on PATH?");
            ExitCode::FAILURE
        }
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
