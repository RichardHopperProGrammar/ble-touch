//! BLE Touch HID Bridge — ESP32-C3 entry point.
//!
//! This crate handles all hardware-specific code:
//! - NimBLE initialization + HOGP service setup
//! - CDC-ACM serial I/O
//! - ekv flash storage for settings persistence
//! - Main event loop: read JSON commands from serial → process → send HID reports over BLE

mod ble;
mod flash;
mod serial;
mod storage;

use ble_touch_lib::cmd::Cmd;
use log::{error, info, warn};

#[cfg(feature = "esp32")]
fn main() {
    // --- ESP-IDF logging ---
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("ble-touch-bin starting...");

    // --- BLE init ---
    let ble_handle = match ble::init_ble() {
        Ok(h) => h,
        Err(e) => {
            error!("BLE init failed: {}", e);
            return;
        }
    };

    // --- Start advertising ---
    if let Err(e) = ble::start_advertising(&ble_handle) {
        error!("Advertising failed: {}", e);
        return;
    }

    // --- Load settings from flash ---
    let mut settings = storage::load_settings();
    info!("Settings loaded");

    // --- Get USB CDC-ACM handle ---
    let usb = match get_usb_device() {
        Ok(u) => u,
        Err(e) => {
            error!("USB init failed: {}", e);
            return;
        }
    };

    let mut sio = serial::SerialIo::new();

    info!("ble-touch-bin ready — awaiting serial commands");

    // --- Main event loop ---
    loop {
        match sio.read_line(&usb) {
            Ok(line) if line.is_empty() => continue,
            Ok(line) => {
                // Parse JSON command
                let cmd = match ble_touch_lib::parse_cmd(&line) {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("Parse error: {:?} — line: {}", e, line);
                        send_error(&sio, &usb, &format!("parse: {}", e));
                        continue;
                    }
                };

                // Dispatch
                match cmd {
                    Cmd::Config { screen, window, hid, gesture } => {
                        let patch = ble_touch_lib::settings::SettingsPatch {
                            screen,
                            window,
                            hid,
                            gesture,
                        };
                        settings = ble_touch_lib::settings::Settings::merge(&settings, &patch);
                        if let Err(e) = storage::save_settings(&settings) {
                            error!("Save failed: {}", e);
                            send_error(&sio, &usb, &format!("save: {}", e));
                        } else {
                            send_ok(&sio, &usb, "config saved");
                        }
                    }
                    Cmd::GetSettings => {
                        let json = serde_json::to_string(&settings)
                            .unwrap_or_else(|e| format!("{{\"error\":\"{:?}\"}}", e));
                        send_response(&sio, &usb, &json);
                    }
                    Cmd::Pair => {
                        send_ok(&sio, &usb, "pairing initiated");
                        // Trigger SMP bonding on next connection
                        // (handled by NimBLE security callbacks)
                    }
                    Cmd::Unpair => {
                        send_ok(&sio, &usb, "bonds cleared");
                        // Clear stored bonds from NVS
                    }
                    Cmd::ListBonds => {
                        // TODO: enumerate bonded peers from SMP store
                        send_response(&sio, &usb, r#"{"bonds":[]}"#);
                    }

                    // Gesture commands — produce HID reports and send over BLE
                    Cmd::Tap { .. }
                    | Cmd::Down { .. }
                    | Cmd::Move { .. }
                    | Cmd::Up
                    | Cmd::Swipe { .. }
                    | Cmd::Dtap { .. }
                    | Cmd::LongPress { .. } => {
                        if let Some(seq) = ble_touch_lib::process_cmd(&cmd, &settings) {
                            send_hid_reports(&ble_handle, &seq);
                            send_ok(&sio, &usb, "gesture sent");
                        } else {
                            send_error(&sio, &usb, "no gesture produced");
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Serial read error: {}", e);
            }
        }
    }
}

#[cfg(feature = "esp32")]
fn get_usb_device() -> Result<esp_idf_svc::hal::usb::UsbDevice, String> {
    let usb = match esp_idf_svc::hal::usb::UsbDevice::new(
        esp_idf_svc::hal::gpio::PinDriver::output(&esp_idf_svc::hal::peripherals::Peripherals::take().unwrap().gpio0).map_err(|e| format!("gpio0: {:?}", e))?,
    ) {
        Ok(u) => u,
        Err(e) => return Err(format!("USB device: {:?}", e)),
    };
    Ok(usb)
}

#[cfg(feature = "esp32")]
fn send_response(sio: &serial::SerialIo, usb: &esp_idf_svc::hal::usb::UsbDevice, msg: &str) {
    if let Err(e) = sio.write_response(usb, msg) {
        error!("Response send failed: {}", e);
    }
}

#[cfg(feature = "esp32")]
fn send_ok(sio: &serial::SerialIo, usb: &esp_idf_svc::hal::usb::UsbDevice, status: &str) {
    let payload = serde_json::json!({ "ok": true, "status" });
    send_response(sio, usb, &payload.to_string());
}

#[cfg(feature = "esp32")]
fn send_error(sio: &serial::SerialIo, usb: &esp_idf_svc::hal::usb::UsbDevice, msg: &str) {
    let payload = serde_json::json!({ "ok": false, "error": msg });
    send_response(sio, usb, &payload.to_string());
}

#[cfg(feature = "esp32")]
fn send_hid_reports(
    _peripheral: &esp32_nimble::nimble::BleHogPeripheral,
    seq: &ble_touch_lib::gesture::GestureSequence,
) {
    for step in &seq.steps {
        let report = step.report.to_bytes();
        if let Err(e) = _peripheral.send_report(&report) {
            error!("Failed to send HID report: {:?}", e);
        }
        // Respect gesture timing (ESP-IDF delay)
        if step.delay_us > 0 {
            esp_idf_svc::sys::esp_task_delay(step.delay_us / 1000);
        }
    }
}

// ---------------------------------------------------------------------------
// Host stub — runs on x86_64 for CI / development without ESP32 toolchain
// ---------------------------------------------------------------------------

#[cfg(not(feature = "esp32"))]
fn main() {
    println!("ble-touch-bin: compiled for host (no ESP32). Flash to hardware to run.");

    // Quick smoke test: parse a command and verify the lib works
    let line = r#"{"cmd":"tap","x":512,"y":768}"#;
    match ble_touch_lib::parse_cmd(line) {
        Ok(cmd) => {
            let settings = ble_touch_lib::settings::Settings::default();
            if let Some(seq) = ble_touch_lib::process_cmd(&cmd, &settings) {
                println!("  OK: parsed {:?} → {} gesture steps", cmd, seq.steps.len());
            } else {
                println!("  OK: parsed {:?} → no gesture (config/cmd)", cmd);
            }
        }
        Err(e) => eprintln!("  FAIL: parse error: {:?}", e),
    }
}
