//! # archon-ui
//!
//! The shared UI toolkit every ArchonSync surface is built from. It has two
//! layers:
//!
//! * A **pure core** — [`anim`] (easing + spring physics), [`particles`], and
//!   [`scene`] (a backend-agnostic [`DrawList`]). This builds and is fully
//!   unit-tested with no GPU, so the *feel* and *layout* of the desktop are
//!   verified in CI.
//! * An optional **wgpu renderer** ([`render`], behind the `gpu` feature) that
//!   turns a [`DrawList`] into pixels with rounded-rect, glow, glass-blur and
//!   particle shaders. Real desktop builds enable `gpu`; logic tests do not.
//!
//! The split is what lets the project promise "compiles and is tested headless"
//! while still shipping a genuine GPU path.

pub mod anim;
pub mod particles;
pub mod scene;

#[cfg(feature = "gpu")]
pub mod render;

pub use anim::{Easing, Spring, Tween};
pub use particles::{Particle, ParticleSystem};
pub use scene::{DrawList, Primitive, Rect, Vec2};

// Re-export the shared color type so downstream crates have one source of truth.
pub use archon_theme::{Color, SurfaceMode, Theme};
