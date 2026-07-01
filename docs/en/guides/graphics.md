# Graphics Protocols

On top of the text grid, kou can describe a rendered frame to a capable
terminal so it draws the actual pixels inline. Select one with
`KOU_GRAPHICS`:

| Value | Protocol | Terminals |
|-------|----------|-----------|
| `kitty` / `kitty2` | kitty APC `\e_G` graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 inline image | iTerm2, wezterm |
| `sixel` | DCS sixel | placeholder — needs a rasterizer |
| `off` (default) | none — render a PNG out of band | all |

## How it is encoded

- **Kitty.** The PNG is base64-encoded and streamed in ≤4096-byte chunks inside
  `ESC G … ST` frames, carrying the control payload `a=T,t=d,f=100,s=<w>,v=<h>,
  c=<cols>,r=<rows>`. Kitty and clones place it at the cursor over the given
  cell area.
- **iTerm2.** A single `ESC ]1337;File=inline=1;width=<w>cells;height=<h>cells;
  size=<n>;name=<b64>:<b64-png>BEL` sequence.

Sixel is modelled but not encoded: producing a real sixel stream needs a
rasterizer (palette quantisation + sixel compression). Callers wanting sixel
should hand the PNG to a dedicated encoder; `GraphicsProtocol::supported()`
reports this honestly.

## Driving it

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
