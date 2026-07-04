# 归档文档

本目录保存历史规划、审计和重构设计资料。它们保留项目演进背景，但不再作为当前状态、用户安装方式或正式发布依据。

English: This directory contains historical planning and audit documents. They are not current user-facing guidance.

## 已归档文件

- `HARDENING_STATUS.md`：早期硬化状态矩阵，包含大量 `PARTIAL_LOCAL` 和旧验证缺口。
- `PROJECT_AUDIT_REPORT.md`：早期全面审计报告，部分风险已经修复或被后续 RC 验证覆盖。
- `PROJECT_HARDENING_SPEC.md`：硬化规格与目标文档，适合作为历史需求背景。
- `RATATUI_COMPONENT_REFACTOR_PLAN.md`：TUI 组件化/retained 架构重构计划。
- `RATATUI_REDESIGN_PLAN.md`：TUI 页面与交互设计计划。

## 使用原则

- 当前安装、更新、卸载、验证方式以仓库根目录 `README.md` 和 `docs/README.md` 为准。
- 归档文档中的状态、版本号、命令和风险等级可能过期。
- 如需恢复其中某项设计，应先基于当前代码重新审查，再新建当前设计文档。
