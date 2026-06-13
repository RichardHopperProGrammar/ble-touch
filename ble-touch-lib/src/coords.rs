use crate::cmd::{ScreenConfig, WindowConfig};

/// Pipeline: CDP CSS pixel → physical screen pixel → HID logical coordinate.
///
/// 1. `cdp_px * scale + offset` → physical screen pixels (accounts for DPR + Chrome UI chrome)
/// 2. `physical_px / screen_px * 4095` → HID logical coordinate (0–4095)
pub fn cdp_to_hid(
    cdp_x: u16,
    cdp_y: u16,
    window: &WindowConfig,
    screen: &ScreenConfig,
) -> (u16, u16) {
    let phys_x = (cdp_x as f32 * window.scale + window.offset_x as f32).clamp(0.0, screen.width_px as f32);
    let phys_y = (cdp_y as f32 * window.scale + window.offset_y as f32).clamp(0.0, screen.height_px as f32);

    let hid_x = ((phys_x / screen.width_px as f32) * 4095.0).round() as u16;
    let hid_y = ((phys_y / screen.height_px as f32) * 4095.0).round() as u16;

    (hid_x.min(4095), hid_y.min(4095))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phone_screen() -> ScreenConfig {
        ScreenConfig {
            width_px: 1080,
            height_px: 2340,
        }
    }

    fn high_dpr_window() -> WindowConfig {
        WindowConfig {
            scale: 3.0,
            offset_x: 0,
            offset_y: 60,
        }
    }

    #[test]
    fn origin_maps_to_near_zero() {
        let (x, y) = cdp_to_hid(0, 0, &high_dpr_window(), &phone_screen());
        // With offset_y=60 and height=2340: (60/2340)*4095 ≈ 105
        assert!(x <= 2); // x should be ~0
        assert!(y < 200); // y should be ~105 from the offset
    }

    #[test]
    fn center_cdp_maps_reasonably() {
        // CDP viewport roughly 360x780 at 3x scale → physical 1080x2340
        let (x, y) = cdp_to_hid(180, 390, &high_dpr_window(), &phone_screen());
        // 180 * 3.0 = 540; 540/1080 * 4095 = 2047.5
        assert!(x >= 2045 && x <= 2050);
        // 390 * 3.0 + 60 = 1230; 1230/2340 * 4095 = 2152.5
        assert!(y >= 2150 && y <= 2155);
    }

    #[test]
    fn max_cdp_clamps_to_4095() {
        // Way out of bounds CDP coords
        let (x, y) = cdp_to_hid(9999, 9999, &high_dpr_window(), &phone_screen());
        assert_eq!(x, 4095);
        assert_eq!(y, 4095);
    }

    #[test]
    fn identity_scale_no_offset() {
        let screen = ScreenConfig {
            width_px: 800,
            height_px: 600,
        };
        let window = WindowConfig {
            scale: 1.0,
            offset_x: 0,
            offset_y: 0,
        };
        // At center of screen: 400px / 800px * 4095 = 2047.5
        let (x, y) = cdp_to_hid(400, 300, &window, &screen);
        assert_eq!(x, 2048); // rounds to nearest
        assert_eq!(y, 2048);
    }

    #[test]
    fn tiny_screen_dimensions_clamp() {
        let screen = ScreenConfig {
            width_px: 1,
            height_px: 1,
        };
        // With offset_y=60 and height_px=1: phys_y clamps to 1, hid_y = 4095
        // That's fine — the important thing is no division by zero panic.
        let window = WindowConfig::default();
        let (x, _y) = cdp_to_hid(0, 0, &window, &screen);
        assert_eq!(x, 0); // x with offset_x=0 should be 0
    }

    #[test]
    fn zero_offset_tiny_screen() {
        let screen = ScreenConfig {
            width_px: 1,
            height_px: 1,
        };
        let window = WindowConfig {
            scale: 1.0,
            offset_x: 0,
            offset_y: 0,
        };
        let (x, y) = cdp_to_hid(0, 0, &window, &screen);
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[test]
    fn overflow_cdp_clamps() {
        let screen = ScreenConfig::default();
        let window = WindowConfig::default();
        // cdp_x = 9999 >> phys_x clamped to screen.width_px >> hid_x = 4095
        let (x, y) = cdp_to_hid(9999, 9999, &window, &screen);
        assert_eq!(x, 4095);
        assert_eq!(y, 4095);
    }

    #[test]
    fn negative_offset_handled() {
        // Offset can be negative (e.g., if Chrome window starts above screen origin)
        let window = WindowConfig {
            scale: 1.0,
            offset_x: -50,
            offset_y: -30,
        };
        let screen = ScreenConfig {
            width_px: 800,
            height_px: 600,
        };
        // x = 100 * 1.0 + (-50) = 50; 50/800 * 4095 = 256
        let (x, _y) = cdp_to_hid(100, 100, &window, &screen);
        assert!(x >= 250 && x <= 260);
    }
}
