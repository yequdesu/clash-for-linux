.PHONY: all build build-cli build-tui build-linux clean install help

# ============================================================
# Clash-Terminal Makefile
# ============================================================

GO_SRC = $(wildcard cmd/clashctl/*.go internal/**/*.go)
GO_BIN = target/clashctl

RUST_SRC = $(wildcard tui/src/**/*.rs)
TUI_BIN = tui/target/release/clash-tui

CLASH_BASE ?= $(HOME)/.clashctl

all: build

build: build-cli

build-cli: $(GO_SRC)
	@echo "Building clashctl..."
	cd cmd/clashctl && go build -ldflags="-s -w" -o ../../target/clashctl .
	@echo "Done: target/clashctl"

build-tui:
	@echo "Building clash-tui..."
	cd tui && cargo build --release
	@echo "Done: tui/target/release/clash-tui"

build-linux:
	@echo "Cross-compiling for Linux..."
	cd tui && bash build-linux.sh
	@echo "Done"

clean:
	rm -rf target/
	cd tui && cargo clean

install: build
	@echo "Installing to $(CLASH_BASE)/bin/..."
	@mkdir -p $(CLASH_BASE)/bin
	cp target/clashctl $(CLASH_BASE)/bin/clashctl
	chmod +x $(CLASH_BASE)/bin/clashctl
	@echo "Installed!"
	@echo ""
	@echo "To install system-wide (requires sudo):"
	@echo "  sudo cp target/clashctl /usr/local/bin/clashctl"

help:
	@echo "Clash-Terminal Build System"
	@echo ""
	@echo "  make build        - Build clashctl CLI"
	@echo "  make build-tui    - Build clash-tui TUI"
	@echo "  make build-linux  - Cross-compile TUI for Linux (x86_64-musl)"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make install      - Install to ~/.clashctl/bin/"
	@echo ""
	@echo "  bash install.sh   - Full installation script"
	@echo "  bash uninstall.sh - Full uninstallation script"
	@echo "  bash update.sh    - Update script"
