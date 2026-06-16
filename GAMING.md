# ArchonSync gaming & apps cheat-sheet

Everything you need to play and run your software. Short version: **Steam games
and Minecraft run natively; League/Valorant need Windows; most other Windows
apps run via Bottles/Lutris/Wine.**

## Steam

1. Open **Steam** (Wheel / app menu). First launch downloads its runtime.
2. Turn on Proton for Windows-only games:
   *Steam → Settings → Compatibility → Enable Steam Play for all other titles.*
3. Just install and play. For the best performance/HDR, set a game's
   **launch options** (right-click game → Properties → Launch Options) to:
   ```
   archonsync-game-launch %command%
   ```
   That wraps it in gamescope (HDR + FSR upscaling), GameMode and the MangoHud
   FPS overlay.

## Minecraft

- Open **Prism Launcher** (installed on first boot). Log in with your Microsoft
  account, pick a version, play. Java is already installed.

## Discord

- Installed on first boot (Flatpak). Open it from the Wheel or app menu. Voice
  uses PipeWire and is prioritised by the network QoS for low latency.

## League of Legends / Valorant (and other Vanguard/anti-cheat games)

These **cannot run on Linux** — Riot's Vanguard anti-cheat runs in the Windows
kernel and blocks Linux and VMs. There is no Proton/Wine workaround.

- Use the **"League of Legends (Windows)"** shortcut (or `archonsync-play-on-windows`).
  It reboots straight into your Windows install to play; the next restart brings
  you back to ArchonSync automatically.
- Requires a Windows install on the machine (dual-boot). See "Windows Mode" in
  the [README](README.md).

## Other Windows games & apps

- **Epic / GOG games:** Heroic Games Launcher (installed first boot).
- **Other launchers / Windows games:** Lutris (installed first boot).
- **Windows desktop programs (.exe):** Bottles (installed first boot) gives each
  app its own tidy Windows environment. Or `wine setup.exe` for quick installs.
- **Manage Proton versions** (e.g. Proton-GE for newer fixes): ProtonUp-Qt
  (installed first boot).

## Game Mode

Toggle **Game Mode** (Control Center, the Wheel, or `archonsync-gamemode`):
- CPU governor → performance
- Network priority bumped so the game owns the connection
- Compositor effects/vsync off for lower latency

## HDR

HDR works on KDE Plasma (Wayland session, the default). Turn it on per display:
*System Settings → Display & Monitor → enable HDR.* For HDR inside a game, launch
it with `archonsync-game-launch %command%` (uses `gamescope --hdr-enabled`).

> Note: `gamescope` is installed when the Debian release provides it. If your
> ISO shipped without it (it's currently only in Debian backports), per-display
> HDR in KDE still works, and you can add gamescope later with
> `sudo apt install -t trixie-backports gamescope`. The launch wrapper detects
> gamescope automatically and just skips it if it's missing.

## GPU notes

- **NVIDIA:** proprietary driver ships in the ISO; DRM modeset is enabled
  automatically for Wayland/HDR.
- **AMD / Intel:** driven by Mesa (also in the ISO); the first-boot detector
  disables the unused NVIDIA bits so your GPU is used cleanly.
- Check yours: `archonsync-hardware` (Info Center) or `nvtop` / `btop`.

## Network tips

The connection is tuned for low latency and high throughput out of the box
(BBR + CAKE + QoS). To squeeze out the last of the bufferbloat, tell CAKE your
real line rate so it can hold ping flat even at full download:

```sh
# Run a speed test first, then set ~90-95% of the measured numbers (Mbit/s):
archonsync-netspeed 470 22        # 470 Mbit down, 22 Mbit up
archonsync-netspeed auto          # measure + set automatically (needs speedtest-cli)
archonsync-netspeed status        # show the current setting
archonsync-netspeed clear         # back to unmanaged
```

The setting persists across reboots and reconnects.

