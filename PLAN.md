# kou — 项目状态与计划 (PLAN)

> 本文件随发布元数据补全于 **2026-07-04** 刷新，记录项目当前状态、近期进展与后续计划。

## 1. 项目概述

- **名称**：`kou`
- **简介**：虚拟终端引擎（VT），含 PTY 管理、VT100/ANSI 屏幕模拟、PNG 渲染与图形协议（kitty / iTerm2 / sixel）。
- **远程仓库**：`Cargo.toml` 声明 `https://github.com/celestia-island/kou`；本地未配置 git remote（无 origin），当前分支 `dev`
- **技术栈**：Rust (edition 2024) / just
- **类别**：rust-lib

## 2. 当前状态

- **当前分支**：`dev`
- **工作区**：干净
- **最近提交时间**：2026-07-04
- **最近提交**：docs: docs.rs badge + crates.io release metadata

## 3. 未提交改动

无。

## 4. 近期进展（最近提交）

- docs: docs.rs badge + crates.io release metadata（补 docs.rs 徽章、keywords/categories、`[package.metadata.docs.rs]`）
- docs: add PLAN.md current-status snapshot
- test: large-format snapshots (120×60) + inline image rendering tests
- docs: neofetch-style snapshots + rainbow gradient + protocol table
- docs: screenshots in guides + <details> before License + wider screens
- test: snapshot tests via raw ANSI → Screen::feed → render_png
- docs: add res/ snapshots + showcase in README
- fix: clippy clean (type alias, manual_contains, unnecessary casts, is_empty)

## 5. 后续计划

1. ~~完善文档示例与 `crates.io` 发布元数据~~ ✅ 已完成：README（含全部多语言版本）顶部 docs.rs 徽章、`keywords`/`categories`、`[package.metadata.docs.rs] all-features = true`。
2. **修复 `sixel` feature 编译错误**（5 处：`GraphicsProtocol::Off` 非穷尽 match + sixel 编/解码的类型不匹配），否则 `--all-features` 与 docs.rs 的 `all-features = true` 构建会失败。该问题为既有缺陷，非本次改动引入。
3. 补充单元/集成测试，保持 `just test` 与 clippy `-D warnings` 通过。
4. 定期刷新本 PLAN.md 以反映最新状态。

## 6. 验证记录（2026-07-04）

- `cargo check`（默认特性）：通过。
- `cargo clippy -- -D warnings`（默认特性）：通过，无告警。
- `cargo test --lib`（默认特性）：通过，32 passed / 0 failed。
- `cargo check --features mcp`：通过。
- ⚠️ `cargo check --all-features`：失败（`sixel` feature 既有缺陷，见后续计划 #2）。
