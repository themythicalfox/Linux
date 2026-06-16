//! The orange edge-swipe unlock gesture.
//!
//! A [`SwipeTracker`] models the futuristic "biometric swipe": the user presses
//! near the left edge and drags a glowing dot across the screen. Progress is the
//! fraction of the configured travel distance covered; once it passes the
//! threshold the gesture unlocks. Releasing early springs the dot back. The
//! tracker is pure state — the renderer reads [`SwipeTracker::dot_x`] and
//! [`SwipeTracker::glow`] each frame — so the gesture feel is unit tested.

use archon_ui::Spring;

/// Outcome of releasing the pointer mid-gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwipeRelease {
    /// Travelled past the threshold — unlock.
    Unlocked,
    /// Released early — the dot springs back and the screen stays locked.
    Reset,
}

/// Tracks an in-progress edge swipe.
#[derive(Clone, Debug)]
pub struct SwipeTracker {
    /// Screen width in logical pixels.
    width: f32,
    /// Fraction of width that must be covered to unlock (`0..=1`).
    threshold: f32,
    /// How close to the left edge a press must start to begin a swipe.
    edge_zone: f32,
    /// Whether a swipe is currently active (pointer down, started at the edge).
    active: bool,
    /// Raw pointer X while dragging.
    pointer_x: f32,
    /// The dot's animated X (spring-follows `pointer_x`, springs home on reset).
    dot: Spring,
}

impl SwipeTracker {
    pub fn new(width: f32, threshold: f32) -> Self {
        SwipeTracker {
            width: width.max(1.0),
            threshold: threshold.clamp(0.05, 1.0),
            edge_zone: 64.0,
            active: false,
            pointer_x: 0.0,
            dot: Spring::new(0.0).with_params(220.0, 30.0),
        }
    }

    /// Begin a swipe if the press started within the left edge zone. Returns
    /// true if a gesture was armed.
    pub fn press(&mut self, x: f32) -> bool {
        if x <= self.edge_zone {
            self.active = true;
            self.pointer_x = x;
            self.dot.set_target(x);
            true
        } else {
            false
        }
    }

    /// Update the pointer position during a drag. Ignored if no swipe is armed.
    pub fn drag(&mut self, x: f32) {
        if self.active {
            self.pointer_x = x.clamp(0.0, self.width);
            self.dot.set_target(self.pointer_x);
        }
    }

    /// Release the pointer; decide whether the swipe unlocked.
    pub fn release(&mut self) -> SwipeRelease {
        if !self.active {
            return SwipeRelease::Reset;
        }
        self.active = false;
        if self.progress() >= self.threshold {
            SwipeRelease::Unlocked
        } else {
            // Spring the dot back to the edge.
            self.dot.set_target(0.0);
            SwipeRelease::Reset
        }
    }

    /// Advance the dot's spring animation by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        self.dot.step(dt);
    }

    /// Current unlock progress in `0..=1` (based on the raw pointer, so the
    /// unlock decision is immediate and not delayed by the dot's spring).
    pub fn progress(&self) -> f32 {
        (self.pointer_x / (self.width * self.threshold)).clamp(0.0, 1.0)
    }

    /// The dot's animated X position for rendering.
    pub fn dot_x(&self) -> f32 {
        self.dot.value
    }

    /// Glow intensity for the dot, ramping up with progress so it "charges" as
    /// it crosses the screen.
    pub fn glow(&self) -> f32 {
        0.6 + 1.4 * self.progress()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Resize handling (multi-monitor / mode change).
    pub fn set_width(&mut self, width: f32) {
        self.width = width.max(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swipe_must_start_at_edge() {
        let mut s = SwipeTracker::new(1000.0, 0.75);
        assert!(!s.press(500.0)); // mid-screen press is ignored
        assert!(!s.is_active());
        assert!(s.press(20.0)); // edge press arms the gesture
        assert!(s.is_active());
    }

    #[test]
    fn crossing_threshold_unlocks() {
        let mut s = SwipeTracker::new(1000.0, 0.75);
        s.press(10.0);
        s.drag(760.0); // 760 / (1000*0.75=750) > 1.0 -> full progress
        assert_eq!(s.release(), SwipeRelease::Unlocked);
    }

    #[test]
    fn releasing_early_resets() {
        let mut s = SwipeTracker::new(1000.0, 0.75);
        s.press(10.0);
        s.drag(300.0); // below threshold
        assert_eq!(s.release(), SwipeRelease::Reset);
        assert!(!s.is_active());
    }

    #[test]
    fn glow_increases_with_progress() {
        let mut s = SwipeTracker::new(1000.0, 0.75);
        s.press(10.0);
        let low = s.glow();
        s.drag(700.0);
        assert!(s.glow() > low);
    }

    #[test]
    fn dot_springs_back_after_reset() {
        let mut s = SwipeTracker::new(1000.0, 0.75);
        s.press(10.0);
        s.drag(300.0);
        s.release();
        for _ in 0..600 {
            s.tick(1.0 / 120.0);
        }
        assert!(s.dot_x().abs() < 1.0, "dot should return to the edge");
    }

    #[test]
    fn drag_without_press_is_ignored() {
        let mut s = SwipeTracker::new(1000.0, 0.75);
        s.drag(900.0);
        assert_eq!(s.progress(), 0.0);
        assert_eq!(s.release(), SwipeRelease::Reset);
    }
}
