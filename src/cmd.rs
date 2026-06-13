use serde::{Deserialize, Serialize};

/// Points in physical screen pixels (after scale + offset applied).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

/// Commands received over CDC-ACM serial.
///
/// Each line on the serial port is one JSON object matching this enum.
/// The `"cmd"` field discriminates the variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Cmd {
    /// Single tap at (x, y). Synthesizes DOWN → short delay → UP.
    Tap { x: u16, y: u16 },

    /// Finger down at (x, y). Start of a drag/gesture.
    Down { x: u16, y: u16 },

    /// Finger move to (x, y). Must follow a Down.
    Move { x: u16, y: u16 },

    /// Finger up. Ends current touch.
    Up,

    /// Swipe from one point to another, interpolated over `steps` moves.
    Swipe {
        from: Point,
        to: Point,
        #[serde(default = "default_swipe_steps")]
        steps: u8,
    },

    /// Double tap at (x, y). Two quick taps.
    Dtap { x: u16, y: u16 },

    /// Long press: DOWN → hold for `duration_ms` → UP.
    LongPress {
        x: u16,
        y: u16,
        #[serde(default = "default_long_press_ms")]
        duration_ms: u16,
    },

    /// Update settings (PATCH — partial update, missing fields kept as-is).
    Config {
        #[serde(default)]
        screen: Option<ScreenConfigPatch>,
        #[serde(default)]
        window: Option<WindowConfigPatch>,
        #[serde(default)]
        hid: Option<HidConfigPatch>,
        #[serde(default)]
        gesture: Option<GestureConfigPatch>,
    },

    /// Query current full settings.
    GetSettings,

    /// Start BLE pairing/bonding with next connecting device.
    Pair,

    /// Clear all stored BLE bonds.
    Unpair,

    /// List currently bonded BLE devices.
    ListBonds,
}

fn default_swipe_steps() -> u8 {
    10
}

fn default_long_press_ms() -> u16 {
    800
}

/// Screen physical dimensions in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenConfig {
    #[serde(default = "default_width_px")]
    pub width_px: u16,
    #[serde(default = "default_height_px")]
    pub height_px: u16,
}

impl Default for ScreenConfig {
    fn default() -> Self {
        Self {
            width_px: default_width_px(),
            height_px: default_height_px(),
        }
    }
}

fn default_width_px() -> u16 {
    1080
}

fn default_height_px() -> u16 {
    2340
}

/// Source viewport offset and scale factor (e.g., for DPR or overlay chrome).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowConfig {
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default)]
    pub offset_x: i16,
    #[serde(default = "default_offset_y")]
    pub offset_y: i16,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            scale: default_scale(),
            offset_x: 0,
            offset_y: default_offset_y(),
        }
    }
}

fn default_scale() -> f32 {
    1.0
}

fn default_offset_y() -> i16 {
    60
}

/// BLE HID configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HidConfig {
    #[serde(default = "default_report_interval_ms")]
    pub report_interval_ms: u8,
}

impl Default for HidConfig {
    fn default() -> Self {
        Self {
            report_interval_ms: default_report_interval_ms(),
        }
    }
}

fn default_report_interval_ms() -> u8 {
    20
}

/// Gesture timing configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GestureConfig {
    #[serde(default = "default_tap_delay_ms")]
    pub tap_delay_ms: u16,
    #[serde(default = "default_swipe_steps_cfg")]
    pub swipe_steps: u8,
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self {
            tap_delay_ms: default_tap_delay_ms(),
            swipe_steps: default_swipe_steps_cfg(),
        }
    }
}

fn default_tap_delay_ms() -> u16 {
    50
}

fn default_swipe_steps_cfg() -> u8 {
    10
}

/// Per-field PATCH sub-structs for Cmd::Config.
/// Each field is Option so you can send {"width_px":1440} without resetting height_px.

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ScreenConfigPatch {
    #[serde(default)]
    pub width_px: Option<u16>,
    #[serde(default)]
    pub height_px: Option<u16>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct WindowConfigPatch {
    #[serde(default)]
    pub scale: Option<f32>,
    #[serde(default)]
    pub offset_x: Option<i16>,
    #[serde(default)]
    pub offset_y: Option<i16>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct HidConfigPatch {
    #[serde(default)]
    pub report_interval_ms: Option<u8>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct GestureConfigPatch {
    #[serde(default)]
    pub tap_delay_ms: Option<u16>,
    #[serde(default)]
    pub swipe_steps: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_tap() {
        let json = r#"{"cmd":"tap","x":512,"y":768}"#;
        let cmd: Cmd = serde_json::from_str(json).unwrap();
        match cmd {
            Cmd::Tap { x, y } => assert_eq!((x, y), (512, 768)),
            _ => panic!("expected Tap"),
        }
    }

    #[test]
    fn round_trip_swipe_with_defaults() {
        let json = r#"{"cmd":"swipe","from":{"x":500,"y":1500},"to":{"x":500,"y":300}}"#;
        let cmd: Cmd = serde_json::from_str(json).unwrap();
        match cmd {
            Cmd::Swipe { steps, .. } => assert_eq!(steps, 10),
            _ => panic!("expected Swipe"),
        }
    }

    #[test]
    fn round_trip_swipe_with_steps() {
        let json = r#"{"cmd":"swipe","from":{"x":500,"y":1500},"to":{"x":500,"y":300},"steps":15}"#;
        let cmd: Cmd = serde_json::from_str(json).unwrap();
        match cmd {
            Cmd::Swipe { steps, .. } => assert_eq!(steps, 15),
            _ => panic!("expected Swipe"),
        }
    }

    #[test]
    fn round_trip_long_press_defaults() {
        let json = r#"{"cmd":"long_press","x":512,"y":768}"#;
        let cmd: Cmd = serde_json::from_str(json).unwrap();
        match cmd {
            Cmd::LongPress { duration_ms, .. } => assert_eq!(duration_ms, 800),
            _ => panic!("expected LongPress"),
        }
    }

    #[test]
    fn round_trip_long_press_custom() {
        let json = r#"{"cmd":"long_press","x":512,"y":768,"duration_ms":1200}"#;
        let cmd: Cmd = serde_json::from_str(json).unwrap();
        match cmd {
            Cmd::LongPress { duration_ms, .. } => assert_eq!(duration_ms, 1200),
            _ => panic!("expected LongPress"),
        }
    }

    #[test]
    fn round_trip_down_move_up() {
        let down: Cmd = serde_json::from_str(r#"{"cmd":"down","x":500,"y":1200}"#).unwrap();
        assert!(matches!(down, Cmd::Down { x: 500, y: 1200 }));

        let move_cmd: Cmd = serde_json::from_str(r#"{"cmd":"move","x":500,"y":900}"#).unwrap();
        assert!(matches!(move_cmd, Cmd::Move { x: 500, y: 900 }));

        let up: Cmd = serde_json::from_str(r#"{"cmd":"up"}"#).unwrap();
        assert!(matches!(up, Cmd::Up));
    }

    #[test]
    fn round_trip_dtap() {
        let json = r#"{"cmd":"dtap","x":512,"y":768}"#;
        let cmd: Cmd = serde_json::from_str(json).unwrap();
        match cmd {
            Cmd::Dtap { x, y } => assert_eq!((x, y), (512, 768)),
            _ => panic!("expected Dtap"),
        }
    }

    #[test]
    fn round_trip_config_partial() {
        let json = r#"{"cmd":"config","screen":{"width_px":1080}}"#;
        let cmd: Cmd = serde_json::from_str(json).unwrap();
        match cmd {
            Cmd::Config { screen, window, hid, gesture } => {
                let screen = screen.unwrap();
                assert_eq!(screen.width_px, Some(1080));
                assert_eq!(screen.height_px, None); // true partial!
                assert!(window.is_none());
                assert!(hid.is_none());
                assert!(gesture.is_none());
            }
            _ => panic!("expected Config"),
        }
    }

    #[test]
    fn round_trip_config_full() {
        let json = r#"{"cmd":"config","screen":{"width_px":1080,"height_px":2340},"window":{"scale":3.0,"offset_y":60},"hid":{"report_interval_ms":15},"gesture":{"tap_delay_ms":80}}"#;
        let cmd: Cmd = serde_json::from_str(json).unwrap();
        match cmd {
            Cmd::Config { screen, window, hid, gesture } => {
                assert_eq!(screen.unwrap().width_px, Some(1080));
                assert_eq!(window.unwrap().scale, Some(3.0));
                assert_eq!(hid.unwrap().report_interval_ms, Some(15));
                assert_eq!(gesture.unwrap().tap_delay_ms, Some(80));
            }
            _ => panic!("expected Config"),
        }
    }

    #[test]
    fn round_trip_get_settings() {
        let cmd: Cmd = serde_json::from_str(r#"{"cmd":"get_settings"}"#).unwrap();
        assert!(matches!(cmd, Cmd::GetSettings));
    }

    #[test]
    fn round_trip_pair() {
        let cmd: Cmd = serde_json::from_str(r#"{"cmd":"pair"}"#).unwrap();
        assert!(matches!(cmd, Cmd::Pair));
    }

    #[test]
    fn round_trip_unpair() {
        let cmd: Cmd = serde_json::from_str(r#"{"cmd":"unpair"}"#).unwrap();
        assert!(matches!(cmd, Cmd::Unpair));
    }

    #[test]
    fn round_trip_list_bonds() {
        let cmd: Cmd = serde_json::from_str(r#"{"cmd":"list_bonds"}"#).unwrap();
        assert!(matches!(cmd, Cmd::ListBonds));
    }

    #[test]
    fn serialize_tap_back_to_json() {
        let cmd = Cmd::Tap { x: 512, y: 768 };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"tap\""));
    }
}
