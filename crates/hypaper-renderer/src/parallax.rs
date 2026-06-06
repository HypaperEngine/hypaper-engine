//! Parallax effect state with workspace-change-driven slide animation.

/// Seconds after a workspace change before the parallax target resets to the origin.
const SLIDE_RESET_SECS: f32 = 0.3;

/// Parallax offset state for workspace-synchronised slide animations.
///
/// On a workspace change, [`on_workspace_change`](ParallaxState::on_workspace_change)
/// kicks the target offset in the appropriate direction. Each frame,
/// [`update`](ParallaxState::update) smoothly interpolates the live offset toward
/// the target and resets the target back to centre after [`SLIDE_RESET_SECS`] seconds.
pub struct ParallaxState {
    /// Current parallax offset in normalised screen coordinates (X axis).
    pub offset_x: f32,
    /// Current parallax offset in normalised screen coordinates (Y axis).
    pub offset_y: f32,
    /// Target offset the live offset is interpolating toward (X axis).
    pub target_x: f32,
    /// Target offset the live offset is interpolating toward (Y axis).
    pub target_y: f32,
    /// Maximum displacement amplitude as a fraction of the viewport (e.g. `0.05` = 5 %).
    pub intensity: f32,
    /// Lerp speed factor applied per frame: `offset += (target − offset) × speed × delta`.
    pub speed: f32,
    /// Countdown in seconds until the slide target is reset to the origin.
    slide_reset_timer: f32,
}

impl ParallaxState {
    /// Creates a new `ParallaxState` with the given `intensity` and a default
    /// lerp speed of `5.0`.  All offsets start at zero.
    pub fn new(intensity: f32) -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            target_x: 0.0,
            target_y: 0.0,
            intensity,
            speed: 5.0,
            slide_reset_timer: 0.0,
        }
    }

    /// Sets an explicit `(x, y)` target offset, overriding any in-progress slide.
    pub fn set_target(&mut self, x: f32, y: f32) {
        self.target_x = x;
        self.target_y = y;
    }

    /// Advances the parallax simulation by `delta` seconds.
    ///
    /// Decrements the slide-reset timer; when it expires the target returns to
    /// the origin so the offset lerps back to centre.  Then applies the lerp:
    /// `offset += (target − offset) × speed × delta`.
    pub fn update(&mut self, delta: f32) {
        if self.slide_reset_timer > 0.0 {
            self.slide_reset_timer -= delta;
            if self.slide_reset_timer <= 0.0 {
                self.slide_reset_timer = 0.0;
                self.target_x = 0.0;
                self.target_y = 0.0;
            }
        }
        self.offset_x += (self.target_x - self.offset_x) * self.speed * delta;
        self.offset_y += (self.target_y - self.offset_y) * self.speed * delta;
    }

    /// Triggers a directional slide kick when the active workspace changes.
    ///
    /// * `to > from` → slides left  (`target_x = −intensity`)
    /// * `to < from` → slides right (`target_x = +intensity`)
    /// * equal → no-op
    ///
    /// The target resets to zero after [`SLIDE_RESET_SECS`] via
    /// [`update`](Self::update).
    pub fn on_workspace_change(&mut self, from: u32, to: u32) {
        if to > from {
            self.target_x = -self.intensity;
        } else if to < from {
            self.target_x = self.intensity;
        } else {
            return;
        }
        self.target_y = 0.0;
        self.slide_reset_timer = SLIDE_RESET_SECS;
    }

    /// Returns the current parallax offset as `[offset_x, offset_y]`.
    pub fn get_offset(&self) -> [f32; 2] {
        [self.offset_x, self.offset_y]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_starts_at_zero() {
        let p = ParallaxState::new(0.05);
        assert_eq!(p.get_offset(), [0.0, 0.0]);
    }

    #[test]
    fn test_workspace_change_forward_sets_negative_target() {
        let mut p = ParallaxState::new(0.05);
        p.on_workspace_change(1, 2);
        assert!(p.target_x < 0.0);
        assert_eq!(p.target_x, -0.05);
    }

    #[test]
    fn test_workspace_change_backward_sets_positive_target() {
        let mut p = ParallaxState::new(0.05);
        p.on_workspace_change(3, 1);
        assert!(p.target_x > 0.0);
        assert_eq!(p.target_x, 0.05);
    }

    #[test]
    fn test_workspace_change_equal_is_noop() {
        let mut p = ParallaxState::new(0.05);
        p.on_workspace_change(2, 2);
        assert_eq!(p.target_x, 0.0);
        assert_eq!(p.slide_reset_timer, 0.0);
    }

    #[test]
    fn test_update_lerps_toward_target() {
        let mut p = ParallaxState::new(0.05);
        p.target_x = 0.05;
        // Use a realistic frame delta (16 ms). With speed=5.0:
        // offset += (0.05 - 0) * 5.0 * 0.016 = 0.004 → strictly between 0 and 0.05.
        p.update(0.016);
        assert!(p.offset_x > 0.0);
        assert!(p.offset_x < 0.05);
    }

    #[test]
    fn test_update_resets_target_after_timer_expires() {
        let mut p = ParallaxState::new(0.05);
        p.on_workspace_change(1, 2);
        assert!(p.slide_reset_timer > 0.0);
        p.update(SLIDE_RESET_SECS + 0.01);
        assert_eq!(p.target_x, 0.0);
        assert_eq!(p.slide_reset_timer, 0.0);
    }

    #[test]
    fn test_set_target_overrides_slide() {
        let mut p = ParallaxState::new(0.05);
        p.on_workspace_change(1, 2);
        p.set_target(0.1, 0.2);
        assert_eq!(p.target_x, 0.1);
        assert_eq!(p.target_y, 0.2);
    }
}
