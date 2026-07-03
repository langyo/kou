<p align="center"><img src="../logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>Automatización de terminal virtual — PTY + una pantalla VT100 real + tipografía estilo ort + protocolos gráficos in-band.</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](../../LICENSE)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/kou/checks.yml)](https://github.com/celestia-island/kou/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-kou.docs.celestia.world-blue)](https://kou.docs.celestia.world)

</div>

<div align="center">

[English](../en/README.md) ·
[简体中文](../zhs/README.md) ·
[繁體中文](../zht/README.md) ·
[日本語](../ja/README.md) ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
**Español** ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## Introducción

kou es un motor de terminal virtual autónomo — gestión de PTY, un emulador de
pantalla VT100/ANSI real, y renderizado de pantalla que realmente dibuja los
glifos. Es el núcleo vtty extraído del empaquetador tairitsu, reforzado como
librería y CLI propios.

Tres cosas lo distinguen de un simple envoltorio de PTY:

- **Una pantalla real.** El flujo de bytes pasa por el analizador [`vte`](https://crates.io/crates/vte),
  por lo que los movimientos CSI del cursor, el borrado, el desplazamiento y la
  paleta SGR de 16 colores se respetan — no es el stub de "tirar ESC al suelo"
  del primer prototipo.
- **Tipografía estilo ort.** kou no incluye fuentes; obtiene una familia
  seleccionada (Fira Code / JetBrains Mono para latín; Source Han Sans / Sarasa
  Mono / Smiley Sans para CJK) en una caché compartida en el primer uso, con
  opciones de mirror/proxy para redes restrictivas. Los glifos se rasterizan con
  `ab_glyph`, latín antes de CJK, de modo que un solo renderizado mezcla
  escrituras sin tofu.
- **Gráficos in-band.** Un fotograma puede rasterizarse a PNG o describirse a
  un terminal compatible mediante el protocolo gráfico kitty (`kitty2`) o
  iTerm2 — así wezterm / kitty / iTerm2 / Ghostty renderizan los píxeles
  reales en línea.

## Inicio rápido

### CLI

```bash
# Launch a command in a virtual terminal and drive it from a REPL.
kou launch bash --cols 80 --rows 24
# > echo hello
# > screen        # prints the current screen text
```

### Librería

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

## Protocolos gráficos

| `KOU_GRAPHICS` | Protocolo | Terminales |
|----------------|-----------|------------|
| `kitty` / `kitty2` | kitty APC graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 imagen en línea | iTerm2, wezterm |
| `sixel` | DCS sixel | (marcador de posición — necesita un rasterizador) |
| `off` (predeterminado) | ninguno — renderiza un PNG fuera de banda | todos |

```rust
use kou::{FontCache, FontSet, GraphicsProtocol, VttyManager, render_graphics};
let frame = render_graphics(&screen, &FontCache::load(&FontSet::from_env(), 16.0), 16.0,
                            GraphicsProtocol::from_env());
if let Some(escape) = frame {
    print!("{escape}"); // capable terminals render the pixels inline
}
```

## Fuentes y obtención

Elige la familia primaria / CJK con `KOU_FONT_PRIMARY` / `KOU_FONT_CJK`, o fija
archivos con `KOU_FONT_PATH` / `KOU_FONT_CJK_PATH`. Orden de resolución:
ruta explícita → caché compartida → descarga en tiempo de ejecución (la
característica `font-fetch`, habilitada por defecto).

| Variable de entorno | Propósito |
|---------------------|-----------|
| `KOU_FONT_PRIMARY` | `fira-code` (predeterminado) / `jetbrains-mono` |
| `KOU_FONT_CJK` | `sarasa` (predeterminado) / `sourcehansans` / `smileysans` / `none` |
| `KOU_FONT_MIRROR` | Sustituye el host de GitHub / jsDelivr por un mirror. |
| `KOU_DOWNLOAD_PROXY` | Enruta las descargas de fuentes a través de un proxy http/https/socks. |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | Tiempo de espera por solicitud (predeterminado 120). |
| `KOU_SKIP_FONT_FETCH` | Deshabilita la obtención en tiempo de ejecución. |

## Desarrollo

```bash
cargo check --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Licencia

SySL-1.0 (Synthetic Source License). Consulta [LICENSE](../../LICENSE).
