# ArchonSync

A custom Linux OS for gaming and creation, built for speed and designed to
look like nothing else. Debian 13 base, KDE Plasma desktop with a dark
minimal theme (near-black surfaces, fox-orange accent), and the **Wheel** —
a dot on the edge of the screen that expands into a scrollable radial
launcher when you hover it.

Ships as a hybrid BIOS/UEFI live ISO with a graphical installer.

## What's inside

**Gaming**
- Steam (with 32-bit libraries and NVIDIA 32-bit GL/Vulkan for Proton)
- Wine 10 (64-bit + 32-bit) for Windows apps
- GameMode and MangoHud
- NVIDIA proprietary driver, prebuilt for the shipped kernel

**Creation**
- Blender
- Visual Studio Code
- Unreal Engine: one-click guided installer (`archonsync-unreal`) with the
  full C++ toolchain (clang, lld, cmake, ninja) already baked in — Epic's
  license doesn't allow shipping the engine itself in an ISO
- Git, build-essential, Vulkan tools

**Design**
- ArchonSync Dark color scheme and wallpaper, dark SDDM login, themed
  lock screen
- The Wheel launcher (custom Plasma widget): hover the orange dot on the
  left edge, scroll to rotate through apps/commands, click to launch.
  Edit the entries in the widget settings (JSON list of name/icon/cmd).
- Minimal top bar with clock and system tray — no taskbar clutter

**Performance**
- Tuned for a high-RAM workstation: low swappiness, SteamOS-grade
  `vm.max_map_count`, high inotify limits for IDEs/engines
- power-profiles-daemon for one-click performance mode

## Getting the ISO

**From GitHub Actions:** every push to `main` (or a manual run of the
"Build ArchonSync ISO" workflow) uploads the ISO as the `archonsync-iso`
artifact on the workflow run page.

**Building locally** (Debian/Ubuntu host, root, ~25 GB free disk):

```sh
make deps   # install live-build and friends
make iso    # produces ArchonSync-1.0-amd64.hybrid.iso
make test   # optional: headless QEMU boot test
```

## Installing on your PC

1. Flash the ISO to a USB stick (8 GB+): [Etcher](https://etcher.balena.io/),
   [Ventoy](https://www.ventoy.net/), or `dd if=ArchonSync-*.iso of=/dev/sdX bs=4M status=progress`.
2. Boot from the USB stick. **Disable Secure Boot** — the NVIDIA module is
   unsigned.
3. Try the live session, then run the installer from the menu to install
   to disk.

## First steps after installing

- **Steam:** first launch downloads its runtime. For Windows-only games
  enable Proton: *Steam → Settings → Compatibility → Enable Steam Play
  for all other titles*.
- **Unreal Engine:** open "Unreal Engine" from the Wheel or app menu — it
  walks you through pulling the native Linux build with your Epic account.
- **Windows apps:** `wine setup.exe` or right-click → Open with Wine.
- **Game performance:** add `gamemoderun %command%` (and `mangohud %command%`
  for an FPS overlay) to a game's Steam launch options.

## Repo layout

- `auto/config` — live-build configuration (name, Debian release, boot options)
- `config/package-lists/` — what gets baked into the image
- `config/hooks/normal/` — build hooks: gaming stack (needs i386 multiarch
  first), VS Code (not in Debian), branding/Calamares rebrand
- `config/includes.chroot/` — files shipped verbatim: theme, wallpaper,
  Wheel widget, skel desktop layout, sysctl tuning, Unreal helper
- `scripts/` — host dependency setup and the QEMU boot test
- `.github/workflows/build-iso.yml` — CI ISO build

## Roadmap

- Plymouth boot splash and themed GRUB menu
- Lutris / Heroic Games Launcher
- NVIDIA-vs-Mesa hardware auto-detection so one ISO fits any GPU
- Signed release ISOs attached to GitHub Releases
