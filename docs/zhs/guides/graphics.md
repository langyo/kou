# 图形协议

在文本网格之上，kou 可以向兼容的终端描述渲染后的帧，使其在行内绘制实际像素。使用 `KOU_GRAPHICS` 选择一种：

| 值 | 协议 | 终端 |
|-------|----------|-----------|
| `kitty` / `kitty2` | kitty APC `\e_G` graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 inline image | iTerm2, wezterm |
| `sixel` | DCS sixel | 占位符 — 需要光栅化器 |
| `off`（默认） | 无 — 在带外生成 PNG | 全部 |

## 编码方式

- **Kitty。** PNG 以 base64 编码，并在 `ESC G … ST` 帧内以 ≤4096 字节的块流式传输，携带控制载荷 `a=T,t=d,f=100,s=<w>,v=<h>,c=<cols>,r=<rows>`。Kitty 及其兼容终端将其放置在指定单元格区域的光标位置。
- **iTerm2。** 单个序列 `ESC ]1337;File=inline=1;width=<w>cells;height=<h>cells;size=<n>;name=<b64>:<b64-png>BEL`。

Sixel 已建模但未编码：生成真正的 sixel 流需要光栅化器（调色板量化 + sixel 压缩）。需要 sixel 的调用者应将 PNG 交给专用编码器；`GraphicsProtocol::supported()` 会如实报告这一情况。

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
