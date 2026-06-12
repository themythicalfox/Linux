# FoxLinux

A custom Linux distribution for PC gaming. Debian 13 ("trixie") base with
KDE Plasma, Steam, Wine, NVIDIA drivers, GameMode, and MangoHud preinstalled,
shipped as a hybrid BIOS/UEFI live ISO with a graphical installer (Calamares).

Windows apps run through Wine; Windows-only Steam games run through Proton,
which the Steam client downloads on first launch.

## Getting the ISO

**From GitHub Actions:** every push to `main` (or a manual run of the
"Build FoxLinux ISO" workflow) builds the ISO and uploads it as the
`foxlinux-iso` artifact on the workflow run page.

**Building locally** (needs a Debian/Ubuntu host, root, ~20 GB free disk,
60–120 min):

```sh
make deps   # install live-build and friends
make iso    # produces FoxLinux-1.0-amd64.hybrid.iso
make test   # optional: headless QEMU boot test of the ISO
```

## Installing on your PC

1. Flash the ISO to a USB stick (4 GB+): [Etcher](https://etcher.balena.io/),
   [Ventoy](https://www.ventoy.net/), or `dd if=FoxLinux-*.iso of=/dev/sdX bs=4M status=progress`.
2. Boot the PC from the USB stick (works on both UEFI and legacy BIOS;
   disable Secure Boot if it refuses to boot — the NVIDIA driver is unsigned).
3. Try the live session, then run **Install Debian** (Calamares) from the
   desktop to install to disk.

## First steps after installing

- **Steam:** launch Steam from the menu — it downloads its runtime on first
  run. For Windows-only games, enable Proton: *Steam → Settings →
  Compatibility → Enable Steam Play for all other titles*.
- **Windows apps:** `wine setup.exe` (or right-click → Open with Wine).
- **Performance:** launch games with `gamemoderun %command%` and overlay
  stats with `mangohud %command%` in Steam launch options.

## Layout

- `auto/config` — live-build configuration (distro name, Debian release,
  archive areas, boot options). Change branding here and in
  `config/hooks/normal/0900-branding.hook.chroot`.
- `config/package-lists/` — packages baked into the image (desktop, NVIDIA,
  installer).
- `config/hooks/normal/0500-gaming.hook.chroot` — enables i386 multiarch and
  installs Steam/Wine/GameMode/MangoHud (must run after multiarch, so it's a
  hook rather than a package list).
- `scripts/` — host dependency setup and the QEMU boot test.
- `.github/workflows/build-iso.yml` — CI ISO build.

## Roadmap

- Custom Plasma theming, wallpaper, and "Install FoxLinux" branding in Calamares
- Lutris / Heroic Games Launcher
- Automatic NVIDIA-vs-Mesa hardware detection so one ISO fits any GPU
- Signed ISO releases attached to GitHub Releases
