<p align="center"><img src="docs/logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>Virtual terminal automation — PTY + a real VT100 screen + ort-style fonts + inband graphics protocols.</strong></p>

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](./LICENSE) [![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/kou/checks.yml)](https://github.com/celestia-island/kou/actions/workflows/checks.yml) [![Docs](https://img.shields.io/badge/docs-kou.docs.celestia.world-blue)](https://kou.docs.celestia.world)

[English](./docs/en/README.md) · [简体中文](./docs/zhs/README.md) · [繁體中文](./docs/zht/README.md) · [日本語](./docs/ja/README.md) · [한국어](./docs/ko/README.md) · [Français](./docs/fr/README.md) · [Español](./docs/es/README.md) · [Русский](./docs/ru/README.md) · [العربية](./docs/ar/README.md)

## Introduction

kou is a standalone virtual-terminal engine — PTY management, a real VT100/ANSI
screen emulator, and screen rendering that actually draws glyphs. It is the vtty
core extracted from the tairitsu packager, hardened into a library and CLI of its
own.

Three things set it apart from a bare PTY wrapper:

- **A real screen.** The byte stream is run through the [`vte`](https://crates.io/crates/vte)
  parser, so CSI cursor moves, erase, scroll and the SGR 16-colour palette are
  honoured — not the "drop ESC on the floor" stub of the early prototype.
- **ort-style fonts.** kou does not ship fonts; it fetches a curated family
  (Fira Code / JetBrains Mono for Latin; Source Han Sans / Sarasa Mono / Smiley
  Sans for CJK) into a shared cache on first use, with mirror/proxy knobs for
  restrictive networks. Glyphs are rasterised with `ab_glyph`, Latin before CJK,
  so a single render mixes scripts without tofu.
- **Inband graphics.** A frame can be rasterised to PNG, or described to a
  capable terminal through the kitty (`kitty2`) or iTerm2 graphics protocol — so
  wezterm / kitty / iTerm2 / Ghostty render the real pixels inline.

## Quick Start

### CLI

```bash
# Launch a command in a virtual terminal and drive it from a REPL.
kou launch bash --cols 80 --rows 24
# > echo hello
# > screen        # prints the current screen text
```

### Library

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

## Graphics protocols

| `KOU_GRAPHICS` | Protocol | Terminals |
|----------------|----------|-----------|
| `kitty` / `kitty2` | kitty APC graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 inline image | iTerm2, wezterm |
| `sixel` | DCS sixel | (placeholder — needs a rasterizer) |
| `off` (default) | none — render a PNG out of band | all |

```rust
use kou::{FontCache, FontSet, GraphicsProtocol, VttyManager, render_graphics};
let frame = render_graphics(&screen, &FontCache::load(&FontSet::from_env(), 16.0), 16.0,
                            GraphicsProtocol::from_env());
if let Some(escape) = frame {
    print!("{escape}"); // capable terminals render the pixels inline
}
```

## Fonts & fetching

Pick the primary / CJK family with `KOU_FONT_PRIMARY` / `KOU_FONT_CJK`, or pin
files with `KOU_FONT_PATH` / `KOU_FONT_CJK_PATH`. Resolution order:
explicit path → shared cache → runtime download (the `font-fetch` feature,
enabled by default).

| Env | Purpose |
|-----|---------|
| `KOU_FONT_PRIMARY` | `fira-code` (default) / `jetbrains-mono` |
| `KOU_FONT_CJK` | `sarasa` (default) / `sourcehansans` / `smileysans` / `none` |
| `KOU_FONT_MIRROR` | Substitute the GitHub / jsDelivr host with a mirror. |
| `KOU_DOWNLOAD_PROXY` | Route font downloads through an http/https/socks proxy. |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | Per-request timeout (default 120). |
| `KOU_SKIP_FONT_FETCH` | Disable runtime fetching. |

## Development

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## License

SySL-1.0 (Synthetic Source License). See [LICENSE](./LICENSE).
