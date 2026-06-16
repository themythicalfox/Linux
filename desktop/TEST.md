# ArchonSync Desktop — manual test checklist

The pure logic is covered by `cargo test --workspace` (run in CI, no display
needed). This checklist covers the parts that need a real GPU/display and so
can only be verified on your own machine.

## 0. Automated (anywhere, including CI)

```sh
cd desktop
cargo test --workspace            # ~120 unit tests across all crates
cargo clippy --workspace          # no warnings
cargo build -p archon-ui --features gpu          # wgpu renderer compiles
cargo build -p archon-comp --features smithay-backend   # compositor compiles
```

Expected: all tests pass, clippy clean, both feature builds succeed.

## 1. Compositor (nested winit)

Run from inside an existing Wayland or X session:

```sh
cargo run -p archon-comp --features smithay-backend -- --backend winit
```

- [ ] A compositor window opens with the near-black ArchonSync background.
- [ ] A `foot` terminal (or your configured terminal) launches inside it.
- [ ] Logs print the advertised `wayland-N` socket name.
- [ ] Launching another client into that socket
      (`WAYLAND_DISPLAY=wayland-N foot`) shows it in the compositor.
- [ ] Closing the window exits cleanly (exit code 0).

## 2. Shell surfaces (demo)

```sh
cargo run -p archon-shell -- --demo
```

- [ ] Prints accent `#ff7a1a` (or a wallpaper-derived accent).
- [ ] Reports the dock edge/rect and 7 wheel items, with "Power" selected.
- [ ] `wheel draws : N primitives` is ≥ 9 (ring + dots + hub label).

## 3. CLI

```sh
cargo run -p archonctl -- state      # with no compositor running
```

- [ ] Reports it can't reach the compositor socket (graceful error, no panic).
- [ ] `archonctl --help` prints the command list.

## 4. Theming from a wallpaper

```sh
# Point config at any image and confirm a coherent dark theme is derived.
cargo run -p archon-shell -- --demo   # uses config.general.wallpaper
```

- [ ] Accent adapts to a colorful wallpaper (harmonize mode) but surfaces stay
      near-black and text stays readable.
- [ ] A greyscale wallpaper falls back to the signature orange.

## 5. ISO integration (on a build host with Rust + the dev libs)

```sh
make desktop      # stages binaries/assets into config/includes.chroot
```

- [ ] `config/includes.chroot/usr/local/bin/` contains `archon-comp`,
      `archon-shell`, `archonctl`, `archonsync-session`.
- [ ] `config/includes.chroot/usr/share/wayland-sessions/archonsync.desktop`
      exists.
- [ ] A subsequent `make iso` still builds successfully.
- [ ] Booting the ISO shows "ArchonSync" as a selectable session in SDDM
      (KDE Plasma remains the default until the DRM backend lands).
