//! Minimal driver for the 16550 UART on COM1.
//!
//! The UART exposes eight one-byte registers at consecutive x86 I/O ports
//! starting at `0x3F8`. We reach them with the `in`/`out` instructions. QEMU,
//! started with `-serial stdio`, forwards every byte written here to the
//! terminal that launched it — so this is the kernel's console for now, and
//! the input side of the `peek>` shell later.

use core::fmt::{self, Write};
use spin::Mutex;

/// COM1. (The other legacy ports are 0x2F8, 0x3E8, 0x2E8.)
const COM1: u16 = 0x3F8;

// 16550 register offsets from `base`. `base + 0` is the data register — and,
// while DLAB (bit 7 of the line-control register) is set, the low byte of the
// baud-rate divisor.
const REG_INT_ENABLE: u16 = 1; // doubles as the divisor high byte while DLAB=1
const REG_FIFO_CTRL: u16 = 2;
const REG_LINE_CTRL: u16 = 3;
const REG_MODEM_CTRL: u16 = 4;
const REG_LINE_STATUS: u16 = 5;

/// The one serial port, behind a lock so the print macros work from anywhere.
/// Brought up once by [`init`].
static SERIAL: Mutex<Uart> = Mutex::new(Uart::new(COM1));

pub struct Uart {
    /// Port address of register 0; the rest are `base + 1 ..= base + 7`.
    base: u16,
}

impl Uart {
    const fn new(base: u16) -> Self {
        Uart { base }
    }

    /// Standard 16550 bring-up: 38400 baud, 8 data bits, no parity, 1 stop bit.
    fn init(&mut self) {
        let b = self.base;
        unsafe {
            outb(b + REG_INT_ENABLE, 0x00); // no UART interrupts for now
            outb(b + REG_LINE_CTRL, 0x80);  // DLAB = 1: b+0 and b+1 are the divisor
            outb(b, 0x03);                  // divisor low  = 3  ->  115200 / 3 = 38400 baud
            outb(b + REG_INT_ENABLE, 0x00); // divisor high = 0
            outb(b + REG_LINE_CTRL, 0x03);  // DLAB = 0, 8 data bits, no parity, 1 stop bit
            outb(b + REG_FIFO_CTRL, 0xC7);  // enable + clear FIFOs, 14-byte trigger
            outb(b + REG_MODEM_CTRL, 0x0B); // DTR + RTS + OUT2 (OUT2 gates the IRQ line)
        }
    }

    fn write_byte(&mut self, byte: u8) {
        unsafe {
            // Bit 5 of the line-status register = "transmit holding register
            // empty". Spin until the UART can accept another byte.
            while inb(self.base + REG_LINE_STATUS) & 0x20 == 0 {}
            outb(self.base, byte);
        }
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            // Terminals want CRLF; the kernel only ever emits '\n'.
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
        Ok(())
    }
}

/// Bring the serial port up. Call once, early, before any `serial_println!`.
pub fn init() {
    SERIAL.lock().init();
}

/// Backing function for [`serial_print!`] / [`serial_println!`].
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    SERIAL
        .lock()
        .write_fmt(args)
        .expect("serial writes are infallible");
}

/// Write straight to the UART ports without taking the lock. For the panic
/// handler only: a panic may have happened *while holding* the lock, and
/// locking again would deadlock. A second handle to the same ports is fine.
pub fn emergency_print(args: fmt::Arguments) {
    let _ = Uart::new(COM1).write_fmt(args);
}

#[inline]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags),
    );
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        out("al") value,
        in("dx") port,
        options(nomem, nostack, preserves_flags),
    );
    value
}

/// Print over the serial console, no trailing newline.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => { $crate::serial::_print(format_args!($($arg)*)) };
}

/// Print over the serial console, with a trailing newline.
#[macro_export]
macro_rules! serial_println {
    () => { $crate::serial_print!("\n") };
    ($($arg:tt)*) => { $crate::serial_print!("{}\n", format_args!($($arg)*)) };
}
