//! Animation: easing curves and spring physics.
//!
//! ArchonSync's "buttery, physics-based" motion comes from two primitives:
//!
//! * [`Easing`] — classic time-remapping curves for scripted transitions
//!   (fades, slides) with a fixed duration.
//! * [`Spring`] — a critically-tunable damped spring for motion that should
//!   feel reactive and interruptible (the dock sliding in, a widget snapping to
//!   a grid, the lock dot following the pointer). Springs have no fixed
//!   duration; they chase a target and settle.
//!
//! Both are pure and deterministic given a timestep, so the feel of the desktop
//! is unit-tested rather than eyeballed.

/// Standard easing curves. Each maps a normalized time `t` in `0..=1` to an
/// eased progress, also in `0..=1`, with `f(0) == 0` and `f(1) == 1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Easing {
    Linear,
    EaseInQuad,
    EaseOutQuad,
    #[default]
    EaseInOutCubic,
    EaseOutExpo,
    /// Slight overshoot then settle — good for playful pop-in.
    EaseOutBack,
}

impl Easing {
    /// Evaluate the curve at `t` (clamped to `0..=1`).
    pub fn eval(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::EaseOutExpo => {
                if t >= 1.0 {
                    1.0
                } else {
                    1.0 - 2f32.powf(-10.0 * t)
                }
            }
            Easing::EaseOutBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
            }
        }
    }
}

/// A scripted tween from `from` to `to` over `duration` seconds under an easing
/// curve. Call [`Tween::value`] with elapsed time to sample it.
#[derive(Clone, Copy, Debug)]
pub struct Tween {
    pub from: f32,
    pub to: f32,
    pub duration: f32,
    pub easing: Easing,
}

impl Tween {
    pub fn new(from: f32, to: f32, duration: f32, easing: Easing) -> Self {
        Tween { from, to, duration, easing }
    }

    /// Sample the tween at `elapsed` seconds.
    pub fn value(&self, elapsed: f32) -> f32 {
        if self.duration <= 0.0 {
            return self.to;
        }
        let t = (elapsed / self.duration).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * self.easing.eval(t)
    }

    /// Has the tween reached its end?
    pub fn finished(&self, elapsed: f32) -> bool {
        elapsed >= self.duration
    }
}

/// A damped spring chasing a target value.
///
/// Parameterized by `stiffness` (how hard it pulls toward the target) and
/// `damping` (how quickly oscillation bleeds off). The defaults are tuned for a
/// snappy-but-smooth UI response. Integrated with a semi-implicit Euler step,
/// which stays stable across the frame intervals a 120 Hz compositor produces.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
    pub target: f32,
    pub stiffness: f32,
    pub damping: f32,
}

impl Spring {
    /// A spring tuned for responsive UI motion, starting settled at `value`.
    pub fn new(value: f32) -> Self {
        Spring {
            value,
            velocity: 0.0,
            target: value,
            stiffness: 180.0,
            damping: 26.0,
        }
    }

    /// Builder: set a custom stiffness/damping pair.
    pub fn with_params(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness;
        self.damping = damping;
        self
    }

    /// Point the spring at a new target without disturbing current motion.
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Advance the simulation by `dt` seconds. Large `dt` values are sub-stepped
    /// so a hitched frame can't blow the integrator up.
    pub fn step(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        // Cap the per-substep interval for stability, then run as many as needed.
        let max_step = 1.0 / 120.0;
        let mut remaining = dt;
        while remaining > 0.0 {
            let h = remaining.min(max_step);
            let force = -self.stiffness * (self.value - self.target) - self.damping * self.velocity;
            self.velocity += force * h;
            self.value += self.velocity * h;
            remaining -= h;
        }
    }

    /// True once the spring is within `eps` of its target and nearly still.
    pub fn settled(&self, eps: f32) -> bool {
        (self.value - self.target).abs() < eps && self.velocity.abs() < eps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_endpoints_are_exact() {
        for e in [
            Easing::Linear,
            Easing::EaseInQuad,
            Easing::EaseOutQuad,
            Easing::EaseInOutCubic,
            Easing::EaseOutExpo,
            Easing::EaseOutBack,
        ] {
            assert!((e.eval(0.0)).abs() < 1e-3, "{e:?} f(0)");
            assert!((e.eval(1.0) - 1.0).abs() < 1e-3, "{e:?} f(1)");
        }
    }

    #[test]
    fn ease_in_out_is_monotonic() {
        let mut prev = -1.0;
        for i in 0..=100 {
            let v = Easing::EaseInOutCubic.eval(i as f32 / 100.0);
            assert!(v >= prev - 1e-4, "non-monotonic at {i}");
            prev = v;
        }
    }

    #[test]
    fn tween_clamps_outside_range() {
        let tw = Tween::new(0.0, 100.0, 2.0, Easing::Linear);
        assert_eq!(tw.value(-1.0), 0.0);
        assert_eq!(tw.value(0.0), 0.0);
        assert!((tw.value(1.0) - 50.0).abs() < 1e-3);
        assert_eq!(tw.value(5.0), 100.0);
        assert!(tw.finished(2.0));
    }

    #[test]
    fn spring_converges_to_target() {
        let mut s = Spring::new(0.0);
        s.set_target(1.0);
        for _ in 0..600 {
            s.step(1.0 / 120.0);
        }
        assert!(s.settled(1e-2), "value={} vel={}", s.value, s.velocity);
        assert!((s.value - 1.0).abs() < 1e-2);
    }

    #[test]
    fn spring_is_stable_under_a_huge_timestep() {
        let mut s = Spring::new(0.0);
        s.set_target(10.0);
        // A 1-second hitch must not produce NaN/inf via sub-stepping.
        s.step(1.0);
        assert!(s.value.is_finite() && s.velocity.is_finite());
    }
}
