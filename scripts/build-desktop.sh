#!/bin/sh
# Build the ArchonSync Rust desktop and stage it into the live-build chroot tree.
#
# The desktop is compiled on the host/CI (not inside the chroot) and the release
# binaries + assets are dropped into config/includes.chroot, exactly like
# fetch-vscode.sh stages a prebuilt artifact. This keeps Rust and its huge build
# tree out of the image build, and means the ISO only ships the final binaries.
#
# Enables the GPU renderer (archon-ui "gpu"), the Smithay runtime
# (archon-comp "smithay-backend") and PAM auth (archon-lock "pam-auth") for the
# real desktop. If the workspace doesn't build with those features on this host
# (e.g. missing system libs), the script falls back to staging the
# feature-light binaries so the ISO still gets a working core + CLI.

set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WS="$ROOT/desktop"
STAGE_BIN="$ROOT/config/includes.chroot/usr/local/bin"
STAGE_SHARE="$ROOT/config/includes.chroot/usr/share/archonsync/desktop"
STAGE_SESSION="$ROOT/config/includes.chroot/usr/share/wayland-sessions"

if ! command -v cargo >/dev/null 2>&1; then
    echo "build-desktop.sh: cargo not found. Install Rust (https://rustup.rs) first." >&2
    exit 1
fi

echo "==> Building ArchonSync desktop (release)"
FEATURES="archon-ui/gpu archon-comp/smithay-backend archon-lock/pam-auth"
if cargo build --release --manifest-path "$WS/Cargo.toml" \
        -p archon-comp -p archon-shell -p archonctl \
        --features "$FEATURES" 2>"$WS/.build-desktop.log"; then
    echo "    full desktop (compositor runtime + GPU shell) built"
else
    echo "    NOTE: full-feature build failed (see desktop/.build-desktop.log)."
    echo "    Falling back to the feature-light core + CLI so the ISO still builds."
    cargo build --release --manifest-path "$WS/Cargo.toml" \
        -p archon-comp -p archon-shell -p archonctl
fi

echo "==> Staging binaries into $STAGE_BIN"
mkdir -p "$STAGE_BIN" "$STAGE_SHARE" "$STAGE_SESSION"
for bin in archon-comp archon-shell archonctl; do
    if [ -f "$WS/target/release/$bin" ]; then
        install -m 0755 "$WS/target/release/$bin" "$STAGE_BIN/$bin"
        echo "    + $bin"
    fi
done

echo "==> Staging session launcher + assets"
install -m 0755 "$WS/session/archonsync-session" "$STAGE_BIN/archonsync-session"
install -m 0644 "$WS/session/archonsync.desktop" "$STAGE_SESSION/archonsync.desktop"
cp -r "$WS/assets/." "$STAGE_SHARE/"

echo "==> ArchonSync desktop staged. The next ISO build will include it."
