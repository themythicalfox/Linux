//! `archonctl` — a tiny CLI that drives the running ArchonSync compositor over
//! its IPC socket. Used by keybindings, the shell, scripts and the user.
//!
//! Examples:
//!   archonctl lock
//!   archonctl workspace 3
//!   archonctl game-mode on
//!   archonctl ai-enhance contrast
//!   archonctl state

use archon_ipc::{default_socket_path, read_frame, write_frame, Request, Response};
use std::os::unix::net::UnixStream;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let request = match parse(&args) {
        Ok(r) => r,
        Err(ParseError::Help) => {
            print_usage();
            return ExitCode::SUCCESS;
        }
        Err(ParseError::Bad(msg)) => {
            eprintln!("archonctl: {msg}\n");
            print_usage();
            return ExitCode::FAILURE;
        }
    };

    match send(request) {
        Ok(Response::Pong) => {
            println!("pong");
            ExitCode::SUCCESS
        }
        Ok(Response::Ok(msg)) => {
            if !msg.is_empty() {
                println!("{msg}");
            }
            ExitCode::SUCCESS
        }
        Ok(Response::State(s)) => {
            println!("workspace   : {} / {}", s.current_workspace + 1, s.workspace_count);
            println!("windows     : {}", s.window_count);
            println!("game mode   : {}", if s.game_mode { "on" } else { "off" });
            println!("fps overlay : {}", if s.fps_overlay { "on" } else { "off" });
            println!("accent      : {}", s.accent_hex);
            ExitCode::SUCCESS
        }
        Ok(Response::Error(e)) => {
            eprintln!("archonctl: compositor error: {e}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("archonctl: {e}");
            ExitCode::FAILURE
        }
    }
}

enum ParseError {
    Help,
    Bad(String),
}

fn parse(args: &[String]) -> Result<Request, ParseError> {
    let cmd = args.first().map(String::as_str).ok_or(ParseError::Help)?;
    let arg = |i: usize| args.get(i).map(String::as_str);
    let on_off = |v: Option<&str>| match v {
        Some("on") | Some("true") | Some("1") => Ok(true),
        Some("off") | Some("false") | Some("0") => Ok(false),
        _ => Err(ParseError::Bad("expected `on` or `off`".into())),
    };

    Ok(match cmd {
        "-h" | "--help" | "help" => return Err(ParseError::Help),
        "ping" => Request::Ping,
        "lock" => Request::Lock,
        "toggle-dock" => Request::ToggleDock,
        "toggle-wheel" => Request::ToggleWheel,
        "fps" | "toggle-fps" => Request::ToggleFpsOverlay,
        "state" => Request::QueryState,
        "workspace" => {
            let n: u32 = arg(1)
                .ok_or_else(|| ParseError::Bad("workspace needs an index".into()))?
                .parse()
                .map_err(|_| ParseError::Bad("workspace index must be a number".into()))?;
            // Accept 1-based on the CLI, convert to 0-based on the wire.
            Request::SwitchWorkspace(n.saturating_sub(1))
        }
        "move-to" => {
            let n: u32 = arg(1)
                .ok_or_else(|| ParseError::Bad("move-to needs an index".into()))?
                .parse()
                .map_err(|_| ParseError::Bad("workspace index must be a number".into()))?;
            Request::MoveToWorkspace(n.saturating_sub(1))
        }
        "game-mode" => Request::SetGameMode(on_off(arg(1))?),
        "ai-enhance" => {
            let mode = arg(1).unwrap_or("harmonize").to_string();
            if !matches!(mode.as_str(), "signature" | "harmonize" | "contrast") {
                return Err(ParseError::Bad("mode must be signature|harmonize|contrast".into()));
            }
            Request::AiEnhance { mode }
        }
        "wallpaper" => {
            let path = arg(1).ok_or_else(|| ParseError::Bad("wallpaper needs a path".into()))?;
            Request::SetWallpaper(path.into())
        }
        other => return Err(ParseError::Bad(format!("unknown command `{other}`"))),
    })
}

fn send(request: Request) -> Result<Response, String> {
    let path = default_socket_path();
    let mut stream = UnixStream::connect(&path)
        .map_err(|e| format!("cannot reach the compositor at {} ({e}). Is it running?", path.display()))?;
    write_frame(&mut stream, &request).map_err(|e| format!("send failed: {e}"))?;
    read_frame(&mut stream).map_err(|e| format!("no reply: {e}"))
}

fn print_usage() {
    eprintln!(
        "archonctl — control the ArchonSync compositor\n\n\
         USAGE:\n\
         \tarchonctl <command> [args]\n\n\
         COMMANDS:\n\
         \tping                     check the compositor is alive\n\
         \tlock                     lock the session\n\
         \ttoggle-dock              show/hide the dock\n\
         \ttoggle-wheel             show/hide the control wheel\n\
         \tfps                      toggle the FPS overlay\n\
         \tworkspace <n>            switch to workspace n (1-based)\n\
         \tmove-to <n>              move the focused window to workspace n\n\
         \tgame-mode <on|off>       enable/disable Game Mode\n\
         \tai-enhance <mode>        re-theme from wallpaper: signature|harmonize|contrast\n\
         \twallpaper <path>         set the wallpaper and re-theme\n\
         \tstate                    print compositor status"
    );
}
