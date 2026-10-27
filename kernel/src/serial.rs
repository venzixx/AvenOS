use core::fmt;
use spin::Mutex;
use uart_16550::SerialPort;
use x86_64::instructions::port::Port;

pub static SERIAL1: Mutex<Option<SerialPort>> = Mutex::new(None);

/// Probes whether a 16550-compatible UART is present at `base_port`
/// using the scratch register (offset +7).
pub fn probe_uart(base_port: u16) -> bool {
    unsafe {
        let mut scratch_port = Port::<u8>::new(base_port + 7);
        // Write test byte 1
        scratch_port.write(0xAE);
        if scratch_port.read() != 0xAE {
            return false;
        }
        // Write test byte 2
        scratch_port.write(0x55);
        scratch_port.read() == 0x55
    }
}

pub fn init() {
    const COM1_BASE: u16 = 0x3F8;

    // Verify hardware presence before blind polling
    if !probe_uart(COM1_BASE) {
        // Hardware not found; leave SERIAL1 as None
        return;
    }

    let mut serial_port = unsafe { SerialPort::new(COM1_BASE) };
    serial_port.init();
    *SERIAL1.lock() = Some(serial_port);
}

/// Force-unlocks the serial mutex during panic handling to prevent deadlocks.
///
/// # Safety
/// Only call from panic handler or emergency halt routines.
pub unsafe fn force_unlock() {
    SERIAL1.force_unlock();
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        if let Some(ref mut serial) = *SERIAL1.lock() {
            let _ = serial.write_fmt(args);
        }
    });
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}
