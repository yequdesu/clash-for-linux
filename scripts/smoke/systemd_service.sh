#!/usr/bin/env bash
# systemd service smoke test.
#
# Installs clashctl with a fake Mihomo binary that validates configs and listens
# on the configured API/proxy ports, then exercises systemctl + clashctl lifecycle.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLASH_BASE_DIR="${CLASH_BASE_DIR:-/tmp/clashctl-systemd-smoke}"
export CLASH_BASE_DIR
export KERNEL_NAME="${KERNEL_NAME:-mihomo}"
export SERVICE_NAME="${SERVICE_NAME:-clashctl}"
export INIT_TYPE=systemd
export CLASH_INIT_TYPE=systemd
export CLASHCTL_SKIP_RELEASE=true
export VERSION_GEODATA="${VERSION_GEODATA:-smoke}"

log() { printf '[systemd-smoke] %s\n' "$*"; }
fail() { printf '[systemd-smoke] ERROR: %s\n' "$*" >&2; exit 1; }

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || fail "missing command: $1"
}

sudo_cmd() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    else
        sudo "$@"
    fi
}

cleanup() {
    sudo_cmd systemctl stop "$SERVICE_NAME" >/dev/null 2>&1 || true
    sudo_cmd systemctl disable "$SERVICE_NAME" >/dev/null 2>&1 || true
    sudo_cmd rm -f "/etc/systemd/system/${SERVICE_NAME}.service"
    sudo_cmd systemctl daemon-reload >/dev/null 2>&1 || true
    sudo_cmd rm -f /usr/local/bin/clashctl /usr/local/bin/"$KERNEL_NAME" /usr/local/bin/yq
    rm -rf "$CLASH_BASE_DIR"
}
trap cleanup EXIT

seed_fake_kernel() {
    install -d "$CLASH_BASE_DIR/bin"
    cat > "$CLASH_BASE_DIR/bin/${KERNEL_NAME}" <<'KERNEL'
#!/usr/bin/env bash
set -euo pipefail
config=""
for arg in "$@"; do
    if [ "$arg" = "-t" ]; then
        exit 0
    fi
done
while [ "$#" -gt 0 ]; do
    case "$1" in
        -f)
            config="${2:-}"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done
exec python3 - "$config" <<'PY'
import http.server
import json
import re
import signal
import socket
import socketserver
import sys
import threading
import time

config_path = sys.argv[1] if len(sys.argv) > 1 else ""
try:
    text = open(config_path, "r", encoding="utf-8").read()
except OSError:
    text = ""

def find_port(name, default):
    match = re.search(rf"(?m)^{re.escape(name)}:\s*\"?([0-9]+)\"?", text)
    return int(match.group(1)) if match else default

api_match = re.search(r'(?m)^external-controller:\s*"?([^"\n]+)"?', text)
controller = api_match.group(1).strip() if api_match else "127.0.0.1:9090"
api_host, _, api_port_text = controller.rpartition(":")
api_host = api_host.strip("[]") or "127.0.0.1"
api_port = int(api_port_text or "9090")
proxy_port = find_port("mixed-port", find_port("port", 7890))

shutdown = threading.Event()

class ApiHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path.startswith("/version"):
            body = json.dumps({"version": "smoke-systemd", "mode": "rule"}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        if self.path.startswith("/logs"):
            body = json.dumps({"logs": [{"type": "info", "payload": "systemd smoke"}]}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        self.send_response(404)
        self.end_headers()

    def log_message(self, *_):
        return

class ThreadingTCPServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    allow_reuse_address = True
    daemon_threads = True

def proxy_loop():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind(("127.0.0.1", proxy_port))
        sock.listen()
        sock.settimeout(0.2)
        while not shutdown.is_set():
            try:
                conn, _ = sock.accept()
            except (TimeoutError, socket.timeout):
                continue
            except OSError:
                break
            conn.close()

def stop(*_):
    shutdown.set()

signal.signal(signal.SIGTERM, stop)
signal.signal(signal.SIGINT, stop)

api_server = ThreadingTCPServer((api_host, api_port), ApiHandler)
api_thread = threading.Thread(target=api_server.serve_forever, daemon=True)
proxy_thread = threading.Thread(target=proxy_loop, daemon=True)
api_thread.start()
proxy_thread.start()

while not shutdown.is_set():
    time.sleep(0.2)

api_server.shutdown()
api_server.server_close()
PY
KERNEL
    chmod 755 "$CLASH_BASE_DIR/bin/${KERNEL_NAME}"
}

seed_fake_yq() {
    install -d "$CLASH_BASE_DIR/bin"
    cat > "$CLASH_BASE_DIR/bin/yq" <<'YQ'
#!/usr/bin/env bash
set -euo pipefail
if [ "${1:-}" = "-i" ]; then
    expr="${2:-}"
    file="${3:-}"
    if [ -z "$file" ]; then
        exit 1
    fi
    if [[ "$expr" == *".secret"* ]]; then
        secret="${CLASHCTL_SECRET:-}"
        if grep -q '^secret:' "$file"; then
            sed -i "s|^secret:.*|secret: \"${secret}\"|" "$file"
        else
            printf '\nsecret: "%s"\n' "$secret" >> "$file"
        fi
    fi
    exit 0
fi
if [ "${1:-}" = "eval-all" ]; then
    mixin="${@: -1}"
    cat "$mixin"
    exit 0
fi
if [ "${1:-}" = "eval" ]; then
    exit 0
fi
echo "fake yq unsupported args: $*" >&2
exit 1
YQ
    chmod 755 "$CLASH_BASE_DIR/bin/yq"
}

seed_fake_resources() {
    install -d "$CLASH_BASE_DIR/resources" "$CLASH_BASE_DIR/bin/subconverter"
    printf 'smoke\n' > "$CLASH_BASE_DIR/resources/Country.mmdb"
    printf 'smoke\n' > "$CLASH_BASE_DIR/resources/geosite.dat"
    printf 'smoke\n' > "$CLASH_BASE_DIR/resources/geoip.dat"
    cat > "$CLASH_BASE_DIR/bin/subconverter/subconverter" <<'SUB'
#!/usr/bin/env bash
trap 'exit 0' TERM INT
while :; do sleep 60; done
SUB
    chmod 755 "$CLASH_BASE_DIR/bin/subconverter/subconverter"
}

require_cmd bash
require_cmd go
require_cmd install
require_cmd python3
require_cmd systemctl

if [ ! -d /run/systemd/system ]; then
    fail "systemd is not running on this host"
fi

cleanup
log "building local clashctl"
go build -trimpath -o /tmp/clashctl-systemd-smoke ./cmd/clashctl
sudo_cmd install -D /tmp/clashctl-systemd-smoke /usr/local/bin/clashctl
rm -f /tmp/clashctl-systemd-smoke

log "seeding fake kernel/yq/geodata"
seed_fake_kernel
seed_fake_yq
seed_fake_resources

log "installing systemd service"
bash "$ROOT_DIR/install.sh" --force --skip-cli
test -f "/etc/systemd/system/${SERVICE_NAME}.service" || fail "systemd unit missing"
systemctl cat "$SERVICE_NAME" >/tmp/clashctl-systemd-smoke-unit.txt
grep -q "$CLASH_BASE_DIR/bin/${KERNEL_NAME}" /tmp/clashctl-systemd-smoke-unit.txt || fail "unit does not use seeded kernel"

log "starting service through clashctl"
sudo_cmd /usr/local/bin/clashctl start
systemctl is-active --quiet "$SERVICE_NAME" || fail "systemd service is not active"
systemctl status "$SERVICE_NAME" --no-pager >/tmp/clashctl-systemd-smoke-status.txt
/usr/local/bin/clashctl status >/tmp/clashctl-systemd-smoke-clashctl-status.txt
grep -q 'kernel: running' /tmp/clashctl-systemd-smoke-clashctl-status.txt || fail "clashctl status did not report running"

log "running doctor"
set +e
/usr/local/bin/clashctl doctor >/tmp/clashctl-systemd-smoke-doctor.txt
doctor_code=$?
set -e
if [ "$doctor_code" -ne 0 ]; then
    cat /tmp/clashctl-systemd-smoke-doctor.txt >&2
    fail "doctor returned ${doctor_code}"
fi
grep -q 'api auth' /tmp/clashctl-systemd-smoke-doctor.txt || fail "doctor did not verify API auth"

log "checking logs"
/usr/local/bin/clashctl log >/tmp/clashctl-systemd-smoke-log.txt || true
journalctl -u "$SERVICE_NAME" --no-pager -n 20 >/tmp/clashctl-systemd-smoke-journal.txt

log "stopping service"
sudo_cmd /usr/local/bin/clashctl stop
if systemctl is-active --quiet "$SERVICE_NAME"; then
    fail "systemd service still active after stop"
fi

log "running uninstall cleanup"
printf 'n\nn\n' | bash "$ROOT_DIR/uninstall.sh"
test ! -f "/etc/systemd/system/${SERVICE_NAME}.service" || fail "systemd unit was not removed"
test ! -d "$CLASH_BASE_DIR" || fail "base dir was not removed"
test ! -e /usr/local/bin/clashctl || fail "clashctl binary was not removed"

log "systemd smoke passed"
