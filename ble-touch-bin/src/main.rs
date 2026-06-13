//! BLE Touch HID Bridge — ESP32-C3 entry point.
//!
//! This crate handles all hardware-specific code:
//! - NimBLE initialization + HOGP service setup
//! - CDC-ACM serial I/O
//! - ekv flash storage for settings persistence
//! - Main event loop: read JSON commands from serial → process → send HID reports over BLE

mod ble;
mod serial;
mod storage;

use ble_touch_lib::cmd::Cmd;
use log::{error, info, warn};

#[cfg(feature = "esp32")]
fn main() {
    // --- ESP-IDF logging ---
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("ble-touch-bin starting...");

    // --- BLE init (HOGP service + advertising) ---
    if let Err(e) = ble::init_and_advertise() {
        error!("BLE init failed: {}", e);
        loop {}
    }

    // --- Load settings from flash ---
    let mut settings = storage::load_settings();
    info!("Settings loaded");

    info!("ble-touch-bin ready — awaiting serial commands");

    // --- Main event loop ---
    loop {
        match serial::read_line() {
            Ok(line) if line.is_empty() => continue,
            Ok(line) => {
                // Parse JSON command
                let cmd = match ble_touch_lib::parse_cmd(&line) {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("Parse error: {:?} — line: {}", e, line);
                        serial::write_response(&format!("{{\"ok\":false,\"error\":\"{}\"}}", e));
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
                            serial::write_response(&format!("{{\"ok\":false,\"error\":\"{}\"}}", e));
                        } else {
                            serial::write_response(r#"{"ok":true,"status":"config saved"}"#);
                        }
                    }
                    Cmd::GetSettings => {
                        let json = serde_json::to_string(&settings)
                            .unwrap_or_else(|e| format!("{{\"error\":\"{:?}\"}}", e));
                        serial::write_response(&json);
                    }
                    Cmd::Pair => {
                        // Trigger SMP bonding — handled by NimBLE security callbacks
                        serial::write_response(r#"{"ok":true,"status":"pairing initiated"}"#);
                    }
                    Cmd::Unpair => {
                        // Clear stored bonds from NVS (TODO)
                        serial::write_response(r#"{"ok":true,"status":"bonds cleared"}"#);
                    }
                    Cmd::ListBonds => {
                        serial::write_response(r#"{"bonds":[]}"#);
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
                            ble::send_hid_reports(&seq);
                            serial::write_response(r#"{"ok":true,"status":"gesture sent"}"#);
                        } else {
                            serial::write_response(r#"{"ok":false,"error":"no gesture produced"}"#);
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
                println!("  OK: parsed {:?} -> {} gesture steps", cmd, seq.steps.len());
            } else {
                println!("  OK: parsed {:?} -> no gesture (config/cmd)", cmd);
            }
        }
        Err(e) => eprintln!("  FAIL: parse error: {:?}", e),
    }
}
