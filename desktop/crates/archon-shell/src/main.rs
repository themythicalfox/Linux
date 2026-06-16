//! `archon-shell` — hosts the dock and control wheel.
//!
//! On a running ArchonSync desktop this process owns two `wlr-layer-shell`
//! surfaces (the dock and the wheel overlay), renders them with `archon-ui`'s
//! GPU backend, and forwards their actions to the compositor over IPC. That
//! Wayland-client + GPU plumbing is the shell's next iteration; today the binary
//! validates the pieces it will use: it loads config + theme, builds the dock
//! and wheel scenes, and (if the compositor is up) queries its state over IPC.
//!
//! Run `archon-shell --demo` to print what it would draw without a compositor.

use archon_ipc::{default_socket_path, read_frame, write_frame, Request, Response};
use archon_shell::{Shell, WheelItem};
use archon_theme::{AccentMode, Theme};
use archon_ui::scene::Vec2;
use std::os::unix::net::UnixStream;
use std::process::ExitCode;

fn main() -> ExitCode {
    let demo = std::env::args().any(|a| a == "--demo");

    let config = archon_config::Config::load(archon_config::Config::default_path())
        .unwrap_or_default();

    // Derive the theme from the configured wallpaper, falling back to the
    // signature dark theme if the image can't be read (e.g. in CI).
    let theme = archon_theme::theme_from_wallpaper(&config.general.wallpaper, AccentMode::Harmonize)
        .unwrap_or_else(|_| Theme::default());

    let mut shell = Shell::new(&config, theme);
    shell.toggle_wheel(); // make the wheel scene available for the demo

    let center = Vec2::new(960.0, 540.0);
    let wheel_scene = shell.wheel_scene(center).expect("wheel is visible");
    let dock_rect = shell.dock.rect(1920.0, 1080.0, 640.0);

    println!("ArchonSync shell");
    println!("  accent      : {}", shell.theme.accent.to_hex());
    println!("  dock edge   : {:?}", shell.dock.edge());
    println!("  dock rect   : {:.0},{:.0} {:.0}x{:.0}", dock_rect.x, dock_rect.y, dock_rect.w, dock_rect.h);
    println!("  wheel items : {}", shell.wheel.len());
    println!("  wheel draws : {} primitives", wheel_scene.len());
    if let Some(sel) = shell.wheel.selected_item() {
        println!("  selected    : {} ({})", sel.label, sel.icon);
    }
    let _: Option<&WheelItem> = shell.wheel.selected_item();

    if demo {
        return ExitCode::SUCCESS;
    }

    // Best-effort: report compositor state if it's running.
    match query_state() {
        Ok(state) => println!(
            "  compositor  : workspace {}/{}, {} windows, accent {}",
            state.current_workspace + 1,
            state.workspace_count,
            state.window_count,
            state.accent_hex,
        ),
        Err(e) => println!("  compositor  : not reachable ({e})"),
    }
    ExitCode::SUCCESS
}

fn query_state() -> Result<archon_ipc::CompositorState, String> {
    let path = default_socket_path();
    let mut stream = UnixStream::connect(&path).map_err(|e| e.to_string())?;
    write_frame(&mut stream, &Request::QueryState).map_err(|e| e.to_string())?;
    match read_frame::<_, Response>(&mut stream).map_err(|e| e.to_string())? {
        Response::State(s) => Ok(s),
        other => Err(format!("unexpected reply: {other:?}")),
    }
}
