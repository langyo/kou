# Protocolos Gráficos

Sobre la cuadrícula de texto, kou puede describir un fotograma renderizado a una terminal capaz para que dibuje los píxeles reales en línea. Selecciona uno con `KOU_GRAPHICS`:

| Valor | Protocolo | Terminales |
|-------|----------|-----------|
| `kitty` / `kitty2` | kitty APC `\e_G` graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 inline image | iTerm2, wezterm |
| `sixel` | DCS sixel | marcador de posición — necesita un rasterizador |
| `off` (predeterminado) | ninguno — genera un PNG fuera de banda | todos |

## Cómo se codifica

- **Kitty.** El PNG se codifica en base64 y se transmite en fragmentos de ≤4096 bytes dentro de tramas `ESC G … ST`, llevando la carga de control `a=T,t=d,f=100,s=<w>,v=<h>,c=<cols>,r=<rows>`. Kitty y sus clones lo colocan en el cursor sobre el área de celdas indicada.
- **iTerm2.** Una única secuencia `ESC ]1337;File=inline=1;width=<w>cells;height=<h>cells;size=<n>;name=<b64>:<b64-png>BEL`.

Sixel está modelado pero no codificado: producir un flujo sixel real necesita un rasterizador (cuantización de paleta + compresión sixel). Quienes deseen sixel deben pasar el PNG a un codificador dedicado; `GraphicsProtocol::supported()` lo informa honestamente.

## Cómo usarlo

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
