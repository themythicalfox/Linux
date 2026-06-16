//! # archon-lock
//!
//! The ArchonSync lock screen. The wallpaper sits behind a blur with subtle
//! parallax; a large clock/title fades into a password field when focused; and
//! by default the user unlocks with the orange edge-swipe gesture
//! ([`swipe::SwipeTracker`]), with a PAM-backed password/PIN as fallback.
//!
//! The state machine and scene building are pure and tested. Real credential
//! checking goes through the [`Authenticator`] trait; the PAM implementation is
//! behind the `pam-auth` feature so tests use a mock.

mod swipe;

pub use swipe::{SwipeRelease, SwipeTracker};

use archon_config::LockConfig;
use archon_theme::{Color, Theme};
use archon_ui::scene::{DrawList, Primitive, Rect, Vec2};
use archon_ui::ParticleSystem;

/// Pluggable credential check so the UI never touches PAM directly (and tests
/// can inject a mock).
pub trait Authenticator {
    /// Return true if `secret` authenticates `user`.
    fn authenticate(&mut self, user: &str, secret: &str) -> bool;
}

/// An authenticator that accepts a single known secret. Used in tests and as a
/// safe default when PAM is not compiled in.
pub struct MockAuthenticator {
    pub user: String,
    pub secret: String,
}

impl Authenticator for MockAuthenticator {
    fn authenticate(&mut self, user: &str, secret: &str) -> bool {
        user == self.user && secret == self.secret
    }
}

/// What the lock screen is currently doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Showing the clock/title; waiting for a swipe or a key to type.
    Idle,
    /// An edge swipe is in progress.
    Swiping,
    /// The password field is focused and accepting input.
    PasswordEntry,
    /// Credentials accepted — the compositor can tear the lock down.
    Unlocked,
}

/// The lock screen state and presentation.
pub struct LockScreen {
    cfg: LockConfig,
    theme: Theme,
    width: f32,
    height: f32,
    phase: Phase,
    swipe: SwipeTracker,
    particles: ParticleSystem,
    /// Typed password/PIN buffer (never logged or rendered as plain text).
    secret: String,
    /// Big centered title shown before focus (e.g. the time or "ArchonSync").
    pub title: String,
    /// Pointer position, for wallpaper parallax.
    pointer: Vec2,
    /// Set when the last password attempt failed, to flash the field.
    pub auth_failed: bool,
}

impl LockScreen {
    pub fn new(cfg: LockConfig, theme: Theme, width: f32, height: f32) -> Self {
        let threshold = cfg.swipe_threshold;
        LockScreen {
            cfg,
            theme,
            width,
            height,
            phase: Phase::Idle,
            swipe: SwipeTracker::new(width, threshold),
            particles: ParticleSystem::new(0x5eed, 512),
            secret: String::new(),
            title: "ArchonSync".into(),
            pointer: Vec2::new(width / 2.0, height / 2.0),
            auth_failed: false,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn is_unlocked(&self) -> bool {
        self.phase == Phase::Unlocked
    }

    /// Pointer pressed. Arms the swipe if it began at the edge.
    pub fn on_press(&mut self, x: f32, y: f32) {
        self.pointer = Vec2::new(x, y);
        if self.phase == Phase::Idle && self.swipe.press(x) {
            self.phase = Phase::Swiping;
        }
    }

    /// Pointer moved. Drives the swipe and parallax, and sprays particles off
    /// the charging dot.
    pub fn on_drag(&mut self, x: f32, y: f32) {
        self.pointer = Vec2::new(x, y);
        if self.phase == Phase::Swiping {
            self.swipe.drag(x);
            // Emit a few embers from the dot, scaled by how charged it is.
            let glow = self.swipe.glow();
            let count = (glow * 2.0) as usize;
            self.particles.emit(
                self.swipe.dot_x(),
                self.height / 2.0,
                count,
                60.0,
                0.6,
                self.theme.accent,
            );
        }
    }

    /// Pointer released. May unlock (if swiping past threshold and swipe-to-
    /// unlock is enabled) or fall back to requiring a password.
    pub fn on_release(&mut self) {
        if self.phase != Phase::Swiping {
            return;
        }
        match self.swipe.release() {
            SwipeRelease::Unlocked => {
                if self.cfg.swipe_to_unlock {
                    self.burst_unlock();
                    self.phase = Phase::Unlocked;
                } else {
                    // Swipe reveals the password field instead of unlocking.
                    self.phase = Phase::PasswordEntry;
                }
            }
            SwipeRelease::Reset => self.phase = Phase::Idle,
        }
    }

    /// A character was typed. Switches to password entry and appends to the
    /// secret buffer. Returns false for control handling elsewhere.
    pub fn on_char(&mut self, c: char) {
        if c.is_control() {
            return;
        }
        if self.phase == Phase::Idle || self.phase == Phase::Swiping {
            self.phase = Phase::PasswordEntry;
        }
        if self.phase == Phase::PasswordEntry {
            self.auth_failed = false;
            self.secret.push(c);
        }
    }

    /// Backspace in the password field.
    pub fn on_backspace(&mut self) {
        if self.phase == Phase::PasswordEntry {
            self.secret.pop();
        }
    }

    /// Submit the typed secret to `auth`. On success transitions to Unlocked;
    /// on failure flags `auth_failed` and clears the buffer.
    pub fn submit(&mut self, user: &str, auth: &mut dyn Authenticator) {
        if self.phase != Phase::PasswordEntry {
            return;
        }
        if auth.authenticate(user, &self.secret) {
            self.burst_unlock();
            self.phase = Phase::Unlocked;
        } else {
            self.auth_failed = true;
        }
        self.secret.clear();
    }

    /// Advance animations (the dot spring and particles).
    pub fn tick(&mut self, dt: f32) {
        self.swipe.tick(dt);
        self.particles.step(dt);
    }

    /// Wallpaper parallax offset, derived from how far the pointer is from
    /// center scaled by the configured strength. The renderer shifts the
    /// blurred wallpaper by this much.
    pub fn parallax_offset(&self) -> Vec2 {
        let cx = self.width / 2.0;
        let cy = self.height / 2.0;
        Vec2::new(
            (self.pointer.x - cx) / cx * self.cfg.parallax,
            (self.pointer.y - cy) / cy * self.cfg.parallax,
        )
    }

    /// Build the frame to render. The wallpaper itself is drawn by the
    /// compositor (it owns the texture + blur); this adds the lock chrome on
    /// top: the title/clock, the password field when focused, the glowing swipe
    /// dot, and any live particles.
    pub fn build_scene(&self) -> DrawList {
        let mut dl = DrawList::new();
        let cx = self.width / 2.0;
        let cy = self.height / 2.0;

        // Title / clock, fading out as the password field takes over.
        if self.phase != Phase::PasswordEntry {
            dl.push(Primitive::Text {
                pos: Vec2::new(cx - 160.0, cy - 80.0),
                content: self.title.clone(),
                size: 72.0,
                color: self.theme.text,
            });
        } else {
            // Password field: a glass pill with masked dots.
            let field = Rect::new(cx - 180.0, cy - 28.0, 360.0, 56.0);
            let border = if self.auth_failed { self.theme.negative } else { self.theme.accent };
            dl.glass_panel(field, 28.0, self.theme.surface_fill(archon_theme::SurfaceMode::Glass), border, self.theme.blur_radius);
            let dots: String = "•".repeat(self.secret.chars().count().min(24));
            dl.push(Primitive::Text {
                pos: Vec2::new(field.x + 24.0, cy - 14.0),
                content: dots,
                size: 28.0,
                color: self.theme.text,
            });
        }

        // The swipe dot lives on the vertical centerline; brighter as it charges.
        let dot_x = if self.phase == Phase::Swiping { self.swipe.dot_x() } else { 28.0 };
        dl.glow_dot(Vec2::new(dot_x, cy), 14.0, self.theme.accent, self.swipe.glow());

        // A faint guide track hinting "swipe right to unlock".
        if self.phase == Phase::Idle || self.phase == Phase::Swiping {
            dl.push(Primitive::RoundedRect {
                rect: Rect::new(28.0, cy - 2.0, self.width * self.cfg.swipe_threshold, 4.0),
                radius: 2.0,
                fill: self.theme.accent.with_alpha(40),
                border: Color::rgba(0, 0, 0, 0),
                border_width: 0.0,
                blur: 0.0,
            });
        }

        // Live particles, if any.
        if !self.particles.is_empty() {
            dl.push(Primitive::Particles(self.particles.particles.clone()));
        }
        dl
    }

    /// A celebratory burst when the screen unlocks.
    fn burst_unlock(&mut self) {
        let x = self.swipe.dot_x().max(28.0);
        self.particles.emit(x, self.height / 2.0, 120, 220.0, 0.9, self.theme.accent);
    }
}

/// PAM-backed authenticator (only with the `pam-auth` feature).
#[cfg(feature = "pam-auth")]
pub struct PamAuthenticator {
    service: String,
}

#[cfg(feature = "pam-auth")]
impl PamAuthenticator {
    /// Use the given PAM service (e.g. `"archonsync"` or `"login"`).
    pub fn new(service: impl Into<String>) -> Self {
        PamAuthenticator { service: service.into() }
    }
}

#[cfg(feature = "pam-auth")]
impl Authenticator for PamAuthenticator {
    fn authenticate(&mut self, user: &str, secret: &str) -> bool {
        match pam::Authenticator::with_password(&self.service) {
            Ok(mut auth) => {
                auth.get_handler().set_credentials(user, secret);
                auth.authenticate().is_ok() && auth.open_session().is_ok()
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> LockScreen {
        LockScreen::new(LockConfig::default(), Theme::default(), 1000.0, 800.0)
    }

    #[test]
    fn swipe_past_threshold_unlocks_when_enabled() {
        let mut s = screen();
        s.on_press(10.0, 400.0);
        assert_eq!(s.phase(), Phase::Swiping);
        s.on_drag(800.0, 400.0); // past 0.75 * 1000
        s.on_release();
        assert!(s.is_unlocked());
    }

    #[test]
    fn short_swipe_returns_to_idle() {
        let mut s = screen();
        s.on_press(10.0, 400.0);
        s.on_drag(100.0, 400.0);
        s.on_release();
        assert_eq!(s.phase(), Phase::Idle);
        assert!(!s.is_unlocked());
    }

    #[test]
    fn typing_switches_to_password_entry() {
        let mut s = screen();
        s.on_char('h');
        s.on_char('i');
        assert_eq!(s.phase(), Phase::PasswordEntry);
        s.on_backspace();
        // Wrong password flags failure, right one unlocks.
        let mut auth = MockAuthenticator { user: "archon".into(), secret: "hunter2".into() };
        s.submit("archon", &mut auth);
        assert!(s.auth_failed);
        for c in "hunter2".chars() {
            s.on_char(c);
        }
        s.submit("archon", &mut auth);
        assert!(s.is_unlocked());
    }

    #[test]
    fn scene_has_content_in_every_phase() {
        let mut s = screen();
        assert!(!s.build_scene().is_empty()); // idle: title + dot + track
        s.on_char('x');
        assert!(!s.build_scene().is_empty()); // password field
    }

    #[test]
    fn parallax_is_zero_at_center() {
        let s = screen();
        let off = s.parallax_offset();
        assert!(off.x.abs() < 1e-3 && off.y.abs() < 1e-3);
    }
}
