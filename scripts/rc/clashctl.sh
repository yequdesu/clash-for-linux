#!/bin/bash
# ============================================================
# Clash-Terminal — Shell RC 集成 (Bash函数 fallback)
# 无 clashctl Go 二进制时使用
# ============================================================

# 代理开关
proxy_on() {
    export http_proxy=http://127.0.0.1:7897
    export HTTP_PROXY=http://127.0.0.1:7897
    export https_proxy=http://127.0.0.1:7897
    export HTTPS_PROXY=http://127.0.0.1:7897
    export all_proxy=socks5h://127.0.0.1:7897
    export ALL_PROXY=socks5h://127.0.0.1:7897
    export no_proxy=localhost,127.0.0.0/8,::1
    export NO_PROXY=localhost,127.0.0.0/8,::1
    echo "✓ System proxy enabled"
}

proxy_off() {
    unset http_proxy HTTP_PROXY
    unset https_proxy HTTPS_PROXY
    unset all_proxy ALL_PROXY
    unset no_proxy NO_PROXY
    echo "✓ System proxy disabled"
}

proxy_status() {
    if [ -n "${http_proxy:-}" ]; then
        echo "● System proxy: ${http_proxy}"
    else
        echo "○ System proxy: not set"
    fi
}

# 别名
alias pon=proxy_on
alias poff=proxy_off
alias pst=proxy_status

echo "Clash-Terminal shell functions loaded."
echo "  proxy_on   - Enable proxy"
echo "  proxy_off  - Disable proxy"
echo "  proxy_status - Show proxy status"
