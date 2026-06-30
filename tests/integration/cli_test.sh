#!/bin/bash
# tests/integration/cli_test.sh

set -e

CLASHCTL="./target/clashctl"

# 测试帮助命令
test_help() {
    output=$($CLASHCTL --help)
    if [[ ! "$output" == *"Clash-Terminal"* ]]; then
        echo "FAIL: --help output missing expected text"
        exit 1
    fi
    echo "PASS: test_help"
}

# 测试版本命令
test_version() {
    output=$($CLASHCTL version)
    if [[ ! "$output" == *"clashctl"* ]]; then
        echo "FAIL: version output missing expected text"
        exit 1
    fi
    echo "PASS: test_version"
}

# 测试状态命令（需要内核运行）
test_status() {
    output=$($CLASHCTL status 2>&1)
    # 检查是否有输出（不管内核是否运行）
    if [[ -z "$output" ]]; then
        echo "FAIL: status command produced no output"
        exit 1
    fi
    echo "PASS: test_status"
}

# 运行测试
test_help
test_version
test_status

echo "All integration tests passed!"
