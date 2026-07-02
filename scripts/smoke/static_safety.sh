#!/usr/bin/env bash
# Static safety checks for patterns that are easy to regress in shell scripts.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

fail() {
    printf '[static-safety] ERROR: %s\n' "$*" >&2
    exit 1
}

require_pattern() {
    local file="$1"
    local pattern="$2"
    grep -qE "$pattern" "$file" || fail "$file does not contain required pattern: $pattern"
}

deny_pattern() {
    local file="$1"
    local pattern="$2"
    if grep -qE "$pattern" "$file"; then
        fail "$file contains forbidden pattern: $pattern"
    fi
}

require_pattern install.sh '^_pid_matches_kernel\(\)'
require_pattern install.sh '_pid_matches_kernel "\$pid"'
require_pattern uninstall.sh '^_pid_matches_kernel\(\)'
require_pattern uninstall.sh '_pid_matches_kernel "\$pid"'
require_pattern scripts/init/nohup.sh '^pid_matches_kernel\(\)'
require_pattern scripts/init/nohup.sh 'pid_matches_kernel "\$pid"'

for file in install.sh update.sh uninstall.sh install_tui.sh scripts/init/nohup.sh scripts/init/systemd.sh; do
    deny_pattern "$file" 'pkill'
    deny_pattern "$file" 'killall'
done

printf '[static-safety] ok\n'
