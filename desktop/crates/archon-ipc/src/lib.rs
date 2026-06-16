//! # archon-ipc
//!
//! The control channel between the ArchonSync compositor and its clients
//! (`archonctl`, the shell). It is a tiny request/response protocol over a unix
//! domain socket: each frame is a little-endian `u32` length followed by a
//! bincode-encoded [`Request`] or [`Response`].
//!
//! The wire helpers ([`write_frame`] / [`read_frame`]) work over any
//! `Read`/`Write`, so the protocol types stay transport-agnostic and unit
//! testable against an in-memory pipe.

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::path::PathBuf;

/// Commands a client can send to the compositor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Request {
    /// Liveness probe; expects [`Response::Pong`].
    Ping,
    /// Lock the session immediately.
    Lock,
    /// Show or hide the dock.
    ToggleDock,
    /// Show or hide the radial control wheel.
    ToggleWheel,
    /// Switch to workspace `index` (0-based).
    SwitchWorkspace(u32),
    /// Move the focused window to workspace `index`.
    MoveToWorkspace(u32),
    /// Enable or disable Game Mode.
    SetGameMode(bool),
    /// Toggle the in-compositor FPS overlay.
    ToggleFpsOverlay,
    /// Re-derive the theme from the current wallpaper using `mode`
    /// (`"signature"`, `"harmonize"`, or `"contrast"`). This is "AI Enhance".
    AiEnhance { mode: String },
    /// Change the wallpaper and re-theme around it.
    SetWallpaper(PathBuf),
    /// Ask the compositor for a snapshot of its state.
    QueryState,
}

/// Replies from the compositor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Pong,
    /// Generic success with a human-readable message.
    Ok(String),
    /// The command failed; carries a human-readable reason.
    Error(String),
    /// Snapshot returned for [`Request::QueryState`].
    State(CompositorState),
}

/// A snapshot of compositor state, used by the shell to render status.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompositorState {
    pub current_workspace: u32,
    pub workspace_count: u32,
    pub window_count: u32,
    pub game_mode: bool,
    pub fps_overlay: bool,
    /// Current accent color as `#rrggbb`, for the shell to match.
    pub accent_hex: String,
}

/// Default socket path: `$XDG_RUNTIME_DIR/archonsync.sock`, falling back to a
/// path under `/tmp` keyed by uid when the runtime dir is unset.
pub fn default_socket_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("archonsync.sock");
    }
    let uid = unsafe { libc_getuid() };
    PathBuf::from(format!("/tmp/archonsync-{uid}.sock"))
}

// Avoid pulling in the whole `libc` crate just for getuid(); declare it.
extern "C" {
    #[link_name = "getuid"]
    fn libc_getuid() -> u32;
}

/// Maximum frame size we will accept (4 MiB). Guards against a malformed or
/// hostile length prefix forcing a huge allocation.
const MAX_FRAME: u32 = 4 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("encode/decode error: {0}")]
    Codec(#[from] bincode::Error),
    #[error("frame of {0} bytes exceeds the {MAX_FRAME}-byte limit")]
    FrameTooLarge(u32),
}

/// Encode `value` and write it as a length-prefixed frame.
pub fn write_frame<W: Write, T: Serialize>(w: &mut W, value: &T) -> Result<(), IpcError> {
    let bytes = bincode::serialize(value)?;
    let len = bytes.len() as u32;
    if len > MAX_FRAME {
        return Err(IpcError::FrameTooLarge(len));
    }
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&bytes)?;
    w.flush()?;
    Ok(())
}

/// Read one length-prefixed frame and decode it into `T`.
pub fn read_frame<R: Read, T: for<'de> Deserialize<'de>>(r: &mut R) -> Result<T, IpcError> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf);
    if len > MAX_FRAME {
        return Err(IpcError::FrameTooLarge(len));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf)?;
    Ok(bincode::deserialize(&buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn request_roundtrips_over_a_frame() {
        let req = Request::AiEnhance { mode: "contrast".into() };
        let mut buf = Vec::new();
        write_frame(&mut buf, &req).unwrap();
        let mut cur = Cursor::new(buf);
        let got: Request = read_frame(&mut cur).unwrap();
        assert_eq!(got, req);
    }

    #[test]
    fn state_response_roundtrips() {
        let resp = Response::State(CompositorState {
            current_workspace: 2,
            workspace_count: 4,
            window_count: 7,
            game_mode: true,
            fps_overlay: false,
            accent_hex: "#ff7a1a".into(),
        });
        let mut buf = Vec::new();
        write_frame(&mut buf, &resp).unwrap();
        let got: Response = read_frame(&mut Cursor::new(buf)).unwrap();
        assert_eq!(got, resp);
    }

    #[test]
    fn oversized_length_is_rejected() {
        // A length prefix bigger than MAX_FRAME, with no body.
        let mut bytes = (MAX_FRAME + 1).to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0u8; 8]);
        let err = read_frame::<_, Request>(&mut Cursor::new(bytes)).unwrap_err();
        assert!(matches!(err, IpcError::FrameTooLarge(_)));
    }

    #[test]
    fn two_frames_back_to_back() {
        let mut buf = Vec::new();
        write_frame(&mut buf, &Request::Ping).unwrap();
        write_frame(&mut buf, &Request::Lock).unwrap();
        let mut cur = Cursor::new(buf);
        assert_eq!(read_frame::<_, Request>(&mut cur).unwrap(), Request::Ping);
        assert_eq!(read_frame::<_, Request>(&mut cur).unwrap(), Request::Lock);
    }
}
