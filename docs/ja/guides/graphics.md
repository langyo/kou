# グラフィックスプロトコル

テキストグリッドに加えて、kou は対応端末に対してレンダリング済みフレームを記述し、実際のピクセルをインラインで描画させることができます。`KOU_GRAPHICS` で選択してください：

| 値 | プロトコル | 対応端末 |
|-------|----------|-----------|
| `kitty` / `kitty2` | kitty APC `\e_G` graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 inline image | iTerm2, wezterm |
| `sixel` | DCS sixel | プレースホルダー — ラスタライザが必要 |
| `off` (デフォルト) | なし — 帯域外で PNG を生成 | すべて |

## エンコード方法

- **Kitty.** PNG は base64 エンコードされ、制御ペイロード `a=T,t=d,f=100,s=<w>,v=<h>,c=<cols>,r=<rows>` を含む `ESC G … ST` フレーム内で ≤4096 バイトのチャンクに分割してストリーミングされます。Kitty および互換端末は、指定されたセル領域のカーソル位置に画像を配置します。
- **iTerm2.** 単一のシーケンス `ESC ]1337;File=inline=1;width=<w>cells;height=<h>cells;size=<n>;name=<b64>:<b64-png>BEL`。

Sixel はモデル化されていますがエンコードはされていません：実際の sixel ストリームを生成するにはラスタライザ（パレット量子化 + sixel 圧縮）が必要です。Sixel を利用したい場合は PNG を専用のエンコーダに渡してください；`GraphicsProtocol::supported()` はこの状況を正直に報告します。

## 使い方

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
