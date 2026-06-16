# ArchonSync

A custom Linux OS for gaming and creation, built for speed and designed to
look like nothing else. Debian 13 base, KDE Plasma desktop with a dark
minimal theme (near-black surfaces, fox-orange accent), and the **Wheel** —
a dot on the edge of the screen that expands into a scrollable radial
launcher when you hover it.

Ships as a hybrid BIOS/UEFI live ISO with a graphical installer.

## What's inside

**Gaming**
- Steam (32-bit libraries + Proton) for your whole Steam library
- Minecraft via Prism Launcher; Discord, both installed on first boot
- Wine 10 + Winetricks + Bottles + Lutris for non-Steam Windows games/apps
- gamescope (HDR + FSR upscaling), GameMode, MangoHud, GOverlay
- **Runs on any GPU** — NVIDIA proprietary *and* Mesa for AMD/Intel ship in one
  ISO; a first-boot detector picks the right one automatically
- **League of Legends / Valorant** use Riot's kernel anti-cheat (Vanguard),
  which blocks Linux entirely — a one-click "Play on Windows" shortcut reboots
  you into Windows for those, then back (see [GAMING.md](GAMING.md))

**Creation**
- Blender
- Visual Studio Code
- Unreal Engine: one-click guided installer (`archonsync-unreal`) with the
  full C++ toolchain (clang, lld, cmake, ninja) already baked in — Epic's
  license doesn't allow shipping the engine itself in an ISO
- Git, build-essential, Vulkan tools

**Design & desktop**
- ArchonSync Dark color scheme, `#ff7a1a` accent, Papirus icons, dark SDDM
  login and themed lock screen — on KDE Plasma 6 (Wayland, for HDR)
- **Switchable layouts**, one click in the Control Center:
  - **Side dial** — the Wheel (default): hover the orange dot on the left edge,
    scroll to rotate, click to launch
  - **Bottom taskbar** — Windows-style, familiar for friends switching over
  - **Top bar** — macOS-style slim menu bar + floating dock
- **ArchonSync Control Center** gathers everything in one place — Settings,
  Task Manager (live CPU/GPU/RAM graphs), Hardware info, Terminal, network,
  Game Mode, ad-block toggle, layout switch — the mature KDE apps, rebranded
- A first-login welcome orients anyone coming from Windows

**Performance & network**
- High-RAM workstation tuning (swappiness, `vm.max_map_count`, inotify limits)
- **BBR + fair-queue + CAKE**: high download speeds *and* low latency — a big
  download won't spike your ping. **QoS prioritises real-time traffic** (games,
  Discord voice, streaming) so your foreground "steals" the connection from
  background transfers
- **Game Mode** (one toggle): performance CPU governor, network priority bump,
  compositor effects off — maximum frames, minimum lag
- **HDR** via KDE on Wayland; per-game HDR with `archonsync-game-launch`

**Windows Mode** (dual-boot)
- "Windows Mode" on the Wheel reboots straight into your installed
  Windows — one click, no boot-menu fiddling; next boot returns to
  ArchonSync (GRUB one-shot via os-prober + `grub-reboot`)
- Your Windows drive is auto-mounted at `/windows`, with `~/Windows`
  and `~/Windows Files` (your Windows user profile) linked in your home
  folder — same files on both systems
- Account-based app sync does the rest across both OSes: Steam cloud
  saves, VS Code Settings Sync, Firefox Sync
- Setup warns you if Windows Fast Startup or BitLocker is blocking
  drive access (with fix instructions). For full read-write access,
  turn off Fast Startup in Windows once.

**Security (maximum lockdown)**

ArchonSync ships locked down by default. Every layer below is on out of the
box — nothing to configure — and tuned so it never costs you FPS or breaks
Steam/Proton.

- **Firewall:** `ufw` denies *all* incoming connections, allows outgoing, drops
  invalid/spoofed/source-routed traffic, and runs the machine in "stealth" mode
  (no replies to port scans).
- **Encrypted, malware-blocking DNS:** every lookup goes out over DNS-over-TLS
  (so your network/ISP can't see or tamper with it) to Quad9, whose resolver
  *blocks known malware, phishing and command-and-control domains* before you
  ever connect. DNSSEC rejects forged answers.
- **Tracker / ad / telemetry blocking, system-wide:** a curated blocklist
  (StevenBlack + abuse.ch URLhaus, tens of thousands of domains) is merged into
  the system resolver so **every** app and game — not just the browser — is cut
  off from ad networks, analytics and telemetry. It refreshes weekly. A seed
  list blocks the worst offenders from the very first boot.
- **Hardened Firefox:** strict tracking/fingerprint/crypto-miner protection,
  Global Privacy Control, encrypted DNS, and all telemetry/studies/Pocket off.
- **Antivirus + rootkit detection:** ClamAV runs an on-access daemon plus a
  weekly deep scan that quarantines anything infected and pops a desktop alert;
  `rkhunter` checks for rootkits. Findings are logged to
  `/var/log/archonsync-av.log`.
- **Kernel hardening:** ~40 sysctl tweaks (kernel-pointer hiding, `ptrace`
  scope 2 so malware can't read other processes' memory, full ASLR, eBPF
  lockdown, SysRq off) plus KSPP boot flags (`slab_nomerge`, heap zeroing,
  kernel-stack randomisation, IOMMU/DMA-attack protection). Rare/legacy kernel
  modules (obscure network protocols, exotic filesystems, FireWire DMA) are
  blocked entirely.
- **Mandatory access control + app sandboxing:** AppArmor in *enforce* mode on
  every profile, plus Firejail for sandboxing browsers and untrusted apps.
- **BadUSB protection:** USBGuard blocks malicious USB devices (e.g. a thumb
  drive pretending to be a keyboard to type commands). Your real devices are
  allow-listed automatically on first boot; approve a new one with
  `archonsync-usb allow <id>`.
- **Login hardening:** strong-password policy (`pwquality`), owner-only `umask`,
  core dumps disabled, the root account locked (you use `sudo`). A brute-force
  lockout module is shipped ready to switch on (`sudo pam-auth-update`).
- **Audit trail:** `auditd` records tampering with accounts, the security
  config, kernel-module loads and privilege escalation.
- **Automatic security updates:** `unattended-upgrades` applies Debian security
  fixes daily and reboots at 4am only if a fix requires it. Repositories must be
  signed; unauthenticated packages are refused.

> No system is unhackable, and anyone with **physical access and your disk
> unencrypted** can still get in — so when you install to disk, **tick "Encrypt
> the system" (LUKS full-disk encryption)** in the installer for the last mile.
> The lockdown above makes *remote* compromise, drive-by malware and tracking
> extremely hard.

See [SECURITY.md](SECURITY.md) for the full layer-by-layer breakdown and how to
verify each one.

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

## Running ArchonSync in a virtual machine (try it safely first)

A VM is the no-risk way to try ArchonSync — it can't touch your real OS or
files. Allocate at least **4 CPU cores, 8 GB RAM, 40 GB disk**, and enable
**EFI/UEFI** firmware. (3D-accelerated gaming inside a VM is limited; use a VM
to explore the desktop, then install on real hardware for games.)

### VMware Fusion (macOS, incl. Intel Macs)

1. Download `ArchonSync-1.0-amd64.hybrid.iso` (from the GitHub Actions
   `archonsync-iso` artifact, or built locally).
2. **File → New…** → *Install from disc or image* → drag in the `.iso` →
   **Continue**.
3. *Choose Operating System* → **Linux → Debian 12.x 64-bit** (closest match) →
   **Continue**.
4. Click **Customize Settings**, save the VM, then in its settings:
   - **Processors & Memory:** 4+ cores, 8192 MB+.
   - **Hard Disk:** 40 GB+.
   - **Advanced / Firmware type:** **UEFI** (not Legacy BIOS). Leave
     *Secure Boot* **off**.
5. **Start up** the VM. At the ArchonSync boot menu pick **Live** to try it, or
   the installer to install into the VM's disk.

### VirtualBox (Windows / Linux / macOS)

1. **New** → Name "ArchonSync", Type **Linux**, Version **Debian (64-bit)**.
2. Memory 8192 MB+, create a 40 GB+ VDI disk, CPUs 4+.
3. **Settings → System → Enable EFI**; **Display → Video Memory 128 MB**.
4. **Settings → Storage** → click the optical drive → choose the `.iso`.
5. **Start**. Pick **Live** or run the installer.

### QEMU/KVM (Linux, fastest)

```sh
qemu-img create -f qcow2 archonsync.qcow2 40G
qemu-system-x86_64 -enable-kvm -m 8192 -smp 4 \
  -bios /usr/share/OVMF/OVMF_CODE.fd \
  -drive file=archonsync.qcow2,if=virtio \
  -cdrom ArchonSync-1.0-amd64.hybrid.iso \
  -vga virtio -display gtk
```

(`make test` also boots the ISO headlessly in QEMU as a smoke test.)

## Installing on your actual computer

> ⚠️ This installs to a real disk. **Back up anything important first.** If you
> want to keep Windows for dual-boot ("Windows Mode"), install ArchonSync to a
> *separate* drive or to free unallocated space, and don't wipe the Windows
> disk.

1. **Write the ISO to a USB stick (8 GB+).**
   - **balenaEtcher** (Windows/macOS/Linux, easiest):
     [etcher.balena.io](https://etcher.balena.io/) → select ISO → select USB →
     Flash.
   - **Ventoy** (lets you keep several ISOs on one stick):
     [ventoy.net](https://www.ventoy.net/).
   - **Command line (Linux/macOS)** — *double-check the device name or you can
     wipe the wrong disk:*
     ```sh
     sudo dd if=ArchonSync-1.0-amd64.hybrid.iso of=/dev/sdX bs=4M status=progress conv=fsync
     ```
2. **Boot from the USB stick.** Reboot and open the firmware boot menu (usually
   **F12**, **F2**, **Esc**, or **Del** at power-on — it varies by maker) and
   pick the USB device.
3. **In the firmware (BIOS/UEFI) settings, disable Secure Boot.** The NVIDIA
   driver module is unsigned and won't load with Secure Boot on. (Leave UEFI
   mode itself on.)
4. **Try the live session first.** Pick **Live** at the boot menu to make sure
   Wi-Fi, display and sound work before committing.
5. **Run the installer** (from the desktop / app menu). When it asks about
   disks:
   - **Tick "Encrypt the system" / LUKS full-disk encryption.** This is the one
     setting that makes a stolen or lost machine actually safe — without it,
     anyone with physical access can read your files.
   - Set a **strong password** (the policy requires 12+ chars, mixed types).
   - For dual-boot, choose *Install alongside* or manual partitioning on a
     separate drive; ArchonSync's Windows Mode wires up the one-click reboot
     afterwards.
6. **First boot:** ArchonSync finishes its security setup automatically (USB
   allow-list, AppArmor enforce, first signature + blocklist download). Give it
   a minute and a network connection. Done — you're locked down.

### Verify the lockdown (optional)

```sh
sudo ufw status verbose          # firewall: deny incoming, stealth
resolvectl status                # DNS: +DNSOverTLS, Quad9
grep -c '0.0.0.0' /etc/hosts     # blocked tracker/malware domains (thousands)
sudo aa-status                   # AppArmor profiles in enforce mode
systemctl is-active clamav-daemon auditd usbguard
```

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
