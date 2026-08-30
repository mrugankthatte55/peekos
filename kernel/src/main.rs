//! The PeekOS kernel.
//!
//! Boots under QEMU and prints a banner over the serial console, then halts —
//! there is nothing else for it to do yet. Under `cargo xtask test` it asks
//! QEMU to exit instead, so the boot can be checked automatically.

#![no_std]
#![no_main]

#[cfg(feature = "boot-test")]
mod qemu;
mod serial;

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;

// Wires `kernel_main` up as the entry point the `bootloader` crate jumps to,
// with the ABI it expects. Replaces the usual `_start` / `no_mangle` dance.
entry_point!(kernel_main);

/// First Rust code that runs in PeekOS. `boot_info` is everything the
/// bootloader discovered for us (memory map, framebuffer, …) — unused so far.
fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    // The bootloader hands off with interrupts enabled. We have no interrupt
    // descriptor table yet, so the first timer tick would fault straight into
    // a triple fault — keep them masked until the kernel builds an IDT.
    unsafe { core::arch::asm!("cli", options(nomem, nostack)) }

    serial::init();

    serial_println!();
    serial_println!("PeekOS - a glass box");
    serial_println!("[boot] serial console up (COM1, 38400 8N1)");
    serial_println!("[boot] interrupts masked; no IDT yet");

    finish()
}

/// How this boot ends once there is nothing left to do.
#[cfg(not(feature = "boot-test"))]
fn finish() -> ! {
    serial_println!("[boot] nothing left to do - halting");
    halt_loop()
}

/// Under `cargo xtask test`: report a clean boot and ask QEMU to exit.
#[cfg(feature = "boot-test")]
fn finish() -> ! {
    serial_println!("[test] boot ok");
    qemu::exit(qemu::EXIT_SUCCESS)
}

/// Runs when anything in the kernel panics. Nothing beneath us to unwind into,
/// so we report why and then stop (or fail the test).
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::emergency_print(format_args!("\n*** KERNEL PANIC ***\n{info}\n"));

    #[cfg(feature = "boot-test")]
    qemu::exit(qemu::EXIT_FAILED);

    #[cfg(not(feature = "boot-test"))]
    halt_loop()
}

/// Park the CPU. `hlt` idles it until the next interrupt rather than spinning
/// hot; the loop is because an interrupt will eventually wake it back up.
#[cfg(not(feature = "boot-test"))]
fn halt_loop() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}
