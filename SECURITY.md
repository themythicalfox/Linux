# ArchonSync security model

ArchonSync is locked down by default. This document lists every layer, what it
defends against, where it lives in the repo, and how to verify it on a running
system. The guiding constraint: **maximum hardening that never costs gaming
performance or breaks Steam/Proton.**

## Threat model

ArchonSync defends well against:

- **Remote attacks** — port scans, network exploits, spoofed/MITM traffic.
- **Drive-by malware & viruses** — malicious downloads, infected files, rootkits.
- **Tracking & telemetry** — ad networks, analytics, fingerprinting, OS/app
  phone-home.
- **Malicious peripherals** — BadUSB / "rubber ducky" keyboard-injection USBs.
- **Local malware escalation** — a compromised app trying to read other
  processes' memory or gain root.

It cannot defend against (no OS can):

- An attacker with **physical access to an unencrypted disk** → always enable
  **LUKS full-disk encryption** at install (the installer offers it).
- You **running malware as yourself and granting it permission** → the sandbox
  and AV layers reduce but can't eliminate this.

## Layers

| # | Layer | Defends against | Config |
|---|-------|-----------------|--------|
| 1 | **ufw firewall** — deny incoming, allow outgoing, drop invalid/spoofed, stealth | Remote access, port scans | `0930-security.hook.chroot` |
| 2 | **Encrypted DNS (DoT) → Quad9** | DNS snooping/tampering, malware/C2 domains | `etc/systemd/resolved.conf.d/archonsync-dns.conf` |
| 3 | **System-wide blocklist** (StevenBlack + URLhaus) | Ads, trackers, telemetry, malware hosts | `usr/local/bin/archonsync-blocklist-update`, `etc/archonsync/` |
| 4 | **Hardened Firefox** | Browser tracking/fingerprinting, telemetry | `usr/lib/firefox-esr/distribution/policies.json` |
| 5 | **ClamAV + rkhunter** | Viruses, malware, rootkits | `usr/local/bin/archonsync-antivirus-scan` + timer |
| 6 | **Kernel sysctl hardening** (~40) | Info leaks, memory scraping, network spoofing | `etc/sysctl.d/99-archonsync-security.conf` |
| 7 | **KSPP kernel boot flags** | Heap/kernel exploitation, DMA attacks | `etc/default/grub.d/45-archonsync-hardening.cfg` |
| 8 | **Module blacklist** | Exotic-protocol/filesystem & FireWire-DMA exploits | `etc/modprobe.d/archonsync-blacklist.conf` |
| 9 | **AppArmor (enforce) + Firejail** | App compromise blast-radius | `0930-security.hook.chroot`, first-boot |
| 10 | **USBGuard** | BadUSB / malicious peripherals | `etc/usbguard/`, `usr/local/bin/archonsync-usb` |
| 11 | **Login hardening** (pwquality, umask, no core dumps, root locked) | Weak passwords, secret leakage | `etc/security/`, `etc/profile.d/` |
| 12 | **auditd** | Stealthy tampering / persistence | `etc/audit/rules.d/archonsync.rules` |
| 13 | **unattended-upgrades + signed APT** | Unpatched CVEs, tampered packages | `etc/apt/apt.conf.d/52archonsync-security` |

## Deliberate gaming-safe choices

These would raise the score on a server but hurt a gaming desktop, so they are
**off** (and documented inline):

- **`lockdown=confidentiality` / `module.sig_enforce=1`** — would refuse to load
  the unsigned NVIDIA driver. (Secure Boot is off on this image by design.)
- **`mitigations=...,nosmt`** — disabling SMT/Hyper-Threading costs significant
  FPS; default CPU mitigations stay on.
- **`init_on_free=1`** — measurable frame-time cost; only `init_on_alloc=1` (≈
  free) is enabled. Add `init_on_free=1` to flag 45 for absolute maximum.
- **Disabling unprivileged user namespaces** — would break Proton's
  pressure-vessel and Flatpak sandboxes. Left enabled.
- **faillock brute-force lockout** — shipped but off, so a fresh install can't
  lock you out; enable with `sudo pam-auth-update`.

## How it's wired (build vs. boot)

- **Build time** (`0930-security.hook.chroot`): enable services, set ufw policy,
  seed the blocklist, route DNS through systemd-resolved, enable pwquality.
- **First boot** (`archonsync-firstboot-security`): allow-list your *real* USB
  devices then arm USBGuard, set AppArmor to enforce, pull the first
  AV signatures + full blocklist, lock root. Runs once.
- **Ongoing** (timers): weekly blocklist refresh, weekly AV + rootkit scan,
  daily unattended security upgrades.

## Verify on a running system

```sh
sudo ufw status verbose                 # 1: deny incoming, stealth
resolvectl status | grep -i tls         # 2: +DNSOverTLS
grep -c '^0\.0\.0\.0' /etc/hosts        # 3: thousands of blocked domains
sudo aa-status                          # 9: profiles in enforce mode
sudo usbguard list-devices              # 10: USB allow-list
systemctl is-active clamav-daemon auditd usbguard systemd-resolved
cat /var/log/archonsync-av.log          # 5: last scan result
```

## Reporting

This is a personal/hobby distro; there's no formal disclosure process. If you
find an issue, open an issue on the repository.
