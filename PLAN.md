# kou — 项目状态与计划 (PLAN)

> 本文件由自动化扫描于 **2026-07-04** 生成，记录项目当前状态、近期进展与后续计划。

## 1. 项目概述

- **名称**：`kou`
- **简介**：虚拟终端引擎（VT），含 ANSI 快照与 PNG 渲染测试。
- **远程仓库**：本地仓库（无 origin）
- **技术栈**：Rust / just
- **类别**：rust-lib

## 2. 当前状态

- **当前分支**：`dev`
- **工作区**：干净
- **最近提交时间**：2026-07-04
- **最近提交**：test: large-format snapshots (120×60) + inline image rendering tests

## 3. 未提交改动

无。

## 4. 近期进展（最近提交）

- test: large-format snapshots (120×60) + inline image rendering tests
- docs: neofetch-style snapshots + rainbow gradient + protocol table
- docs: screenshots in guides + <details> before License + wider screens
- test: snapshot tests via raw ANSI → Screen::feed → render_png
- docs: add res/ snapshots + showcase in README
- fix: clippy clean (type alias, manual_contains, unnecessary casts, is_empty)

## 5. 后续计划

1. 完善文档示例与 `crates.io` 发布元数据（rust-version / metadata / docs.rs badge）。
2. 补充单元/集成测试，保持 `just test` 与 clippy `-D warnings` 通过。
3. 定期刷新本 PLAN.md 以反映最新状态。

