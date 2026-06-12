#!/bin/sh
# Headless QEMU boot test for the FoxLinux ISO.
#
# Extracts the live kernel/initrd from the ISO and boots them directly with
# console=ttyS0 so boot progress is visible on the serial console even though
# the live image itself targets a graphical session. Passes when systemd
# reaches the graphical target (or a login prompt) before the timeout.
#
# Software emulation (no KVM) is slow; allow ~30 minutes.

set -e

ISO="${1:-$(ls -1 *.iso 2>/dev/null | head -1)}"
TIMEOUT="${TIMEOUT:-1800}"
LOG="${LOG:-boot-test.log}"

if [ -z "$ISO" ] || [ ! -f "$ISO" ]; then
    echo "usage: $0 <image.iso>  (no ISO found in current directory)" >&2
    exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Pull the kernel and initrd out of the ISO without needing loop mounts.
bsdtar -C "$WORK" -xf "$ISO" live
KERNEL="$(ls "$WORK"/live/vmlinuz* | head -1)"
INITRD="$(ls "$WORK"/live/initrd* | head -1)"

echo "Booting $ISO (timeout ${TIMEOUT}s), serial log: $LOG"
timeout "$TIMEOUT" qemu-system-x86_64 \
    -m 4096 -smp "$(nproc)" \
    -kernel "$KERNEL" \
    -initrd "$INITRD" \
    -append "boot=live components console=ttyS0 systemd.show_status=1" \
    -cdrom "$ISO" \
    -display none -serial file:"$LOG" -no-reboot &
QEMU_PID=$!

while kill -0 "$QEMU_PID" 2>/dev/null; do
    if grep -qE "Reached target.*(Graphical|Multi-User)|foxlinux login:" "$LOG" 2>/dev/null; then
        echo "PASS: live system booted."
        kill "$QEMU_PID" 2>/dev/null || true
        wait "$QEMU_PID" 2>/dev/null || true
        exit 0
    fi
    sleep 10
done

echo "FAIL: boot did not reach a usable target within ${TIMEOUT}s. See $LOG" >&2
exit 1
