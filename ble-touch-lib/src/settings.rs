use serde::{Deserialize, Serialize};

use crate::cmd::{GestureConfig, HidConfig, ScreenConfig, WindowConfig};

/// Full settings blob stored as a single key in ekv.
///
/// Uses `#[serde(default)]` so partial PATCH deserialization fills
/// missing fields with defaults, then we merge into the live settings
/// via `Settings::merge()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub screen: ScreenConfig,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub hid: HidConfig,
    #[serde(default)]
    pub gesture: GestureConfig,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            screen: ScreenConfig::default(),
            window: WindowConfig::default(),
            hid: HidConfig::default(),
            gesture: GestureConfig::default(),
        }
    }
}

impl Settings {
    /// Merge a PATCH object into current settings.
    ///
    /// Only fields present in `patch` overwrite the current values.
    /// Missing fields are kept from `current`.
    pub fn merge(current: &Settings, patch: &SettingsPatch) -> Self {
        Self {
            screen: patch.screen.clone().unwrap_or_else(|| current.screen),
            window: patch.window.clone().unwrap_or_else(|| current.window),
            hid: patch.hid.clone().unwrap_or_else(|| current.hid),
            gesture: patch.gesture.clone().unwrap_or_else(|| current.gesture),
        }
    }
}

/// PATCH-compatible settings struct (all fields optional).
/// Used for deserializing partial config updates from serial.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsPatch {
    #[serde(default)]
    pub screen: Option<ScreenConfig>,
    #[serde(default)]
    pub window: Option<WindowConfig>,
    #[serde(default)]
    pub hid: Option<HidConfig>,
    #[serde(default)]
    pub gesture: Option<GestureConfig>,
}

impl Default for SettingsPatch {
    fn default() -> Self {
        Self {
            screen: None,
            window: None,
            hid: None,
            gesture: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let s = Settings::default();
        assert_eq!(s.screen.width_px, 1080);
        assert_eq!(s.screen.height_px, 2340);
        assert_eq!(s.window.scale, 1.0);
        assert_eq!(s.window.offset_x, 0);
        assert_eq!(s.window.offset_y, 60);
        assert_eq!(s.hid.report_interval_ms, 20);
        assert_eq!(s.gesture.tap_delay_ms, 50);
    }

    #[test]
    fn merge_partial_patch() {
        let current = Settings {
            screen: ScreenConfig {
                width_px: 1080,
                height_px: 2340,
            },
            window: WindowConfig {
                scale: 3.0,
                offset_x: 0,
                offset_y: 60,
            },
            ..Default::default()
        };

        let patch = SettingsPatch {
            window: Some(WindowConfig {
                scale: 2.5,
                offset_x: 0,
                offset_y: 40,
            }),
            ..Default::default()
        };

        let merged = Settings::merge(&current, &patch);
        // Patched field changed
        assert_eq!(merged.window.scale, 2.5);
        assert_eq!(merged.window.offset_y, 40);
        // Unpatched fields preserved
        assert_eq!(merged.screen.width_px, 1080);
        assert_eq!(merged.hid.report_interval_ms, 20);
    }

    #[test]
    fn merge_empty_patch_preserves_all() {
        let current = Settings::default();
        let patch = SettingsPatch::default();
        let merged = Settings::merge(&current, &patch);
        assert_eq!(merged.screen.width_px, current.screen.width_px);
        assert_eq!(merged.window.scale, current.window.scale);
    }

    #[test]
    fn deserialize_partial_json() {
        let json = r#"{"screen":{"width_px":1440}}"#;
        let patch: SettingsPatch = serde_json::from_str(json).unwrap();
        assert_eq!(patch.screen.unwrap().width_px, 1440);
        assert_eq!(patch.screen.unwrap().height_px, 2340); // default
        assert!(patch.window.is_none());
    }

    #[test]
    fn round_trip_serialize_deserialize() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let s2: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s.screen.width_px, s2.screen.width_px);
        assert_eq!(s.window.scale, s2.window.scale);
    }
}
