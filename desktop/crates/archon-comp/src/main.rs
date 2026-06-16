//! `archon-comp` — the ArchonSync compositor binary.
//!
//! Usage:
//!   archon-comp [--backend winit|udev]
//!
//! `winit` runs Lennox-style nested inside an existing Wayland/X session (for
//! development and testing). `udev` runs on bare metal from a TTY (how the ISO
//! launches it). The actual runtime lives behind the `smithay-backend` feature
//! so the window-management core can be tested without dragging in the full
//! graphics stack.

use std::process::ExitCode;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "archon_comp=info".into()),
        )
        .init();

    let backend = parse_backend();

    // Load config up front so a bad config fails fast with a clear message,
    // regardless of which backend (or no backend) is compiled in.
    let config_path = archon_config::Config::default_path();
    let config = match archon_config::Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("archon-comp: failed to load {}: {e}", config_path.display());
            return ExitCode::FAILURE;
        }
    };
    tracing::info!(?backend, "starting ArchonSync compositor");

    run(backend, config)
}

#[derive(Clone, Copy, Debug)]
enum Backend {
    Winit,
    Udev,
}

fn parse_backend() -> Backend {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--backend" {
            match args.next().as_deref() {
                Some("winit") => return Backend::Winit,
                Some("udev") | Some("drm") => return Backend::Udev,
                other => {
                    eprintln!("archon-comp: unknown backend {other:?}, defaulting to winit");
                    return Backend::Winit;
                }
            }
        }
    }
    // Default: nested winit if a Wayland/X display is present, else udev.
    if std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some() {
        Backend::Winit
    } else {
        Backend::Udev
    }
}

#[cfg(feature = "smithay-backend")]
fn run(backend: Backend, config: archon_config::Config) -> ExitCode {
    let result = match backend {
        Backend::Winit => archon_comp::runtime::run_winit(config),
        Backend::Udev => archon_comp::runtime::run_udev(config),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("compositor exited with error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(feature = "smithay-backend"))]
fn run(_backend: Backend, config: archon_config::Config) -> ExitCode {
    // Built without the Smithay runtime (the default for fast, headless CI).
    // The window-management core is still fully functional and tested; this
    // binary just can't drive real outputs unless rebuilt with the feature.
    let core = archon_comp::CompositorCore::new(&config);
    eprintln!(
        "archon-comp was built without the `smithay-backend` feature, so it \n\
         cannot drive a display. The window-management core initialised \n\
         successfully ({} workspaces, accent {}).\n\n\
         Rebuild with the runtime to run the desktop:\n\
         \tcargo build -p archon-comp --features smithay-backend --release",
        core.workspaces.count(),
        core.theme.accent.to_hex(),
    );
    ExitCode::SUCCESS
}
