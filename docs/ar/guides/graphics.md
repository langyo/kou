# بروتوكولات الرسوميات

بالإضافة إلى شبكة النص، يمكن لـ kou وصف إطار مُنشأ إلى محطة طرفية قادرة بحيث ترسم البيكسلات الفعلية بشكل مضمن. اختر واحدًا باستخدام `KOU_GRAPHICS`:

| القيمة | البروتوكول | المحطات الطرفية |
|-------|----------|-----------|
| `kitty` / `kitty2` | kitty APC `\e_G` graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 inline image | iTerm2, wezterm |
| `sixel` | DCS sixel | عنصر نائب — يحتاج إلى مكوّن تنقيط |
| `off` (افتراضي) | لا شيء — يُنشئ PNG خارج النطاق | الكل |

## كيفية الترميز

- **Kitty.** يتم ترميز PNG بـ base64 وإرساله في أجزاء لا تتجاوز 4096 بايت داخل إطارات `ESC G … ST`، حاملاً حمولة التحكم `a=T,t=d,f=100,s=<w>,v=<h>,c=<cols>,r=<rows>`. تضعها Kitty والبرامج المتوافقة عند المؤشر فوق مساحة الخلايا المحددة.
- **iTerm2.** تسلسل واحد `ESC ]1337;File=inline=1;width=<w>cells;height=<h>cells;size=<n>;name=<b64>:<b64-png>BEL`.

تم نمذجة Sixel ولكن لم يتم ترميزه بعد: إنتاج تيار sixel حقيقي يحتاج إلى مكوّن تنقيط (تكميم اللوحة + ضغط sixel). على المستخدمين الراغبين في sixel تسليم PNG إلى مُرمِّز مخصص؛ `GraphicsProtocol::supported()` تُبلغ عن ذلك بصدق.

## كيفية الاستخدام

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
