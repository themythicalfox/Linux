#!/bin/sh
# Build the ArchonSync Rust desktop and stage it into the live-build chroot tree.
#
# The desktop is compiled on the host/CI (not inside the chroot) and the release
# binaries + assets are dropped into config/includes.chroot, exactly like
# fetch-vscode.sh stages a prebuilt artifact.
#
# IMPORTANT: the custom compositor is OPTIONAL for the ISO — KDE Plasma is the
# shipping daily-driver desktop, and the compositor has no bare-metal backend
# yet. So if Rust is missing or too old (e.g. the distro's apt cargo, which is
# older than modern crates need), this script SKIPS gracefully and lets the ISO
# build continue. The compositor is still built and tested by the dedicated
# desktop CI workflow, which uses an up-to-date toolchain.

set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WS="$ROOT/desktop"
STAGE_BIN="$ROOT/config/includes.chroot/usr/local/bin"
STAGE_SHARE="$ROOT/config/includes.chroot/usr/share/archonsync/desktop"
STAGE_SESSION="$ROOT/config/includes.chroot/usr/share/wayland-sessions"

# Minimum Cargo for the workspace's dependency tree (edition-2024 crates).
MIN_MAJOR=1
MIN_MINOR=85

skip() {
    echo "build-desktop.sh: $1" >&2
    echo "build-desktop.sh: skipping the optional ArchonSync compositor; the ISO" >&2
    echo "                  ships KDE Plasma as the desktop and builds normally." >&2
    exit 0
}

command -v cargo >/dev/null 2>&1 || skip "cargo not found"

# Parse "cargo 1.75.0 (...)" -> major/minor and require >= MIN.
ver="$(cargo --version 2>/dev/null | awk '{print $2}')"
maj="${ver%%.*}"
rest="${ver#*.}"
min="${rest%%.*}"
case "$maj$min" in
    *[!0-9]*|"") skip "could not parse cargo version ('$ver')" ;;
esac
if [ "$maj" -lt "$MIN_MAJOR" ] || { [ "$maj" -eq "$MIN_MAJOR" ] && [ "$min" -lt "$MIN_MINOR" ]; }; then
    skip "cargo $ver is too old (need >= ${MIN_MAJOR}.${MIN_MINOR})"
fi

echo "==> Building ArchonSync desktop (release, cargo $ver)"
FEATURES="archon-ui/gpu archon-comp/smithay-backend archon-lock/pam-auth"
built=""
if cargo build --release --manifest-path "$WS/Cargo.toml" \
        -p archon-comp -p archon-shell -p archonctl \
        --features "$FEATURES" 2>"$WS/.build-desktop.log"; then
    echo "    full desktop (compositor runtime + GPU shell) built"
    built=1
elif cargo build --release --manifest-path "$WS/Cargo.toml" \
        -p archon-comp -p archon-shell -p archonctl 2>>"$WS/.build-desktop.log"; then
    echo "    feature-light core + CLI built (see desktop/.build-desktop.log)"
    built=1
else
    skip "compositor build failed (see desktop/.build-desktop.log)"
fi

echo "==> Staging binaries into $STAGE_BIN"
mkdir -p "$STAGE_BIN" "$STAGE_SHARE" "$STAGE_SESSION"
staged_comp=""
for bin in archon-comp archon-shell archonctl; do
    if [ -f "$WS/target/release/$bin" ]; then
        install -m 0755 "$WS/target/release/$bin" "$STAGE_BIN/$bin"
        echo "    + $bin"
        [ "$bin" = "archon-comp" ] && staged_comp=1
    fi
done

# Only advertise the ArchonSync Wayland session if its compositor actually got
# staged (otherwise SDDM would show a session that can't start).
if [ -n "$staged_comp" ]; then
    echo "==> Staging session launcher + assets"
    install -m 0755 "$WS/session/archonsync-session" "$STAGE_BIN/archonsync-session"
    install -m 0644 "$WS/session/archonsync.desktop" "$STAGE_SESSION/archonsync.desktop"
    cp -r "$WS/assets/." "$STAGE_SHARE/"
    echo "==> ArchonSync desktop staged. The next ISO build will include it."
fi

[ -n "$built" ] || skip "nothing was built"
