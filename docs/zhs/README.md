<p align="center"><img src="../logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>虚拟终端自动化——PTY + 真正的 VT100 屏幕 + 构建时字体预取 + 带内图形协议</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](../../LICENSE)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/kou/checks.yml)](https://github.com/celestia-island/kou/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-kou.docs.celestia.world-blue)](https://kou.docs.celestia.world)

</div>

<div align="center">

[English](../en/README.md) ·
**简体中文** ·
[繁體中文](../zht/README.md) ·
[日本語](../ja/README.md) ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## 简介

kou 是一个独立的虚拟终端引擎——集 PTY 管理、真正的 VT100/ANSI 屏幕模拟器以及
真正会绘制字形的屏幕渲染于一身。它是从 tairitsu 打包器中提取的 vtty 核心，
经过强化后成为独立的库和命令行工具。

让它区别于普通 PTY 封装的三点：

- **真正的屏幕。** 字节流通过 [`vte`](https://crates.io/crates/vte) 解析器
  处理，因此 CSI 光标移动、擦除、滚动以及 SGR 16 色调色板都能被正确响应——
  而不是早期原型那种"丢弃 ESC 序列"的占位实现。
- **构建时字体预取。** kou 不内置字体；它会在首次使用时将精选字体族（Latin 用
  Fira Code / JetBrains Mono；CJK 用思源黑体 / 更纱黑体 / 得意黑）拉取到
  共享缓存中，并提供镜像/代理开关以适配受限网络环境。字形由 `ab_glyph`
  光栅化，Latin 优先、CJK 兜底，因此单次渲染即可混排多种文字而不会出现
  豆腐块。
- **带内图形。** 一帧既可以光栅化为 PNG，也可以通过 kitty（`kitty2`）或 iTerm2
  图形协议描述给支持这些协议的终端——这样 wezterm / kitty / iTerm2 / Ghostty
  就能原位渲染真实像素。

## 快速开始

### CLI

```bash
# Launch a command in a virtual terminal and drive it from a REPL.
kou launch bash --cols 80 --rows 24
# > echo hello
# > screen        # prints the current screen text
```

### 库

```rust
use kou::{FontCache, FontSet, VttyManager, render_png};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mgr = VttyManager::new();
    let id = mgr.launch("bash", None, 80, 24).await?;
    mgr.send_text(&id, "echo hello\n").await?;

    // Plain text.
    println!("{}", mgr.screenshot(&id).await?);

    // A real PNG, rendered with auto-fetched fonts (Latin + CJK fallback).
    let fonts = FontCache::load(&FontSet::from_env(), 16.0);
    let screen = mgr.screen(&id).await?;
    let png = render_png(&screen, &fonts, 16.0)?;
    std::fs::write("screen.png", png)?;
    Ok(())
}
```

## 图形协议

| `KOU_GRAPHICS` | 协议 | 支持的终端 |
|----------------|------|-----------|
| `kitty` / `kitty2` | kitty APC 图形协议 | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 内联图像 | iTerm2, wezterm |
| `sixel` | DCS sixel | （占位——需要光栅化器） |
| `off`（默认） | 无——带外渲染为 PNG | 全部 |

```rust
use kou::{FontCache, FontSet, GraphicsProtocol, VttyManager, render_graphics};
let frame = render_graphics(&screen, &FontCache::load(&FontSet::from_env(), 16.0), 16.0,
                            GraphicsProtocol::from_env());
if let Some(escape) = frame {
    print!("{escape}"); // capable terminals render the pixels inline
}
```

## 字体与拉取

kou 不内置字体——它会在构建时将精选字体族拉取到共享缓存中，
并提供镜像/代理开关以适配受限网络环境。每种文字各选择**一种**字体；
默认值与备选如下：

| 文字 | 默认 | 备选 |
|------|------|------|
| Latin | Fira Code | JetBrains Mono |
| CJK | Source Han Sans SC (思源黑体) | Sarasa Mono SC (更纱黑体), Smiley Sans (得意黑), `none` |

通过 `KOU_FONT_PRIMARY` / `KOU_FONT_CJK` 选择主要/CJK 字体族，或通过
`KOU_FONT_PATH` / `KOU_FONT_CJK_PATH` 指定字体文件。解析顺序：
显式路径 → 共享缓存 → 运行时下载（`font-fetch` 特性，默认启用）。

| 环境变量 | 用途 |
|---------|------|
| `KOU_FONT_PRIMARY` | `fira-code`（默认）/ `jetbrains-mono` |
| `KOU_FONT_CJK` | `sourcehansans`（默认）/ `sarasa` / `smileysans` / `none` |
| `KOU_FONT_MIRROR` | 将 GitHub / jsDelivr 主机替换为镜像源。 |
| `KOU_DOWNLOAD_PROXY` | 通过 http/https/socks 代理路由字体下载。 |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | 单次请求超时时间（默认 120）。 |
| `KOU_SKIP_FONT_FETCH` | 禁用运行时字体拉取。 |

## 开发

```bash
cargo check --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## 许可证

SySL-1.0（Synthetic Source License）。详见 [LICENSE](../../LICENSE)。
