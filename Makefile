# ArchonSync ISO build system. Run `make deps` once, then `make iso`.
# Building requires root (live-build uses chroots).
#
# GitHub Codespaces: the /workspaces filesystem is mounted noexec/nodev,
# which prevents debootstrap from running. `make iso` detects this
# automatically and falls through to `make docker-iso`, which builds
# inside a privileged Debian container where those restrictions don't apply.

.PHONY: all deps config fetch desktop iso docker-iso test clean distclean

all: iso

deps:
	scripts/install-deps.sh

config:
	lb config

fetch:
	scripts/fetch-vscode.sh

# Build the ArchonSync Rust desktop and stage its binaries/assets into the
# chroot tree. Separate target so it can be run (and debugged) on its own.
desktop:
	scripts/build-desktop.sh

iso: fetch desktop
	@if [ "$$CODESPACES" = "true" ]; then \
	    echo "==> GitHub Codespaces detected — routing to docker-iso to bypass noexec."; \
	    $(MAKE) docker-iso; \
	else \
	    lb config; \
	    lb build; \
	fi

docker-iso: fetch desktop
	scripts/build-docker.sh

test:
	scripts/test-iso.sh

clean:
	lb clean

distclean:
	lb clean --purge
	rm -rf .build local-package-lists
