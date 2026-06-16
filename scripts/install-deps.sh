#!/bin/sh
# Install host packages needed to build (and boot-test) the FoxLinux ISO.
#
# On Ubuntu hosts two packages must come from Debian instead:
#   - live-build: Ubuntu ships an ancient fork (3.0~a57) that doesn't support
#     the modern Debian config layout.
#   - debian-archive-keyring: Ubuntu's copy lacks the trixie signing keys,
#     so debootstrap can't verify the Release file.

set -e

DEBIAN_MIRROR="${DEBIAN_MIRROR:-http://deb.debian.org/debian}"
LIVE_BUILD_DEB="pool/main/l/live-build/live-build_20250505+deb13u1_all.deb"
KEYRING_DEB="pool/main/d/debian-archive-keyring/debian-archive-keyring_2025.1_all.deb"

SUDO=""
[ "$(id -u)" -ne 0 ] && SUDO="sudo"

$SUDO apt-get update
$SUDO apt-get install -y \
    debootstrap \
    squashfs-tools \
    xorriso \
    mtools \
    dosfstools \
    libarchive-tools \
    ca-certificates \
    curl

# Toolchain + dev libraries for building the ArchonSync Rust desktop
# (scripts/build-desktop.sh). The desktop is compiled on the host and only its
# release binaries are staged into the image, so these are host build deps. If
# they can't be installed, the ISO still ships the feature-light desktop core.
$SUDO apt-get install -y \
    cargo \
    rustc \
    pkg-config \
    libwayland-dev \
    libinput-dev \
    libudev-dev \
    libseat-dev \
    libgbm-dev \
    libdrm-dev \
    libegl1-mesa-dev \
    libgles2-mesa-dev \
    libxkbcommon-dev \
    libpam0g-dev \
    || echo "install-deps: desktop build deps unavailable; shipping feature-light desktop."

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
curl -fsSL -o "$tmpdir/live-build.deb" "$DEBIAN_MIRROR/$LIVE_BUILD_DEB"
curl -fsSL -o "$tmpdir/keyring.deb" "$DEBIAN_MIRROR/$KEYRING_DEB"
$SUDO apt-get install -y --allow-downgrades "$tmpdir/live-build.deb" "$tmpdir/keyring.deb"

echo "live-build $(lb --version) ready."
