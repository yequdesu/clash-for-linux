# Linux CLI&TUI Clash

Terminal-based Clash/Mihomo proxy management tool for Ubuntu Linux.
Provides CLI commands (`clashctl`) and optional TUI dashboard (`clash-tui`).

## Quick Start

```bash
# Install
bash install.sh --with-tui

# Add a subscription
clashctl sub add https://your-subscription-url

# Start proxy
clashctl start

# Load proxy env in current shell
eval $(clashctl env)

# Launch TUI dashboard
clashctl tui
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `clashctl start` | Start proxy kernel |
| `clashctl stop` | Stop proxy kernel |
| `clashctl restart` | Restart proxy kernel |
| `clashctl status` | Show running status |
| `clashctl log` | View kernel logs |
| `clashctl proxy on/off` | Manage system proxy |
| `clashctl tun on/off` | Toggle TUN mode |
| `clashctl config edit/view` | Manage config |
| `clashctl sub add/list/remove/use/update` | Subscription management |
| `clashctl node list/switch/delay` | Proxy node management |
| `clashctl test [url]` | Test proxy connectivity |
| `clashctl env` | Print proxy env vars |
| `clashctl tui` | Launch TUI dashboard |
| `clashctl upgrade-kernel` | Upgrade Mihomo kernel |

## TUI Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `1-5` | Switch tabs directly |
| `Tab/←→` | Switch tabs |
| `j/k/↑↓` | Navigate lists |
| `Enter` | Test delay / switch |
| `s` | Switch proxy |
| `d` | Test delay |
| `c/C` | Close connection(s) |
| `p` | Pause/resume logs |
| `r` | Refresh data |
| `q/Esc` | Quit |
| `Ctrl+Arrows` | Move window |
| `=/-/0` | Zoom in/out/reset |
| `?` | Help |

## Uninstall

```bash
bash uninstall.sh
```

## License

GPL-3.0
