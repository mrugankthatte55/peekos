//! PeekOS build & run orchestrator.
//!
//! Runs on the host (your Mac or Windows box), not in the OS. Everything you do
//! with PeekOS goes through `cargo xtask <command>`, so the build pipeline is
//! plain readable Rust rather than a Makefile.
//!
//! Increment 1: only `build` exists, and it just compiles the kernel. Building
//! a bootable image and launching QEMU come next.

use std::path::PathBuf;
use std::process::{Command, ExitCode};

/// The kernel is always built for this bare-metal target, never the host.
const KERNEL_TARGET: &str = "x86_64-unknown-none";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("build") => build_kernel(),
        Some(other) => usage(&format!("unknown command: {other}")),
        None => usage("no command given"),
    }
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("xtask: {problem}");
    eprintln!("usage: cargo xtask build");
    ExitCode::FAILURE
}

/// Compile the kernel crate for `x86_64-unknown-none` by shelling out to cargo.
fn build_kernel() -> ExitCode {
    let status = Command::new(env!("CARGO"))
        .args(["build", "--package", "kernel", "--target", KERNEL_TARGET])
        .status()
        .expect("failed to run cargo");

    if !status.success() {
        return ExitCode::FAILURE;
    }

    let elf = kernel_elf_path();
    match std::fs::metadata(&elf) {
        Ok(m) => {
            println!("\nkernel: {} ({} bytes)", elf.display(), m.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("built, but cannot find {}: {e}", elf.display());
            ExitCode::FAILURE
        }
    }
}

/// `<workspace root>/target/x86_64-unknown-none/debug/kernel`.
fn kernel_elf_path() -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ always has a parent")
        .to_path_buf();
    workspace_root
        .join("target")
        .join(KERNEL_TARGET)
        .join("debug")
        .join("kernel")
}
