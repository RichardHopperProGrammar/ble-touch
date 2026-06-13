//! Settings persistence.
//!
//! On ESP32: loads/saves the [`Settings`] blob from NVS (Non-Volatile Storage).
//! On host: uses defaults / prints to stdout.

use ble_touch::settings::Settings;
#[cfg(feature = "esp32")]
use log::{info, warn};

/// NVS namespace for settings storage.
const SETTINGS_NAMESPACE: &str = "ble_touch";

/// NVS key name for the settings JSON blob.
const SETTINGS_KEY: &str = "cfg";

#[cfg(feature = "esp32")]
/// Load settings from NVS.  Falls back to [`Settings::default`] if the key
/// doesn't exist or the stored JSON is malformed.
pub fn load_settings() -> Settings {
    let partition = match esp_idf_svc::nvs::EspDefaultNvsPartition::take() {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to open NVS default partition: {:?} — using defaults", e);
            return Settings::default();
        }
    };

    let nvs = match esp_idf_svc::nvs::EspNvs::new(partition, SETTINGS_NAMESPACE, false) {
        Ok(h) => h,
        Err(e) => {
            warn!("Failed to open NVS namespace: {:?} — using defaults", e);
            return Settings::default();
        }
    };

    // Get string length first
    let len = match nvs.str_len(SETTINGS_KEY) {
        Ok(Some(len)) => len,
        Ok(None) => {
            info!("No settings found in NVS — using defaults");
            return Settings::default();
        }
        Err(e) => {
            warn!("Failed to get NVS string length: {:?} — using defaults", e);
            return Settings::default();
        }
    };

    // Read the string (length includes null terminator)
    let mut buf = vec![0u8; len];
    match nvs.get_str(SETTINGS_KEY, &mut buf) {
        Ok(Some(json_str)) => match serde_json::from_str::<Settings>(json_str) {
            Ok(settings) => {
                info!("Loaded settings from NVS");
                return settings;
            }
            Err(e) => {
                warn!("Malformed JSON in NVS: {} — using defaults", e);
            }
        },
        Ok(None) => {
            warn!("NVS key exists but returned None — using defaults");
        }
        Err(e) => {
            warn!("Failed to read NVS string: {:?} — using defaults", e);
        }
    }

    Settings::default()
}

#[cfg(feature = "esp32")]
/// Serialize settings to JSON and write to NVS.
pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_string(settings).map_err(|e| format!("serialize: {}", e))?;

    let partition = esp_idf_svc::nvs::EspDefaultNvsPartition::take()
        .map_err(|e| format!("open NVS partition: {:?}", e))?;

    let nvs = esp_idf_svc::nvs::EspNvs::new(partition, SETTINGS_NAMESPACE, true)
        .map_err(|e| format!("open NVS namespace: {:?}", e))?;

    nvs.set_str(SETTINGS_KEY, &json).map_err(|e| format!("write NVS: {:?}", e))?;

    info!("Settings saved to NVS ({} bytes)", json.len());
    Ok(())
}

// ---------------------------------------------------------------------------
// Host stubs — allow compilation on x86_64 without the ESP32 toolchain
// ---------------------------------------------------------------------------
#[cfg(not(feature = "esp32"))]
#[allow(dead_code)]

#[cfg(not(feature = "esp32"))]
/// Stub for host builds — always returns defaults.
pub fn load_settings() -> Settings {
    Settings::default()
}

#[cfg(not(feature = "esp32"))]
/// Stub for host builds — no-op, just prints the JSON.
pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    println!("[storage] Would save settings:\n{}", json);
    Ok(())
}
