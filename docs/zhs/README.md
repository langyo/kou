<p align="center"><img src="../logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>虚拟终端自动化——PTY + VT100 屏幕 + 构建时字体预取 + 带内图形协议</strong></p>

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

kou 是一个独立的虚拟终端引擎——集 PTY 管理、VT100/ANSI 屏幕模拟器以及
会绘制字形的屏幕渲染于一身。它是从 tairitsu 打包器中提取的 vtty 核心，
经过强化后成为独立的库和命令行工具。

让它区别于普通 PTY 封装的三点：

- **VT100 屏幕。** 字节流通过 [`vte`](https://crates.io/crates/vte) 解析器
  处理，因此 CSI 光标移动、擦除、滚动以及 SGR 16 色调色板都能被正确响应——
  而不是早期原型那种"丢弃 ESC 序列"的占位实现。
- **构建时字体预取。** kou 会在构建时为每种文字各预下载一个字体——Latin 用
  Fira Code，CJK 用 Source Han Sans，阿拉伯文用 Noto Naskh Arabic，
  Noto Sans Devanagari、Noto Sans Thai——并存入共享缓存。可通过环境变量
  覆盖字体族或指定本地文件；身处受限网络时，可经由 HTTP(S) 代理（传给
  reqwest）来路由下载。字形由 `ab_glyph` 光栅化，按顺序逐个尝试字体面，
  因此单次渲染即可混排多种文字而不会出现豆腐块。
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

kou 会在构建时为每种文字各预下载一个字体到共享缓存中：

| 文字 | 字体 |
|------|------|
| Latin | Fira Code |
| CJK (中文 · 日本語 · 한국어) | Source Han Sans SC (思源黑体) |
| Arabic | Noto Naskh Arabic |
| Devanagari (हिन्दी · मराठी) | Noto Sans Devanagari |
| Thai (ไทย) | Noto Sans Thai |

在构建时通过 `KOU_FONT_PRIMARY` / `KOU_FONT_CJK` / `KOU_FONT_ARABIC` /
`KOU_FONT_DEVANAGARI` / `KOU_FONT_THAI` 覆盖任意字体族，或通过
`KOU_FONT_*_PATH` 指定本地文件。下载可经由 `KOU_DOWNLOAD_PROXY`（直接传给
reqwest）通过 HTTP(S) 代理路由。

| 环境变量 | 用途 |
|---------|------|
| `KOU_FONT_PRIMARY` | 覆盖 Latin 字体族。 |
| `KOU_FONT_CJK` | 覆盖 / 禁用 CJK 字体（`none` 为禁用）。 |
| `KOU_FONT_ARABIC` | 覆盖 / 禁用阿拉伯文字体。 |
| `KOU_FONT_DEVANAGARI` | 覆盖 / 禁用天城文字体。 |
| `KOU_FONT_THAI` | 覆盖 / 禁用泰文字体。 |
| `KOU_FONT_MIRROR` | 将下载主机替换为镜像源。 |
| `KOU_DOWNLOAD_PROXY` | 通过 HTTP(S) 代理路由下载（reqwest）。 |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | 单次请求超时时间（默认 120）。 |
| `KOU_SKIP_FONT_FETCH` | 禁用字体拉取。 |

## 开发

```bash
cargo check --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## 许可证

SySL-1.0（Synthetic Source License）。详见 [LICENSE](../../LICENSE)。
