#!/bin/sh
# Download the VS Code .deb on the build host into config/packages.chroot/,
# where live-build installs local packages into the image automatically.
#
# Done on the host rather than in a chroot hook so the download works
# behind TLS-intercepting proxies the chroot's trust store doesn't know.

set -e

DEST="$(dirname "$0")/../config/packages.chroot/code.deb"

if [ -f "$DEST" ]; then
    echo "VS Code package already present: $DEST"
    exit 0
fi

echo "Fetching VS Code (stable, linux-deb-x64)..."
curl -fL -o "$DEST" "https://update.code.visualstudio.com/latest/linux-deb-x64/stable"
echo "Saved $(du -h "$DEST" | cut -f1) to $DEST"
