//! Serial I/O for reading JSON commands and sending responses.
//!
//! On ESP32: reads from UART0 (stdin/console).
//! On host: uses stdin/stdout for testing.

#[cfg(feature = "esp32")]
use log::warn;

/// Maximum bytes we accept per serial line before dropping the remainder.
const MAX_LINE: usize = 256;

#[cfg(feature = "esp32")]
/// Read one newline-delimited line from UART0 stdin.
pub fn read_line() -> Result<String, String> {
    use std::io::{self, Read};

    // Use ESP-IDF's console/stdio which is already set up by esp-idf-svc binstart
    let stdin = io::stdin();
    let mut lock = stdin.lock();

    let mut buffer = [0u8; MAX_LINE];
    let mut total: usize = 0;

    while total < MAX_LINE - 1 {
        match lock.read(&mut buffer[total..total + 1]) {
            Ok(1) => {
                if buffer[total] == b'\n' || buffer[total] == b'\r' {
                    break;
                }
                total += 1;
            }
            Ok(_) => break,
            Err(e) => {
                if total > 0 {
                    warn!("Serial read error after {} bytes: {:?}", total, e);
                    break;
                }
                return Err(format!("Serial read error: {:?}", e));
            }
        }
    }

    match core::str::from_utf8(&buffer[..total]) {
        Ok(s) => Ok(s.trim().to_string()),
        Err(_) => Err("Invalid UTF-8 on serial line".into()),
    }
}

#[cfg(feature = "esp32")]
/// Write a JSON response string back over serial with trailing newline.
pub fn write_response(msg: &str) {
    println!("{}", msg);
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
        Ok(n) => match core::str::from_utf8(&buf[..n]) {
            Ok(s) => Ok(s.trim().to_string()),
            Err(_) => Err("Invalid UTF-8".into()),
        },
        Err(e) => Err(format!("stdin read error: {}", e)),
    }
}

#[cfg(not(feature = "esp32"))]
/// Stub for host builds — writes to stdout.
pub fn write_response(msg: &str) {
    println!("{}", msg);
}
