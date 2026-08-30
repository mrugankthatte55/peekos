//! The PeekOS kernel.
//!
//! Increment 2: boots under QEMU and prints a banner over the serial console.
//! Then it has nothing to do, so it halts.

#![no_std]
#![no_main]

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
    // a triple fault — mask IRQs until we build an IDT (increment 5).
    unsafe { core::arch::asm!("cli", options(nomem, nostack)) }

    serial::init();

    serial_println!();
    serial_println!("PeekOS - a glass box");
    serial_println!("[boot] serial console up (COM1, 38400 8N1)");
    serial_println!("[boot] interrupts masked; no IDT yet");
    serial_println!("[boot] nothing left to do - halting");

    halt_loop()
}

/// Runs when anything in the kernel panics. There is nothing beneath us to
/// unwind into, so we report why and stop the CPU for good.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::emergency_print(format_args!("\n*** KERNEL PANIC ***\n{info}\n"));
    halt_loop()
}

/// Park the CPU. `hlt` idles it until the next interrupt rather than spinning
/// hot; the loop is because an interrupt will eventually wake it back up.
fn halt_loop() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}
