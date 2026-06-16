//! A small CPU particle system for the lock-screen swipe feedback and other
//! flourishes. Simulation is pure and deterministic given a seed + timestep, so
//! the renderer just uploads the live particle set each frame.

use archon_theme::Color;

/// One particle. Position/velocity are in logical pixels and pixels-per-second.
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    /// Remaining life in seconds.
    pub life: f32,
    /// Life it spawned with, for fading.
    pub max_life: f32,
    pub size: f32,
    pub color: Color,
}

impl Particle {
    /// Fade factor `0..=1` derived from remaining life — 1 when fresh, 0 at death.
    pub fn alpha(&self) -> f32 {
        (self.life / self.max_life).clamp(0.0, 1.0)
    }
}

/// A tiny deterministic PRNG (xorshift) so particle spread is reproducible in
/// tests without pulling in the `rand` crate.
#[derive(Clone, Copy, Debug)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(seed.max(1))
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Uniform float in `0..1`.
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform float in `-1..1`.
    fn signed(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }
}

/// Particle emitter + simulator.
#[derive(Clone, Debug)]
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    /// Downward acceleration (px/s²). The lock effect uses a gentle upward
    /// drift, so this is typically small or negative.
    pub gravity: f32,
    rng: Rng,
    cap: usize,
}

impl ParticleSystem {
    pub fn new(seed: u32, cap: usize) -> Self {
        ParticleSystem {
            particles: Vec::new(),
            gravity: -40.0,
            rng: Rng::new(seed),
            cap: cap.max(1),
        }
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Emit `count` particles in a burst around `(x, y)`, spraying outward with
    /// up to `speed` px/s and living `life` seconds. Respects the capacity cap.
    pub fn emit(&mut self, x: f32, y: f32, count: usize, speed: f32, life: f32, color: Color) {
        for _ in 0..count {
            if self.particles.len() >= self.cap {
                break;
            }
            let angle = self.rng.next_f32() * std::f32::consts::TAU;
            let mag = speed * (0.4 + 0.6 * self.rng.next_f32());
            let jitter = 2.0;
            self.particles.push(Particle {
                x: x + self.rng.signed() * jitter,
                y: y + self.rng.signed() * jitter,
                vx: angle.cos() * mag,
                vy: angle.sin() * mag,
                life,
                max_life: life,
                size: 2.0 + self.rng.next_f32() * 3.0,
                color,
            });
        }
    }

    /// Advance all particles by `dt` seconds and drop any that have died.
    pub fn step(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        for p in &mut self.particles {
            p.vy += self.gravity * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_respects_capacity() {
        let mut ps = ParticleSystem::new(1, 10);
        ps.emit(0.0, 0.0, 100, 50.0, 1.0, Color::rgb(255, 122, 26));
        assert_eq!(ps.len(), 10);
    }

    #[test]
    fn particles_die_after_their_life() {
        let mut ps = ParticleSystem::new(7, 64);
        ps.emit(100.0, 100.0, 20, 30.0, 0.5, Color::rgb(255, 122, 26));
        assert_eq!(ps.len(), 20);
        // Step well past their lifetime.
        for _ in 0..10 {
            ps.step(0.1);
        }
        assert!(ps.is_empty());
    }

    #[test]
    fn alpha_fades_with_life() {
        let mut ps = ParticleSystem::new(3, 8);
        ps.emit(0.0, 0.0, 1, 10.0, 1.0, Color::rgb(255, 255, 255));
        let fresh = ps.particles[0].alpha();
        ps.step(0.5);
        let half = ps.particles[0].alpha();
        assert!(fresh > half);
        assert!((fresh - 1.0).abs() < 1e-3);
    }

    #[test]
    fn simulation_is_deterministic() {
        let run = || {
            let mut ps = ParticleSystem::new(42, 64);
            ps.emit(50.0, 50.0, 16, 40.0, 1.0, Color::rgb(255, 122, 26));
            ps.step(0.25);
            ps.particles.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }
}
