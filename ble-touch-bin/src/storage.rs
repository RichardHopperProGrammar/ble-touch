//! ekv-based settings persistence.
//!
//! Stores and loads the full [`Settings`] blob under a single key ("settings")
//! in an LSM-tree keyed-value store backed by ESP32 flash.
//!
//! All hardware code behind `#[cfg(feature = "esp32")]`.

#[cfg(feature = "esp32")]
use ble_touch_lib::settings::Settings;
#[cfg(feature = "esp32")]
use log::{error, warn};

/// The ekv key name for the settings blob.
const SETTINGS_KEY: &[u8] = b"settings";

#[cfg(feature = "esp32")]
/// Load settings from ekv.  Falls back to [`Settings::default`] if the key
/// doesn't exist or the stored JSON is malformed.
pub fn load_settings(_flash: &impl embedded_storage::nor_flash::NorFlash) -> Settings {
    let storage = crate::flash::EspFlashBacking::new(_flash);
    let mut kv = match ekv::Ekv::new(storage) {
        Ok(kv) => kv,
        Err(e) => {
            warn!("ekv open failed: {:?} — using defaults", e);
            return Settings::default();
        }
    };

    match kv.get(SETTINGS_KEY) {
        Ok(Some(data)) => {
            // Deserialize stored JSON
            let json = match core::str::from_utf8(data.as_ref()) {
                Ok(s) => s,
                Err(_) => {
                    warn!("Stored settings not valid UTF-8 — using defaults");
                    return Settings::default();
                }
            };
            match serde_json::from_str(json) {
                Ok(settings) => settings,
                Err(e) => {
                    error!("Failed to deserialize settings: {:?} — using defaults", e);
                    Settings::default()
                }
            }
        }
        Ok(None) => {
            // Key doesn't exist yet — first boot
            warn!("No stored settings found — using defaults");
            Settings::default()
        }
        Err(e) => {
            error!("ekv read error: {:?} — using defaults", e);
            Settings::default()
        }
    }
}

#[cfg(feature = "esp32")]
/// Serialize settings to JSON and write back to ekv.
pub fn save_settings(
    _flash: &impl embedded_storage::nor_flash::NorFlash,
    settings: &Settings,
) -> Result<(), String> {
    let storage = crate::flash::EspFlashBacking::new(_flash);
    let mut kv = ekv::Ekv::new(storage).map_err(|e| format!("ekv open: {:?}", e))?;

    let json = serde_json::to_string(settings).map_err(|e| format!("serialize: {}", e))?;

    // ekv writes need the old length for garbage collection.
    let old_len = kv.get(SETTINGS_KEY).map(|r| {
        r.ok().and_then(|d| Some(d.as_ref().len())).unwrap_or(0)
    });

    kv.put(SETTINGS_KEY, json.as_bytes(), old_len)
        .map_err(|e| format!("ekv write: {:?}", e))?;

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
