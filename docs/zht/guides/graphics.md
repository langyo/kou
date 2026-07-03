# 圖形協定

在文字網格之上，kou 可以向相容的終端描述渲染後的幀，使其在行內繪製實際像素。使用 `KOU_GRAPHICS` 選擇一種：

| 值 | 協定 | 終端 |
|-------|----------|-----------|
| `kitty` / `kitty2` | kitty APC `\e_G` graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 inline image | iTerm2, wezterm |
| `sixel` | DCS sixel | 佔位符 — 需要光柵化器 |
| `off`（預設） | 無 — 在帶外生成 PNG | 全部 |

## 編碼方式

- **Kitty。** PNG 以 base64 編碼，並在 `ESC G … ST` 幀內以 ≤4096 位元組的區塊串流傳輸，攜帶控制載荷 `a=T,t=d,f=100,s=<w>,v=<h>,c=<cols>,r=<rows>`。Kitty 及其相容終端將其放置在指定儲存格區域的游標位置。
- **iTerm2。** 單個序列 `ESC ]1337;File=inline=1;width=<w>cells;height=<h>cells;size=<n>;name=<b64>:<b64-png>BEL`。

Sixel 已建模但未編碼：產生真正的 sixel 串流需要光柵化器（調色板量化 + sixel 壓縮）。需要 sixel 的呼叫者應將 PNG 交給專用編碼器；`GraphicsProtocol::supported()` 會如實報告此情況。

## 使用方式

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
