//! The PeekOS kernel.
//!
//! Increment 1: this compiles for the bare-metal `x86_64-unknown-none` target
//! and defines the entry point the bootloader will jump to. It does not print
//! anything yet — serial output and a boot banner arrive in the next increment.

#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;

// Wires `kernel_main` up as the entry point the `bootloader` crate calls,
// with the ABI it expects. Replaces the usual `_start` / `no_mangle` dance.
entry_point!(kernel_main);

/// First Rust code that runs in PeekOS. `boot_info` is everything the
/// bootloader discovered for us (memory map, framebuffer, …) — unused for now.
fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    halt_loop()
}

/// Runs when anything in the kernel panics. There is nothing beneath us to
/// unwind into, so we stop the CPU for good.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    halt_loop()
}

/// Park the CPU. `hlt` idles it until the next interrupt instead of spinning
/// hot; the loop is because interrupts will wake it back up.
fn halt_loop() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}
