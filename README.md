# Clash-Terminal

A terminal-based proxy management tool for Ubuntu Linux.

## Features

- **clashctl**: Go CLI for full proxy lifecycle management
- **clash-tui**: Rust TUI dashboard with 5 tabs
- **Bash scripts**: Install, uninstall, update scripts

## Installation

```bash
bash install.sh
```

## Usage

```bash
# Start kernel
clashctl start

# Stop kernel
clashctl stop

# Open TUI dashboard
clashctl tui

# Manage subscriptions
clashctl sub add <url>
clashctl sub list
clashctl sub use <id>
```

## Development

```bash
# Build CLI
make build

# Build TUI
make build-tui

# Run tests
make test
```

## License

GPL-3.0
