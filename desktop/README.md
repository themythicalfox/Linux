# ArchonSync Desktop

A Wayland-native desktop environment for the [ArchonSync](../README.md) OS,
written in Rust on [Smithay](https://github.com/Smithay/smithay). It is the
gaming-and-creative desktop ArchonSync boots into: a low-latency compositor, an
orange edge-swipe lock screen, an edge-activated dock with a radial control
wheel, a modular widget desktop, and a color-science theming engine that adapts
the UI to your wallpaper.

This is the **foundation**: a buildable, tested core with a real (nested)
compositor runtime. Some surfaces are logic-complete and tested but not yet
wired to their own GPU windows — see [What runs today](#what-runs-today).

## Architecture

A Cargo workspace of focused crates. The compositor is the privileged process;
the shell (dock/wheel/widgets) are Wayland layer-shell clients; the lock screen
is drawn inside the compositor so it can't be bypassed. A shared GPU toolkit
gives every surface the same animation pipeline.

| Crate | Role | Status |
|-------|------|--------|
| `archon-theme` | Color-science engine: k-means wallpaper palette extraction, WCAG-contrast/harmony rules, the `Theme` every surface reads. Seeded from `ArchonSyncDark`. | ✅ done, tested |
| `archon-config` | Typed TOML config + keybinding parser, under `~/.config/archonsync`. | ✅ done, tested |
| `archon-ipc` | Length-prefixed bincode protocol over a unix socket. | ✅ done, tested |
| `archon-gaming` | Launcher detection, per-game perf profiles → launch env, Game Mode. | ✅ done, tested |
| `archon-widgets` | Desktop widget model: placement, rotation-aware hit testing, z-order. | ✅ core done, tested |
| `archon-ui` | Animation (easing + springs), particles, a backend-agnostic `DrawList`, and a wgpu renderer (`gpu` feature). | ✅ core + renderer |
| `archon-lock` | Orange edge-swipe unlock state machine, particles, PAM fallback, scene. | ✅ logic done, tested |
| `archon-shell` | Edge dock (spring reveal) + radial control wheel geometry; `archon-shell` binary. | ✅ logic done, tested |
| `archon-comp` | The compositor: a tested window-management core + a Smithay winit runtime (`smithay-backend`). | ✅ core + winit runtime |
| `archonctl` | CLI to drive the compositor over IPC. | ✅ done |

The design choice throughout: **pure, headless-testable logic** (tiling math,
springs, palette extraction, swipe gesture, wheel geometry) is separated from
the GPU/Wayland glue, so the *feel* and *behaviour* of the desktop are pinned by
unit tests (~120 of them) rather than eyeballed.

## What runs today

- **Compositor (nested):** `archon-comp --backend winit` is a real Smithay
  compositor — run it inside an existing Wayland/X session and it hosts xdg-shell
  clients, renders them, and tracks them in its workspace/tiling core.
- **Window management core:** tiling (master-stack/columns/grid), edge/corner
  snapping, 9 workspaces, keybindings, auto Game Mode on fullscreen games — all
  implemented and tested.
- **CLI:** `archonctl` speaks the IPC protocol (the compositor's IPC server is
  the next wiring step; the client and protocol are complete).
- **Lock / dock / wheel / theming:** logic, geometry and scene-building are
  complete and tested; `archon-shell --demo` prints what the shell would draw.

### Not yet wired (next iterations)

- The **bare-metal DRM/udev** compositor backend (`run_udev`) — today it points
  you at the winit backend. Until it lands, KDE Plasma stays ArchonSync's default
  login session and the ArchonSync session is selectable but nested-only.
- The shell/lock as their own **layer-shell GPU surfaces** (the rendering
  toolkit and all the logic exist; the SCTK+wgpu client loop is the remaining
  glue).
- Text shaping in the wgpu renderer (a `glyphon` pass), the widget data sources
  (sysinfo/MPRIS), and the compositor's IPC server loop.

## Build

```sh
# System deps (Debian/Ubuntu): Wayland/DRM/input/PAM dev libraries.
sudo apt install pkg-config libwayland-dev libinput-dev libudev-dev \
    libseat-dev libgbm-dev libdrm-dev libegl1-mesa-dev libgles2-mesa-dev \
    libxkbcommon-dev libpam0g-dev

# Fast, headless: the pure core + all logic tests (no GPU/Wayland needed).
cargo test --workspace

# The compositor runtime + GPU shell + PAM auth.
cargo build --release \
    -p archon-comp -p archon-shell -p archonctl \
    --features "archon-ui/gpu archon-comp/smithay-backend archon-lock/pam-auth"
```

## Run (nested, for testing)

From inside your current desktop session:

```sh
# Start the ArchonSync compositor in a window.
./target/release/archon-comp --backend winit
# A new wayland-N socket is advertised; the launched terminal (foot) appears.

# In another shell, inspect the shell surfaces it would draw:
./target/release/archon-shell --demo
```

See [TEST.md](TEST.md) for the full manual checklist.

## Configuration

`~/.config/archonsync/config.toml` (auto-defaulted). Keybindings, the dock edge,
swipe-to-unlock, accent mode (`signature`/`harmonize`/`contrast`), and gaming
toggles all live here. The default accent is the ArchonSync orange `#ff7a1a`.

## Packaging into the ISO

`scripts/build-desktop.sh` (run by `make desktop`, and by `make iso`) compiles
the release binaries on the host and stages them — plus the session launcher and
assets — into `config/includes.chroot`, exactly like the VS Code `.deb` is
staged. The compositor's runtime libraries are in
`config/package-lists/archonsync-desktop.list.chroot`.

## License

GPL-3.0-or-later.
