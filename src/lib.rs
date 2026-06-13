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
/// Incoming coordinates are source pixels from the host controller.
/// This function transforms them through [`coords::px_to_hid`] into
/// HID logical space (0–4095) before
/// dispatching to gesture synthesizers.
///
/// Returns `None` for commands that don't produce touch events
/// (Config, GetSettings, Pair, Unpair, ListBonds).
pub fn process_cmd(
    cmd: &cmd::Cmd,
    settings: &settings::Settings,
) -> Option<gesture::GestureSequence> {
    use cmd::Cmd;

    let transform = |x: u16, y: u16| -> (u16, u16) {
        coords::px_to_hid(x, y, &settings.window, &settings.screen)
    };

    match cmd {
        Cmd::Tap { x, y } => {
            let (hx, hy) = transform(*x, *y);
            Some(gesture::synthesize_tap(hx, hy, &settings.gesture))
        }
        Cmd::Down { x, y } => {
            let (hx, hy) = transform(*x, *y);
            Some(gesture::synthesize_down(hx, hy))
        }
        Cmd::Move { x, y } => {
            let (hx, hy) = transform(*x, *y);
            Some(gesture::synthesize_move(hx, hy))
        }
        Cmd::Up => Some(gesture::synthesize_up()),
        Cmd::Swipe { from, to, steps } => {
            let (hx1, hy1) = transform(from.x, from.y);
            let (hx2, hy2) = transform(to.x, to.y);
            let hfrom = cmd::Point { x: hx1, y: hy1 };
            let hto = cmd::Point { x: hx2, y: hy2 };
            Some(gesture::synthesize_swipe(hfrom, hto, *steps, &settings.gesture))
        }
        Cmd::Dtap { x, y } => {
            let (hx, hy) = transform(*x, *y);
            Some(gesture::synthesize_dtap(hx, hy, &settings.gesture))
        }
        Cmd::LongPress { x, y, duration_ms } => {
            let (hx, hy) = transform(*x, *y);
            Some(gesture::synthesize_long_press(hx, hy, *duration_ms))
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
    fn full_pipeline_px_to_hid_reports() {
        // Full pipeline: parse JSON → process_cmd transforms coords → HID reports
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

        // process_cmd now internally calls px_to_hid
        let seq = process_cmd(&cmd, &settings).unwrap();
        assert_eq!(seq.steps.len(), 2);
        let bytes = seq.steps[0].report.to_bytes();
        assert_eq!(bytes.len(), 8);
        // Verify coords were transformed (source 360,780 with 3x scale + 60 offset
        // should produce HID coords significantly higher than raw values)
        assert!(seq.steps[0].report.x > 0);
        assert!(seq.steps[0].report.y > 0);
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
