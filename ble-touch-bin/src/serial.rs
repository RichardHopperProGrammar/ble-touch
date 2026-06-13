//! CDC-ACM serial I/O over USB.
//!
//! Reads newline-delimited JSON commands from the host PC and sends JSON
//! responses back.  All hardware-specific code is behind `#[cfg(feature = "esp32")]`.

#[cfg(feature = "esp32")]
use esp_idf_svc::hal::usb::UsbDevice;
#[cfg(feature = "esp32")]
use log::{error, warn};

/// Maximum bytes we accept per serial line before dropping the remainder.
const MAX_LINE: usize = 256;

/// Internal line buffer.
#[cfg(feature = "esp32")]
struct SerialIo {
    buf: [u8; MAX_LINE],
    pos: usize,
}

#[cfg(feature = "esp32")]
impl SerialIo {
    fn new() -> Self {
        Self {
            buf: [0u8; MAX_LINE],
            pos: 0,
        }
    }

    /// Read one newline-delimited line from CDC-ACM.
    ///
    /// Returns `Ok(line)` (without the trailing `\n`) or `Err` on read failure.
    pub fn read_line(&mut self, _usb: &UsbDevice) -> Result<String, String> {
        // Blocking read loop — accumulate bytes until '\n' or buffer full.
        let mut total = 0usize;

        while total < MAX_LINE - 1 {
            match _usb.read(1) {
                Ok([byte]) if byte == b'\n' || byte == b'\r' => {
                    // Drop carriage return (Windows \r\n)
                    break;
                }
                Ok([byte]) => {
                    self.buf[total] = byte;
                    total += 1;
                }
                Ok(_) => continue, // shouldn't happen with read(1)
                Err(e) => {
                    if total > 0 {
                        warn!("Serial read error after {} bytes: {:?}", total, e);
                        break;
                    }
                    return Err(format!("Serial read error: {:?}", e));
                }
            }
        }

        if total == MAX_LINE - 1 {
            warn!("Line exceeded {} bytes — truncating", MAX_LINE);
        }

        match core::str::from_utf8(&self.buf[..total]) {
            Ok(s) => {
                self.pos = 0;
                Ok(s.trim().to_string())
            }
            Err(_) => {
                self.pos = 0;
                Err("Invalid UTF-8 on serial line".into())
            }
        }
    }

    /// Write a JSON response string back over CDC-ACM with trailing newline.
    pub fn write_response(&self, _usb: &UsbDevice, msg: &str) -> Result<(), String> {
        let payload = format!("{}\n", msg);
        match _usb.write_all(payload.as_bytes()) {
            Ok(()) => Ok(()),
            Err(e) => Err(format!("Serial write error: {:?}", e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Host stubs — allow compilation on x86_64 without the ESP32 toolchain
// ---------------------------------------------------------------------------

#[cfg(not(feature = "esp32"))]
/// Stub for host builds — reads from stdin for testing.
pub fn read_line() -> Result<String, String> {
    use std::io::Read;

    let mut buf = [0u8; MAX_LINE];
    match std::io::stdin().read(&mut buf) {
        Ok(0) => Err("EOF".into()),
        Ok(n) => {
            match core::str::from_utf8(&buf[..n]) {
                Ok(s) => Ok(s.trim().to_string()),
                Err(_) => Err("Invalid UTF-8".into()),
            }
        }
        Err(e) => Err(format!("stdin read error: {}", e)),
    }
}

#[cfg(not(feature = "esp32"))]
/// Stub for host builds — writes to stdout.
pub fn write_response(msg: &str) {
    println!("{}", msg);
}
