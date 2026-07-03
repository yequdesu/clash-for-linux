# Clash for Linux

Linux-first Clash/Mihomo management toolkit with a CLI control plane (`clashctl`), a terminal TUI (`clash-tui`), and install/update/uninstall scripts.

This repository is currently in a hardening phase. The codebase has gained atomic config writes, subscription rollback paths, safer service management, CLI exit-code checks, CI/smoke entry points, and audit documentation. It should not be treated as a stable release until the external validation checklist has passed on real Linux/systemd, release artifacts, TUN, desktop proxy, and real Mihomo/geodata downloads.

## Scope

This project manages Mihomo. It does not implement a proxy kernel and does not fork Mihomo.

Components:

- `clashctl`: Linux CLI for install-time and runtime control.
- `clash-tui`: Rust terminal UI for status, nodes, connections, logs, and subscription actions.
- `install.sh`, `update.sh`, `uninstall.sh`: installer and lifecycle scripts.
- `scripts/smoke`: local and CI smoke checks.

The TUI is a terminal application, not a Tauri desktop GUI.

## Current Status

Read these first:

- [Hardening specification](docs/PROJECT_HARDENING_SPEC.md)
- [Hardening status matrix](docs/HARDENING_STATUS.md)
- [Audit report](docs/PROJECT_AUDIT_REPORT.md)
- [External validation checklist](docs/EXTERNAL_VALIDATION_CHECKLIST.md)
- [Install smoke validation](docs/INSTALL_SMOKE.md)

Local checks currently cover Go tests, Rust tests, shell syntax, smoke scripts, CLI process exit codes, config rollback, subscription consistency, geodata staging, kernel upgrade rollback, and nohup pid safety.

Still required before a stable release:

- Real Linux VM/systemd install/start/status/log/stop/uninstall.
- GitHub Actions and release workflow evidence.
- Release artifact + `SHA256SUMS` install.
- Real Mihomo, yq, geodata download and startup.
- Real TUN behavior.
- Real GNOME/KDE desktop proxy behavior.

## Local Verification

```bash
go test ./...
go vet ./...
GOOS=linux GOARCH=amd64 go build ./cmd/clashctl

cd tui
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Shell checks:

```bash
bash -n install.sh update.sh uninstall.sh install_tui.sh scripts/preflight.sh scripts/init/nohup.sh scripts/init/systemd.sh scripts/smoke/*.sh
bash scripts/smoke/static_safety.sh
bash scripts/smoke/nohup_pid_safety.sh
```

On Linux with Go installed:

```bash
bash scripts/smoke/cli_exit_codes.sh
```

## Install

For development or validation only:

```bash
bash install.sh --with-tui
```

For a release candidate, install from the explicit tag instead of `latest`:

```bash
curl -fsSLO https://raw.githubusercontent.com/yequdesu/clash-for-linux/v0.2.0-rc.4/install.sh
chmod +x install.sh
CLASHCTL_RELEASE_BASE_URL="https://github.com/yequdesu/clash-for-linux/releases/download/v0.2.0-rc.4" bash install.sh --with-tui
```

Common flow:

```bash
clashctl sub add https://your-subscription-url
clashctl start
clashctl status
clashctl doctor
eval "$(clashctl env)"
clashctl tui
```

Uninstall:

```bash
bash uninstall.sh
```

For release-grade verification, follow [docs/EXTERNAL_VALIDATION_CHECKLIST.md](docs/EXTERNAL_VALIDATION_CHECKLIST.md).

## License

GPL-3.0
