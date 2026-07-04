# 文档索引

本文档说明当前仍有效的项目文档，以及已经归档的历史设计资料。

English: This index separates current documentation from archived historical planning documents.

## 当前有效文档

- [README](../README.md)：用户入口文档，中文为主，包含安装、更新、卸载、TUN、验证和当前状态。
- [EXTERNAL_VALIDATION_CHECKLIST.md](EXTERNAL_VALIDATION_CHECKLIST.md)：真实 Linux、CI、release、TUN、桌面代理等外部验证清单。
- [INSTALL_SMOKE.md](INSTALL_SMOKE.md)：CI install smoke 的覆盖范围、运行方式和限制。

## 当前验证基线

当前已验证的 release candidate 是 `v0.2.0-rc.16`：

- GitHub Actions `CI`、`Install Smoke`、`Release` 通过。
- release artifact 与 `SHA256SUMS` 已生成。
- 裸机安装、镜像 fallback、真实 Mihomo/yq/geodata 下载、systemd service、TUI 启动、TUN route guard 和卸载 proxy 提示已有实机验证。

当前分支包含 `v0.2.0-rc.16` 之后的卸载体验改进：卸载数据保留项已拆分为订阅、配置、geodata、流量、日志、TUI 设置等独立选择。该改动需要新的 RC 或正式 tag 才会进入安装版用户路径。

English: `v0.2.0-rc.16` is the validated baseline. The branch has post-rc.16 uninstall UX improvements awaiting the next tag.

## 归档文档

以下文档已经移动到 [archive](archive)，仅作为历史背景：

- `HARDENING_STATUS.md`
- `PROJECT_AUDIT_REPORT.md`
- `PROJECT_HARDENING_SPEC.md`
- `RATATUI_COMPONENT_REFACTOR_PLAN.md`
- `RATATUI_REDESIGN_PLAN.md`

归档原因：这些文档包含早期 hardening 状态、旧的验证判定、旧的 TUI 计划或已经被实现/修正的风险描述。它们不再代表当前项目状态。

English: Archived documents may contain outdated status, old plans, or already-resolved risks.
