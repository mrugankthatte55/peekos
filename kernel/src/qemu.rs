//! Talking to the QEMU virtual machine from inside the guest.
//!
//! `cargo xtask test` starts QEMU with an `isa-debug-exit` device. Writing a
//! 32-bit value `v` to port `0xF4` then makes the QEMU *process* exit with
//! status `(v << 1) | 1`, which is how a headless boot reports its result.
//!
//! This module only exists under the `boot-test` feature.

/// Clean boot. Makes the QEMU process exit with code 33.
pub const EXIT_SUCCESS: u32 = 0x10;

/// Something went wrong (e.g. a panic). QEMU process exits with code 35.
pub const EXIT_FAILED: u32 = 0x11;

/// Ask QEMU to terminate with the code derived from `request`. If the
/// `isa-debug-exit` device is not present the port write does nothing, so we
/// park the CPU afterwards rather than falling through.
pub fn exit(request: u32) -> ! {
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") 0xF4u16,
            in("eax") request,
            options(nomem, nostack, preserves_flags),
        );
    }
    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}
