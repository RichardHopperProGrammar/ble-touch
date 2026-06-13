//! BLE Touch HID Bridge — ESP32-C3 entry point.
//!
//! This crate handles all hardware-specific code:
//! - NimBLE initialization + HOGP service setup
//! - CDC-ACM serial I/O
//! - ekv flash storage
//! - Main event loop (read serial → process → send HID reports over BLE)

fn main() {
    // TODO: Initialize ESP-IDF / NimBLE
    // TODO: Set up CDC-ACM serial reader
    // TODO: Load settings from ekv
    // TODO: Create HOGP GATT service with touch screen descriptor
    // TODO: Main loop: read JSON lines → parse_cmd → process_cmd → BLE notify

    #[cfg(not(feature = "esp32"))]
    {
        println!("ble-touch-bin: compiled for host (no ESP32). Flash to hardware to run.");
    }
}
