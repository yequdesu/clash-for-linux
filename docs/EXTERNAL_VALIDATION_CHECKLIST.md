# External Validation Checklist

本文档用于真实 Linux、CI、release、TUN 和桌面代理验证。它是当前有效验证清单；历史硬化规格已归档到 `docs/archive/PROJECT_HARDENING_SPEC.md`。

建议只在干净 VM、临时云主机或专用测试机执行。不要直接在日常主力机上验证卸载、TUN、桌面代理和内核升级。

## 0. 记录验证上下文

每次验证先记录：

```bash
date -Is
git rev-parse HEAD
git status --short
uname -a
cat /etc/os-release
id
go version || true
rustc --version || true
cargo --version || true
systemctl --version || true
```

证据至少保存：

- 发行版和版本。
- commit SHA。
- 每条命令的退出码。
- 关键 stdout/stderr。
- `install-state.json`。
- `systemctl status clashctl --no-pager`。
- `clashctl doctor` 输出。
- 失败时保留日志：`clashctl log`、`journalctl -u clashctl --no-pager -n 200`。

## 1. GitHub Actions 验证

必须在 GitHub 上看到这些 workflow 通过：

- `CI`
  - Go format check。
  - `go test ./...`。
  - `GOOS=linux GOARCH=amd64 go test ./...`。
  - `GOOS=linux GOARCH=amd64 go build ./cmd/clashctl`。
  - `bash scripts/smoke/cli_exit_codes.sh`。
  - `go vet ./...`。
  - Rust `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`。
  - shell syntax、static safety、nohup pid safety。
- `Install Smoke`
  - release artifact installer。
  - systemd service lifecycle。
  - Ubuntu、Debian、Fedora、Arch 容器安装 smoke。
- `Release`
  - amd64 artifact。
  - arm64 artifact。
  - `SHA256SUMS`。

通过标准：

- 所有 required jobs 绿色。
- `Release` 生成 `clash-for-linux-amd64.tar.gz`、`clash-for-linux-arm64.tar.gz`、`SHA256SUMS`。
- 若任一 job 失败，不能把对应状态标记为真实验证通过。

## 2. Linux VM + systemd smoke

用途：先用 fake Mihomo 在真实 systemd 上验证 unit、start/status/doctor/stop 逻辑。

前置条件：

- VM 使用 systemd 作为 init。
- 已安装 `bash`、`go`、`python3`、`curl`、`sudo`、`systemctl`。

执行：

```bash
export CLASH_BASE_DIR=/tmp/clashctl-systemd-smoke
export SERVICE_NAME=clashctl
export KERNEL_NAME=mihomo
bash scripts/smoke/systemd_service.sh
```

通过标准：

- `/etc/systemd/system/clashctl.service` 被创建。
- unit 指向当前 `CLASH_BASE_DIR/bin/mihomo`。
- `clashctl start` 通过 systemd 启动服务。
- `systemctl is-active clashctl` 为 active。
- `clashctl status` 显示 running。
- `clashctl doctor` 返回 0，并验证 API auth。
- `clashctl stop` 后服务不再 active。

## 3. CLI 退出码 smoke

用途：在真实 Linux runner 上验证 `ilog.Fatal/os.Exit` 路径不会被单元测试遗漏。

执行：

```bash
bash scripts/smoke/cli_exit_codes.sh
```

通过标准：

- 父命令帮助调用返回非 0：`clashctl`、`config`、`sub`、`node`、`geodata`。
- 空状态展示命令返回 0：`sub list`、`sub log`、`proxy on`、`proxy off`。
- 失败路径返回非 0：非法 proxy 参数、空 secret show、`sub update` 无订阅、非法订阅 ID、损坏 profiles 元数据。

## 4. 真实安装与真实下载

用途：验证真实 Mihomo、yq、geodata、CLI 和 TUI 的安装链路。

前置条件：

- 干净 VM。
- 有 sudo。
- 有外网访问 GitHub 或配置了可用代理。
- 若验证 TUI 源码构建，需要 Rust toolchain。

建议环境：

```bash
export CLASH_BASE_DIR="$HOME/clashctl-validation"
export SERVICE_NAME=clashctl
export KERNEL_NAME=mihomo
export INIT_TYPE=systemd
export CLASHCTL_SKIP_RELEASE=true
```

安装：

```bash
bash install.sh --force --with-tui
```

检查：

```bash
test -x /usr/local/bin/clashctl
test -x /usr/local/bin/clash-tui
test -x "$CLASH_BASE_DIR/bin/mihomo"
test -x "$CLASH_BASE_DIR/bin/yq"
test -f "$CLASH_BASE_DIR/resources/Country.mmdb"
test -f "$CLASH_BASE_DIR/resources/geosite.dat"
test -f "$CLASH_BASE_DIR/resources/geoip.dat"
python3 -m json.tool "$CLASH_BASE_DIR/install-state.json"
clashctl version
clashctl status
clashctl doctor || true
```

通过标准：

- 安装命令返回 0。
- `install-state.json` 存在且 JSON 合法。
- `components.mihomo`、`components.yq`、`components.geodata`、`components.clashctl` 存在。
- `runtime.yaml` 存在。
- `external-controller` 默认是 loopback。
- secret 非空，普通 `clashctl status` 不打印 secret 明文。

## 5. 真实订阅与内核启动

前置条件：

- 已完成真实安装。
- 准备一个可用订阅 URL。

执行：

```bash
export CLASH_SUB_URL='REPLACE_WITH_REAL_SUBSCRIPTION_URL'
clashctl sub add "$CLASH_SUB_URL"
clashctl sub list
clashctl config merge
clashctl start
systemctl status clashctl --no-pager
systemctl is-active --quiet clashctl
clashctl status
clashctl doctor
clashctl node list
clashctl test https://www.gstatic.com/generate_204
```

通过标准：

- `sub add` 返回 0，并写入 profile。
- `sub list` 显示 active profile。
- `clashctl start` 返回 0。
- `systemctl is-active --quiet clashctl` 返回 0。
- `clashctl doctor` 返回 0，或只返回已明确接受的 warning；不能有 fatal。
- `node list` 能从真实 Mihomo API 获取代理组。
- `clashctl test` 通过代理返回成功 HTTP 状态。

失败判定：

- 订阅下载、转换、配置校验、active profile 激活失败任一返回 0，都视为失败。
- `systemctl` active 但 `clashctl status` stopped，视为失败。
- `clashctl status` running 但 `systemctl` inactive，视为失败。

## 6. 日志验证

执行：

```bash
clashctl log
journalctl -u clashctl --no-pager -n 200
ls -la "$CLASH_BASE_DIR/logs"
```

通过标准：

- `clashctl log` 能读取项目当前服务的日志，或在没有日志时明确说明路径。
- `journalctl -u clashctl` 能看到同一服务的启动/停止记录。
- 日志命令不得再查错 unit 名。

## 7. TUN 验证

前置条件：

- 已完成真实订阅与内核启动。
- VM/测试机支持 `/dev/net/tun`。
- 可以使用 sudo。

执行：

```bash
test -e /dev/net/tun
command -v ip
sudo -E clashctl tun on
clashctl status
clashctl doctor || true
ip link show
ip rule show
ip route show table all | sed -n '1,120p'
clashctl test https://www.gstatic.com/generate_204
sudo -E clashctl tun off
clashctl status
```

通过标准：

- 无 `/dev/net/tun` 时，`tun on` 必须失败且旧代理继续可用。
- capability 缺失时，输出必须包含可执行的 `setcap` 修复命令。
- `tun on` 成功后，runtime 中 `tun.enable` 为 true，且能观察到 TUN 相关网络设备或路由变化。
- `tun off` 成功后，runtime 中 `tun.enable` 为 false。
- 失败时不得留下半更新 `mixin.yaml/runtime.yaml` 或不可启动服务。

## 8. 桌面代理验证

前置条件：

- 在真实图形桌面会话内执行，不要只在 SSH session 中执行。
- GNOME 需要 `gsettings`。
- KDE 需要 `kwriteconfig6` 或 `kwriteconfig5`。

执行：

```bash
clashctl proxy desktop status
clashctl proxy desktop on
clashctl proxy desktop status
clashctl proxy desktop off
clashctl proxy desktop status
```

GNOME 可额外检查：

```bash
gsettings get org.gnome.system.proxy mode
gsettings get org.gnome.system.proxy.http host
gsettings get org.gnome.system.proxy.http port
gsettings get org.gnome.system.proxy.socks host
gsettings get org.gnome.system.proxy.socks port
```

KDE 可额外检查：

```bash
kreadconfig6 --file kioslaverc --group "Proxy Settings" --key ProxyType || true
kreadconfig5 --file kioslaverc --group "Proxy Settings" --key ProxyType || true
```

通过标准：

- 支持的桌面环境中，`desktop on` 返回 0 并能观察到系统代理配置改变。
- `desktop off` 返回 0 并能观察到系统代理关闭。
- 不支持的桌面环境或工具缺失时，`desktop on/off` 必须返回非 0；`desktop status` 可以返回 0 并说明不支持。

## 9. geodata 更新验证

执行：

```bash
clashctl geodata update --version latest
python3 -m json.tool "$CLASH_BASE_DIR/install-state.json"
ls -l "$CLASH_BASE_DIR/resources/Country.mmdb" \
      "$CLASH_BASE_DIR/resources/geosite.dat" \
      "$CLASH_BASE_DIR/resources/geoip.dat"
```

通过标准：

- 三个 geodata 文件都存在且非空。
- `install-state.json` 中 `components.geodata.version`、`source_url`、`path`、`updated_at` 更新。
- 任一下载或状态写入失败时，旧 geodata 文件保持可用，不出现混合版本。

## 10. 内核升级验证

执行前先确认当前版本：

```bash
clashctl status
clashctl doctor
```

执行：

```bash
clashctl upgrade-kernel
clashctl status
clashctl doctor
python3 -m json.tool "$CLASH_BASE_DIR/install-state.json"
```

如果 release 没有 checksum，只有在明确接受风险时才执行：

```bash
clashctl upgrade-kernel --allow-unsigned
```

通过标准：

- 有 checksum 时必须校验成功。
- 成功后 `components.mihomo` 包含 `version`、`source_url`、`checksum_url`、`checksum_verified`、`allow_unsigned`、`path`、`updated_at`。
- 新内核启动失败时，命令返回非 0，旧内核恢复并能启动。
- `install-state.json` 写入失败时，命令返回非 0，旧内核和旧状态保持可用。

## 11. Release artifact 安装验证

用途：验证 GitHub release 产物而不是源码 fallback。

准备变量：

```bash
export RELEASE_TAG='vX.Y.Z'
export CLASHCTL_RELEASE_BASE_URL="https://github.com/yequdesu/clash-for-linux/releases/download/${RELEASE_TAG}"
export CLASH_BASE_DIR="$HOME/clashctl-release-validation"
export SERVICE_NAME=clashctl
export KERNEL_NAME=mihomo
export INIT_TYPE=systemd
unset CLASHCTL_SKIP_RELEASE
```

执行：

```bash
curl -fsS "$CLASHCTL_RELEASE_BASE_URL/SHA256SUMS"
bash install.sh --force --with-tui
test -x /usr/local/bin/clashctl
test -x /usr/local/bin/clash-tui
python3 -m json.tool "$CLASH_BASE_DIR/install-state.json"
grep -F "$CLASHCTL_RELEASE_BASE_URL" "$CLASH_BASE_DIR/install-state.json"
clashctl version
clashctl status
```

通过标准：

- installer 下载 tarball 和 `SHA256SUMS`。
- checksum 校验通过。
- `install-state.json` 中 `components.clashctl.source_url` 和 `components.clash_tui.source_url` 指向 release tarball。
- 若 checksum 缺失、tarball 缺失或 hash 不匹配，安装必须失败，不能 fallback 成伪成功。

## 12. 卸载验证

执行：

```bash
clashctl stop || true
printf 'n\nn\n' | bash uninstall.sh
test ! -e /usr/local/bin/clashctl
test ! -e /usr/local/bin/clash-tui
test ! -e "/etc/systemd/system/${SERVICE_NAME}.service"
test ! -d "$CLASH_BASE_DIR"
systemctl daemon-reload
```

通过标准：

- 受管服务停止。
- systemd unit 删除。
- `/usr/local/bin/clashctl` 和 `/usr/local/bin/clash-tui` 删除。
- 测试安装目录删除。
- 卸载脚本不得杀死不属于当前安装目录的无关进程。

## 13. 验证结果回填

每个验证项记录为：

```text
item:
host:
os:
commit:
command:
exit_code:
result: pass|fail
evidence:
notes:
```

回填规则：

- 历史 `docs/archive/HARDENING_STATUS.md` 中的旧状态判定仅作为归档参考；当前状态以仓库根目录 `README.md` 和 `docs/README.md` 为准。
- 如果真实验证失败，先记录失败命令、输出、日志和环境，不要只记录“失败”。
- 对于 release、systemd、TUN、desktop proxy，必须至少保存一份成功日志，才能作为稳定发布依据。
