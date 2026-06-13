use crate::cmd::{GestureConfig, Point};
use crate::hid::HidReport;

/// A single step in a gesture sequence.
#[derive(Debug, Clone, Copy)]
pub struct GestureStep {
    /// The HID report to send.
    pub report: HidReport,
    /// Delay before next step (milliseconds). 0 = send immediately.
    pub delay_ms: u16,
}

/// Result of synthesizing a gesture into a sequence of HID reports.
#[derive(Debug, Clone)]
pub struct GestureSequence {
    pub steps: Vec<GestureStep>,
}

impl GestureSequence {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn push_touch(&mut self, x: u16, y: u16, delay_ms: u16) {
        self.steps.push(GestureStep {
            report: HidReport::touch(x, y),
            delay_ms,
        });
    }

    pub fn push_release(&mut self, delay_ms: u16) {
        self.steps.push(GestureStep {
            report: HidReport::release(),
            delay_ms,
        });
    }
}

/// Synthesize a TAP gesture: DOWN at (x,y) → delay → UP.
pub fn synthesize_tap(x: u16, y: u16, gesture: &GestureConfig) -> GestureSequence {
    let mut seq = GestureSequence::new();
    seq.push_touch(x, y, gesture.tap_delay_ms);
    seq.push_release(0);
    seq
}

/// Synthesize a DOWN event.
pub fn synthesize_down(x: u16, y: u16) -> GestureSequence {
    let mut seq = GestureSequence::new();
    seq.push_touch(x, y, 0);
    seq
}

/// Synthesize a MOVE event.
pub fn synthesize_move(x: u16, y: u16) -> GestureSequence {
    let mut seq = GestureSequence::new();
    seq.push_touch(x, y, 0);
    seq
}

/// Synthesize an UP (release) event.
pub fn synthesize_up() -> GestureSequence {
    let mut seq = GestureSequence::new();
    seq.push_release(0);
    seq
}

/// Synthesize a SWIPE: DOWN → interpolated MOVEs → UP.
/// Interpolates `num_steps` intermediate points between `from` and `to`.
pub fn synthesize_swipe(from: Point, to: Point, num_steps: u8, gesture: &GestureConfig) -> GestureSequence {
    let steps = num_steps.max(2) as usize; // At least from + to

    let mut seq = GestureSequence::new();

    // Start touch
    seq.push_touch(from.x, from.y, 0);

    // Interpolated intermediate points
    for i in 1..steps {
        let t = i as f32 / (steps - 1) as f32;
        let ix = lerp(from.x as f32, to.x as f32, t) as u16;
        let iy = lerp(from.y as f32, to.y as f32, t) as u16;
        let delay = (gesture.tap_delay_ms / num_steps.max(1) as u16).max(1);
        seq.push_touch(ix, iy, delay);
    }

    // Release at end
    seq.push_release(0);
    seq
}

/// Synthesize a DOUBLE TAP: two quick taps at the same position.
pub fn synthesize_dtap(x: u16, y: u16, gesture: &GestureConfig) -> GestureSequence {
    let mut seq = GestureSequence::new();
    // First tap
    seq.push_touch(x, y, gesture.tap_delay_ms);
    seq.push_release(5); // Small gap between taps
    // Second tap
    seq.push_touch(x, y, gesture.tap_delay_ms);
    seq.push_release(0);
    seq
}

/// Synthesize a LONG PRESS: DOWN → hold → UP.
pub fn synthesize_long_press(x: u16, y: u16, duration_ms: u16) -> GestureSequence {
    let mut seq = GestureSequence::new();
    seq.push_touch(x, y, duration_ms);
    seq.push_release(0);
    seq
}

/// Linear interpolation between a and b at parameter t (0.0 to 1.0).
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::GestureConfig;

    fn default_gesture() -> GestureConfig {
        GestureConfig::default()
    }

    #[test]
    fn tap_produces_down_then_up() {
        let seq = synthesize_tap(500, 1000, &default_gesture());
        assert_eq!(seq.steps.len(), 2);
        // First step: touch at (500, 1000)
        assert!(seq.steps[0].report.tip_switch);
        assert_eq!(seq.steps[0].report.x, 500);
        assert_eq!(seq.steps[0].report.y, 1000);
        // Second step: release
        assert!(!seq.steps[1].report.tip_switch);
        assert_eq!(seq.steps[1].report.contact_count, 0);
    }

    #[test]
    fn down_produces_single_touch() {
        let seq = synthesize_down(300, 600);
        assert_eq!(seq.steps.len(), 1);
        assert!(seq.steps[0].report.tip_switch);
    }

    #[test]
    fn up_produces_single_release() {
        let seq = synthesize_up();
        assert_eq!(seq.steps.len(), 1);
        assert!(!seq.steps[0].report.tip_switch);
    }

    #[test]
    fn swipe_interpolates_path() {
        let from = Point { x: 0, y: 0 };
        let to = Point { x: 400, y: 800 };
        let seq = synthesize_swipe(from, to, 5, &default_gesture());

        // Should have at least: down + intermediates + release
        assert!(seq.steps.len() >= 3);

        // First point is from
        assert_eq!(seq.steps[0].report.x, 0);
        assert_eq!(seq.steps[0].report.y, 0);

        // Last touch before release should be near `to`
        let last_touch = seq.steps.iter().filter(|s| s.report.tip_switch).last().unwrap();
        assert!(last_touch.report.x >= 395);
        assert!(last_touch.report.y >= 795);

        // Final step is release
        assert!(!seq.steps.last().unwrap().report.tip_switch);
    }

    #[test]
    fn dtap_has_four_steps() {
        let seq = synthesize_dtap(200, 400, &default_gesture());
        // touch → release → touch → release
        assert_eq!(seq.steps.len(), 4);
        assert!(seq.steps[0].report.tip_switch);
        assert!(!seq.steps[1].report.tip_switch);
        assert!(seq.steps[2].report.tip_switch);
        assert!(!seq.steps[3].report.tip_switch);
    }

    #[test]
    fn long_press_has_hold_delay() {
        let seq = synthesize_long_press(500, 1000, 800);
        assert_eq!(seq.steps.len(), 2);
        // Touch step should have the duration delay
        assert_eq!(seq.steps[0].delay_ms, 800);
        assert!(seq.steps[0].report.tip_switch);
    }

    #[test]
    fn swipe_min_steps_clamped() {
        let from = Point { x: 0, y: 0 };
        let to = Point { x: 100, y: 200 };
        let seq = synthesize_swipe(from, to, 0, &default_gesture());
        // Even with 0 steps, should produce at least from + release
        assert!(seq.steps.len() >= 2);
    }

    #[test]
    fn gesture_reports_clamped_to_4095() {
        let seq = synthesize_tap(5000, 5000, &default_gesture());
        assert_eq!(seq.steps[0].report.x, 4095);
        assert_eq!(seq.steps[0].report.y, 4095);
    }
}
