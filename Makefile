# FoxLinux ISO build system. Run `make deps` once, then `make iso`.
# Building requires root (live-build uses chroots).

.PHONY: all deps config fetch iso test clean distclean

all: iso

deps:
	scripts/install-deps.sh

config:
	lb config

fetch:
	scripts/fetch-vscode.sh

iso: config fetch
	lb build

test:
	scripts/test-iso.sh

clean:
	lb clean

distclean:
	lb clean --purge
	rm -rf .build local-package-lists
