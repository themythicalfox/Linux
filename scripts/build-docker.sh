#!/bin/sh
# Build the ArchonSync ISO inside a privileged Docker container.
#
# Use this (or `make docker-iso`) when your host filesystem is mounted
# noexec/nodev — e.g. GitHub Codespaces /workspaces — which prevents
# debootstrap from executing binaries or creating device nodes inside chroot/.
#
# Requires: docker (available by default in GitHub Codespaces)

set -e

REPO="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="debian:trixie"

echo "==> Building ArchonSync ISO inside Docker (20-30 min on 4 cores)..."
echo "    Repo : $REPO"
echo "    Image: $IMAGE"
echo ""

docker run --rm --privileged \
    -v "$REPO:/output" \
    "$IMAGE" \
    sh -c '
        set -e

        echo "--- Copying source tree into /build ---"
        mkdir /build
        cp -ra /output/. /build/
        cd /build

        echo "--- Installing build dependencies ---"
        scripts/install-deps.sh

        echo "--- Fetching VS Code package ---"
        scripts/fetch-vscode.sh

        echo "--- Configuring live-build ---"
        lb config

        echo "--- Building ISO (this is the long part) ---"
        lb build

        echo "--- Copying ISO to output ---"
        if ! cp /build/ArchonSync*.iso /output/ 2>/dev/null; then
            cp /build/*.iso /output/ 2>/dev/null || \
                { echo "ERROR: no .iso found — check lb output above"; exit 1; }
        fi
    '

echo ""
echo "==> Done. ISO file:"
ls -lh "$REPO"/*.iso 2>/dev/null || echo "(not found in $REPO)"
