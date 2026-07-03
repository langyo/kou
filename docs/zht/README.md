<p align="center"><img src="../logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>虛擬終端機自動化——PTY + 真正的 VT100 螢幕 + ort 風格字型 + 帶內圖形協定。</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](../../LICENSE)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/kou/checks.yml)](https://github.com/celestia-island/kou/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-kou.docs.celestia.world-blue)](https://kou.docs.celestia.world)

</div>

<div align="center">

[English](../en/README.md) ·
[简体中文](../zhs/README.md) ·
**繁體中文** ·
[日本語](../ja/README.md) ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## 簡介

kou 是一個獨立的虛擬終端機引擎——PTY 管理、真正的 VT100/ANSI 螢幕模擬器，以及會實際繪製字形的螢幕渲染器。它是從 tairitsu 打包器中抽出的 vtty 核心，經過強化後獨立成為一個程式庫與命令列工具。

以下三點讓它不同於一個簡陋的 PTY 封裝：

- **真正的螢幕。** 位元組串流會經過 [`vte`](https://crates.io/crates/vte) 解析器處理，因此 CSI 游標移動、清除、捲動以及 SGR 16 色調色盤都會被正確處理——不再是早期原型中那種「直接丟掉 ESC」的陽春做法。
- **ort 風格字型。** kou 本身不內嵌字型；首次使用時會將一套精選字型家族（拉丁字型為 Fira Code / JetBrains Mono；CJK 字型為思源黑體 / 更紗黑體 / 得意黑）擷取至共用快取中，並提供鏡像站/代理伺服器選項以適應受限網路環境。字形由 `ab_glyph` 進行點陣化，先處理拉丁字元再處理 CJK 字元，因此單次渲染即可混合多種書寫系統而不會出現缺字（tofu）。
- **帶內圖形。** 畫面可以點陣化為 PNG，或透過 kitty（`kitty2`）或 iTerm2 圖形協定描述給支援的終端機——因此 wezterm / kitty / iTerm2 / Ghostty 可以直接在行內顯示真實像素。

## 快速入門

### 命令列介面

```bash
# Launch a command in a virtual terminal and drive it from a REPL.
kou launch bash --cols 80 --rows 24
# > echo hello
# > screen        # prints the current screen text
```

### 程式庫

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

## 圖形協定

| `KOU_GRAPHICS` | 協定 | 終端機 |
|----------------|----------|-----------|
| `kitty` / `kitty2` | kitty APC 圖形 | kitty、wezterm、Ghostty |
| `iterm` / `iterm2` | OSC 1337 行內圖片 | iTerm2、wezterm |
| `sixel` | DCS sixel | （預留——需要點陣化器） |
| `off`（預設） | 無——在頻外渲染 PNG | 全部 |

```rust
use kou::{FontCache, FontSet, GraphicsProtocol, VttyManager, render_graphics};
let frame = render_graphics(&screen, &FontCache::load(&FontSet::from_env(), 16.0), 16.0,
                            GraphicsProtocol::from_env());
if let Some(escape) = frame {
    print!("{escape}"); // capable terminals render the pixels inline
}
```

## 字型與擷取

使用 `KOU_FONT_PRIMARY` / `KOU_FONT_CJK` 選擇主要字型 / CJK 字型家族，或使用 `KOU_FONT_PATH` / `KOU_FONT_CJK_PATH` 指定字型檔案。解析順序：明確路徑 → 共用快取 → 執行期下載（`font-fetch` 功能，預設啟用）。

| 環境變數 | 用途 |
|-----|---------|
| `KOU_FONT_PRIMARY` | `fira-code`（預設）/ `jetbrains-mono` |
| `KOU_FONT_CJK` | `sarasa`（預設）/ `sourcehansans` / `smileysans` / `none` |
| `KOU_FONT_MIRROR` | 以鏡像站取代 GitHub / jsDelivr 主機。 |
| `KOU_DOWNLOAD_PROXY` | 透過 http/https/socks 代理伺服器進行字型下載。 |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | 每個請求的逾時時間（預設 120 秒）。 |
| `KOU_SKIP_FONT_FETCH` | 停用執行期字型擷取。 |

## 開發

```bash
cargo check --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## 授權條款

SySL-1.0（Synthetic Source License）。詳見 [LICENSE](../../LICENSE)。
