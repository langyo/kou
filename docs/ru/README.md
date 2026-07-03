<p align="center"><img src="../logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>Автоматизация виртуального терминала — PTY + настоящий экран VT100 + получение шрифтов во время сборки + встроенные графические протоколы</strong></p>

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
[Español](../es/README.md) ·
**Русский** ·
[العربية](../ar/README.md)

</div>

## Введение

kou — это автономный движок виртуального терминала: управление PTY, настоящий
эмулятор экрана VT100/ANSI и отрисовка экрана с прорисовкой глифов. Это ядро
vtty, выделенное из упаковщика tairitsu и оформленное в виде самостоятельной
библиотеки и CLI.

Три особенности отличают его от простой обёртки над PTY:

- **Настоящий экран.** Поток байтов обрабатывается парсером [`vte`](https://crates.io/crates/vte),
  поэтому перемещения курсора CSI, стирание, прокрутка и 16-цветная палитра SGR
  поддерживаются полноценно — а не отбрасываются, как в упрощённой заглушке
  раннего прототипа.
- **Получение шрифтов во время сборки.** kou не поставляется со шрифтами; он загружает
  подобранное семейство (Fira Code / JetBrains Mono для латиницы; Source Han
  Sans / Sarasa Mono / Smiley Sans для CJK) в общий кеш при первом
  использовании, с возможностью настройки зеркала/прокси для сетей с
  ограничениями. Глифы растеризуются с помощью `ab_glyph`, сначала латиница,
  затем CJK, что позволяет смешивать письменности в одном рендере без «тофу».
- **Встроенная графика.** Кадр может быть растеризован в PNG или передан
  совместимому терминалу через графический протокол kitty (`kitty2`) или
  iTerm2 — таким образом wezterm / kitty / iTerm2 / Ghostty отображают
  пиксели непосредственно в потоке.

## Быстрый старт

### CLI

```bash
# Launch a command in a virtual terminal and drive it from a REPL.
kou launch bash --cols 80 --rows 24
# > echo hello
# > screen        # prints the current screen text
```

### Библиотека

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

## Графические протоколы

| `KOU_GRAPHICS` | Протокол | Терминалы |
|----------------|----------|-----------|
| `kitty` / `kitty2` | kitty APC graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 inline image | iTerm2, wezterm |
| `sixel` | DCS sixel | (заглушка — требуется растеризатор) |
| `off` (по умолчанию) | нет — рендер PNG вне потока | все |

```rust
use kou::{FontCache, FontSet, GraphicsProtocol, VttyManager, render_graphics};
let frame = render_graphics(&screen, &FontCache::load(&FontSet::from_env(), 16.0), 16.0,
                            GraphicsProtocol::from_env());
if let Some(escape) = frame {
    print!("{escape}"); // capable terminals render the pixels inline
}
```

## Шрифты и загрузка

kou не поставляется со шрифтами — он загружает подобранное семейство в общий
кеш во время сборки, с возможностью настройки зеркала/прокси для сетей с
ограничениями. Каждая письменность выбирает **один** шрифт; значения по
умолчанию и альтернативы:

| Письменность | По умолчанию | Альтернативы |
|--------------|--------------|--------------|
| Latin | Fira Code | JetBrains Mono |
| CJK | Source Han Sans SC (思源黑体) | Sarasa Mono SC (更纱黑体), Smiley Sans (得意黑), `none` |

Выберите основное / CJK семейство с помощью `KOU_FONT_PRIMARY` /
`KOU_FONT_CJK` или укажите файлы через `KOU_FONT_PATH` /
`KOU_FONT_CJK_PATH`. Порядок разрешения: явный путь → общий кеш → загрузка
во время выполнения (функция `font-fetch`, включена по умолчанию).

| Переменная | Назначение |
|------------|------------|
| `KOU_FONT_PRIMARY` | `fira-code` (по умолчанию) / `jetbrains-mono` |
| `KOU_FONT_CJK` | `sourcehansans` (по умолчанию) / `sarasa` / `smileysans` / `none` |
| `KOU_FONT_MIRROR` | Замена хоста GitHub / jsDelivr на зеркало. |
| `KOU_DOWNLOAD_PROXY` | Маршрутизация загрузки шрифтов через http/https/socks прокси. |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | Таймаут одного запроса (по умолчанию 120). |
| `KOU_SKIP_FONT_FETCH` | Отключить загрузку во время выполнения. |

## Разработка

```bash
cargo check --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Лицензия

SySL-1.0 (Synthetic Source License). См. [LICENSE](../../LICENSE).
