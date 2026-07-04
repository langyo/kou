# Графические протоколы

Поверх текстовой сетки kou может описать отрисованный кадр совместимому терминалу, чтобы тот отобразил реальные пиксели в строке. Выберите протокол с помощью `KOU_GRAPHICS`:

| Значение | Протокол | Терминалы |
|-------|----------|-----------|
| `kitty` / `kitty2` | kitty APC `\e_G` graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 inline image | iTerm2, wezterm |
| `sixel` | DCS sixel | заглушка — требуется растеризатор |
| `off` (по умолчанию) | нет — создаёт PNG вне полосы | все |

## Как это кодируется

- **Kitty.** PNG кодируется в base64 и передаётся потоками чанками ≤4096 байт внутри кадров `ESC G … ST`, несущих управляющую нагрузку `a=T,t=d,f=100,s=<w>,v=<h>,c=<cols>,r=<rows>`. Kitty и его клоны размещают изображение по позиции курсора в указанной области ячеек.
- **iTerm2.** Одна последовательность `ESC ]1337;File=inline=1;width=<w>cells;height=<h>cells;size=<n>;name=<b64>:<b64-png>BEL`.

Sixel смоделирован, но не закодирован: для создания реального sixel-потока требуется растеризатор (квантование палитры + sixel-сжатие). Желающим использовать sixel следует передать PNG специализированному кодировщику; `GraphicsProtocol::supported()` честно сообщает об этом.

## Как использовать

```rust
use kou::{FontCache, FontSet, GraphicsProtocol, VttyManager, render_graphics};

let screen = mgr.screen(&id).await?;
let fonts = FontCache::load(&FontSet::from_env(), 16.0);
if let Some(escape) = render_graphics(&screen, &fonts, 16.0, GraphicsProtocol::from_env()) {
    print!("{escape}"); // capable terminals render the pixels inline
    println!();         // advance past the placement
} else {
    // Protocol off or unsupported: fall back to a PNG.
    let png = kou::render_png(&screen, &fonts, 16.0)?;
    std::fs::write("screen.png", png)?;
}
```
