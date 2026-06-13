//! Settings persistence.
//!
//! On ESP32: loads/saves the [`Settings`] blob from NVS (Non-Volatile Storage).
//! On host: uses defaults / prints to stdout.

#[cfg(feature = "esp32")]
use ble_touch_lib::settings::Settings;
#[cfg(feature = "esp32")]
use log::warn;


#[cfg(feature = "esp32")]
/// Load settings from NVS.  Falls back to [`Settings::default`] if the key
/// doesn't exist or the stored JSON is malformed.
pub fn load_settings() -> Settings {
    // TODO: Implement NVS-based storage
    // For now, return defaults — actual NVS implementation requires
    // esp_idf_svc::nvs::NvsPartition setup
    warn!("NVS storage not yet implemented — using defaults");
    Settings::default()
}

#[cfg(feature = "esp32")]
/// Serialize settings to JSON and write to NVS.
pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_string(settings).map_err(|e| format!("serialize: {}", e))?;
    // TODO: Write to NVS partition
    log::info!("[storage] Would save: {}", json);
    Ok(())
}

// ---------------------------------------------------------------------------
// Host stubs — allow compilation on x86_64 without the ESP32 toolchain
// ---------------------------------------------------------------------------

#[cfg(not(feature = "esp32"))]
/// Stub for host builds — always returns defaults.
pub fn load_settings() -> ble_touch_lib::settings::Settings {
    ble_touch_lib::settings::Settings::default()
}

#[cfg(not(feature = "esp32"))]
/// Stub for host builds — no-op, just prints the JSON.
pub fn save_settings(settings: &ble_touch_lib::settings::Settings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    println!("[storage] Would save settings:\n{}", json);
    Ok(())
}
