pub mod cmd;
pub mod coords;
pub mod gesture;
pub mod hid;
pub mod settings;

/// Parse a JSON line from serial into a [`Cmd`].
pub fn parse_cmd(line: &str) -> Result<cmd::Cmd, serde_json::Error> {
    serde_json::from_str(line.trim())
}

/// Process a command and return the appropriate gesture sequence.
///
/// Returns `None` for commands that don't produce touch events
/// (Config, GetSettings, Pair, Unpair, ListBonds).
pub fn process_cmd(
    cmd: &cmd::Cmd,
    settings: &settings::Settings,
) -> Option<gesture::GestureSequence> {
    use cmd::Cmd;

    match cmd {
        Cmd::Tap { x, y } => Some(gesture::synthesize_tap(*x, *y, &settings.gesture)),
        Cmd::Down { x, y } => Some(gesture::synthesize_down(*x, *y)),
        Cmd::Move { x, y } => Some(gesture::synthesize_move(*x, *y)),
        Cmd::Up => Some(gesture::synthesize_up()),
        Cmd::Swipe { from, to, steps } => {
            Some(gesture::synthesize_swipe(*from, *to, *steps, &settings.gesture))
        }
        Cmd::Dtap { x, y } => Some(gesture::synthesize_dtap(*x, *y, &settings.gesture)),
        Cmd::LongPress { x, y, duration_ms } => {
            Some(gesture::synthesize_long_press(*x, *y, *duration_ms))
        }
        Cmd::Config { .. } => None,
        Cmd::GetSettings => None,
        Cmd::Pair => None,
        Cmd::Unpair => None,
        Cmd::ListBonds => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::Cmd;

    #[test]
    fn parse_and_process_tap() {
        let line = r#"{"cmd":"tap","x":512,"y":768}"#;
        let cmd = parse_cmd(line).unwrap();
        let settings = settings::Settings::default();
        let seq = process_cmd(&cmd, &settings);
        assert!(seq.is_some());
        let seq = seq.unwrap();
        assert_eq!(seq.steps.len(), 2);
    }

    #[test]
    fn parse_and_process_config_returns_none() {
        let line = r#"{"cmd":"config","screen":{"width_px":1080}}"#;
        let cmd = parse_cmd(line).unwrap();
        let settings = settings::Settings::default();
        assert!(process_cmd(&cmd, &settings).is_none());
    }

    #[test]
    fn full_pipeline_cdp_to_hid_reports() {
        // Full pipeline: parse JSON → apply coordinate transform → get HID reports
        let line = r#"{"cmd":"tap","x":360,"y":780}"#;
        let cmd = parse_cmd(line).unwrap();

        let settings = settings::Settings {
            screen: crate::cmd::ScreenConfig {
                width_px: 1080,
                height_px: 2340,
            },
            window: crate::cmd::WindowConfig {
                scale: 3.0,
                offset_x: 0,
                offset_y: 60,
            },
            ..Default::default()
        };

        // Manually transform coords first
        if let Cmd::Tap { x, y } = &cmd {
            let (hid_x, hid_y) = coords::cdp_to_hid(*x, *y, &settings.window, &settings.screen);
            let seq = gesture::synthesize_tap(hid_x, hid_y, &settings.gesture);
            let bytes = seq.steps[0].report.to_bytes();
            assert_eq!(bytes.len(), 8);
        } else {
            panic!("expected Tap");
        }
    }

    #[test]
    fn parse_invalid_json() {
        let result = parse_cmd("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_string() {
        let result = parse_cmd("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_whitespace_only() {
        let result = parse_cmd("   \n  ");
        assert!(result.is_err());
    }
}
