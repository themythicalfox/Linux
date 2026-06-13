# ArchonSync ISO build system. Run `make deps` once, then `make iso`.
# Building requires root (live-build uses chroots).

.PHONY: all deps config fetch iso test clean distclean

all: iso

deps:
	scripts/install-deps.sh

config:
	lb config

fetch:
	scripts/fetch-vscode.sh

# GitHub Codespaces mounts /workspaces with noexec,nodev which prevents
# debootstrap from executing binaries or creating device nodes inside chroot/.
# When that restriction is detected, a tmpfs is mounted over chroot/ so
# live-build gets a proper filesystem while everything else stays in place.
iso: config fetch
	@if findmnt --noheadings -o OPTIONS --target . 2>/dev/null | grep -q noexec; then \
	    echo "==> noexec filesystem detected — mounting tmpfs over chroot/"; \
	    mkdir -p chroot; \
	    mount -t tmpfs -o exec,dev tmpfs chroot; \
	fi
	lb build

test:
	scripts/test-iso.sh

clean:
	@if mountpoint -q chroot 2>/dev/null; then \
	    echo "==> Unmounting tmpfs from chroot/"; \
	    umount chroot 2>/dev/null || true; \
	fi
	lb clean

distclean:
	@if mountpoint -q chroot 2>/dev/null; then \
	    echo "==> Unmounting tmpfs from chroot/"; \
	    umount chroot 2>/dev/null || true; \
	fi
	lb clean --purge
	rm -rf .build local-package-lists
