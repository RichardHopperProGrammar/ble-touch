use serde::{Deserialize, Serialize};

use crate::cmd::{
    GestureConfig, GestureConfigPatch, HidConfig, HidConfigPatch, ScreenConfig,
    ScreenConfigPatch, WindowConfig, WindowConfigPatch,
};

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
    /// Missing fields (None) are kept from `current`.
    pub fn merge(current: &Settings, patch: &SettingsPatch) -> Self {
        let screen = match &patch.screen {
            Some(p) => ScreenConfig {
                width_px: p.width_px.unwrap_or(current.screen.width_px),
                height_px: p.height_px.unwrap_or(current.screen.height_px),
            },
            None => current.screen,
        };
        let window = match &patch.window {
            Some(p) => WindowConfig {
                scale: p.scale.unwrap_or(current.window.scale),
                offset_x: p.offset_x.unwrap_or(current.window.offset_x),
                offset_y: p.offset_y.unwrap_or(current.window.offset_y),
            },
            None => current.window,
        };
        let hid = match &patch.hid {
            Some(p) => HidConfig {
                report_interval_ms: p.report_interval_ms.unwrap_or(current.hid.report_interval_ms),
            },
            None => current.hid,
        };
        let gesture = match &patch.gesture {
            Some(p) => GestureConfig {
                tap_delay_ms: p.tap_delay_ms.unwrap_or(current.gesture.tap_delay_ms),
                swipe_steps: p.swipe_steps.unwrap_or(current.gesture.swipe_steps),
            },
            None => current.gesture,
        };
        Self { screen, window, hid, gesture }
    }
}

/// PATCH-compatible settings struct (all fields optional).
/// Used for deserializing partial config updates from serial.
/// Each sub-struct uses per-field Option so you can send
/// {"screen":{"width_px":1440}} without resetting height_px.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsPatch {
    #[serde(default)]
    pub screen: Option<ScreenConfigPatch>,
    #[serde(default)]
    pub window: Option<WindowConfigPatch>,
    #[serde(default)]
    pub hid: Option<HidConfigPatch>,
    #[serde(default)]
    pub gesture: Option<GestureConfigPatch>,
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

    use crate::cmd::WindowConfigPatch;

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
            window: Some(WindowConfigPatch {
                scale: Some(2.5),
                offset_x: None,
                offset_y: Some(40),
            }),
            ..Default::default()
        };

        let merged = Settings::merge(&current, &patch);
        // Patched fields changed
        assert_eq!(merged.window.scale, 2.5);
        assert_eq!(merged.window.offset_y, 40);
        // Unpatched fields within window preserved
        assert_eq!(merged.window.offset_x, 0); // was None in patch, kept from current
        // Unpatched sub-structs preserved
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
        let screen_patch = patch.screen.unwrap();
        assert_eq!(screen_patch.width_px, Some(1440));
        assert_eq!(screen_patch.height_px, None); // NOT defaulted — true partial!
        assert!(patch.window.is_none());
    }

    #[test]
    fn merge_preserves_unsent_fields() {
        let current = Settings::default();
        let json = r#"{"screen":{"width_px":1440}}"#;
        let patch: SettingsPatch = serde_json::from_str(json).unwrap();
        let merged = Settings::merge(&current, &patch);
        // Only width_px changed
        assert_eq!(merged.screen.width_px, 1440);
        // height_px preserved from current (not reset to default)
        assert_eq!(merged.screen.height_px, 2340);
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
