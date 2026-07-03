<p align="center"><img src="../logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>أتمتة الطرفية الافتراضية — PTY + شاشة VT100 + جلب الخطوط أثناء البناء + بروتوكولات رسومية داخل النطاق</strong></p>

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
[Русский](../ru/README.md) ·
**العربية**

</div>

## مقدمة

kou هو محرّك طرفية افتراضية مستقل — إدارة PTY، ومحاكٍ لشاشة VT100/ANSI، وعرض
للشاشة يرسم المحارف. وهو قلب vtty المستخرج من مُغلِّف tairitsu، والمُعزَّز ليكون
مكتبة وأداة سطر أوامر مستقلة بذاتها.

ثلاثة أمور تميّزه عن أي مُغلِّف PTY بسيط:

- **شاشة VT100.** يُمرَّر تدفّق البايتات عبر مُحلِّل [`vte`](https://crates.io/crates/vte)،
  بحيث تُحترَم تحركات المؤشر CSI، والمسح، والتمرير، ولوحة الألوان SGR ذات الـ16 لونًا —
  وليس مجرد «إسقاط رموز ESC» كما في النماذج الأولية المبكرة.
- **جلب الخطوط أثناء البناء.** يُحمِّل kou مُسبقًا خطًّا واحدًا لكل كتابة إلى
  ذاكرة تخزين مؤقت مشتركة عند وقت البناء. تجاوز العائلات أو ثبّت ملفات محلية
  عبر متغيرات البيئة؛ وجّه التنزيلات عبر وسيط HTTP(S) عند التواجد خلف شبكة
  مقيَّدة. طالع [الخطوط والجلب](#الخطوط-والجلب) للقائمة الكاملة.
- **رسوميات داخل النطاق.** يمكن تنقيط الإطار إلى PNG، أو وصفه لطرفية قادرة على ذلك
  عبر بروتوكول kitty (`kitty2`) أو iTerm2 الرسومي — بحيث تعرض wezterm / kitty /
  iTerm2 / Ghostty البكسلات الحقيقية في موضعها.

## البدء السريع

### CLI

```bash
# Launch a command in a virtual terminal and drive it from a REPL.
kou launch bash --cols 80 --rows 24
# > echo hello
# > screen        # prints the current screen text
```

### المكتبة

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

## بروتوكولات الرسوميات

| `KOU_GRAPHICS` | البروتوكول | الطرفيات |
|----------------|------------|----------|
| `kitty` / `kitty2` | kitty APC للرسوميات | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 صورة مضمنة | iTerm2, wezterm |
| `sixel` | DCS sixel | (عنصر نائب — يحتاج إلى منقِّط) |
| `off` (افتراضي) | لا شيء — تصيير PNG خارج النطاق | الكل |

```rust
use kou::{FontCache, FontSet, GraphicsProtocol, VttyManager, render_graphics};
let frame = render_graphics(&screen, &FontCache::load(&FontSet::from_env(), 16.0), 16.0,
                            GraphicsProtocol::from_env());
if let Some(escape) = frame {
    print!("{escape}"); // capable terminals render the pixels inline
}
```

## الخطوط والجلب

يُحمِّل kou مُسبقًا خطًّا واحدًا لكل كتابة إلى ذاكرة تخزين مؤقت مشتركة عند وقت البناء:

| الكتابة | الخط |
|---------|------|
| Latin | Fira Code |
| CJK (中文 · 日本語 · 한국어) | Source Han Sans SC (思源黑体) |
| Arabic | Noto Naskh Arabic |
| Devanagari (हिन्दी · मराठी) | Noto Sans Devanagari |
| Thai (ไทย) | Noto Sans Thai |

تجاوز أي عائلة عند وقت البناء باستخدام `KOU_FONT_PRIMARY` / `KOU_FONT_CJK` /
`KOU_FONT_ARABIC` / `KOU_FONT_DEVANAGARI` / `KOU_FONT_THAI`، أو ثبّت ملفات محلية
عبر `KOU_FONT_*_PATH`. يمكن توجيه التنزيلات عبر وسيط HTTP(S) عبر
`KOU_DOWNLOAD_PROXY` (يُمرَّر مباشرة إلى reqwest).

| متغير البيئة | الغرض |
|--------------|-------|
| `KOU_FONT_PRIMARY` | تجاوز عائلة الخط اللاتيني. |
| `KOU_FONT_CJK` | تجاوز / تعطيل خط CJK (`none` للتعطيل). |
| `KOU_FONT_ARABIC` | تجاوز / تعطيل خط العربي. |
| `KOU_FONT_DEVANAGARI` | تجاوز / تعطيل خط الديفاناغارية. |
| `KOU_FONT_THAI` | تجاوز / تعطيل خط التايلاندية. |
| `KOU_FONT_MIRROR` | استبدال مضيف التنزيل بمرآة. |
| `KOU_DOWNLOAD_PROXY` | توجيه التنزيلات عبر وسيط HTTP(S) (reqwest). |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | مهلة لكل طلب (افتراضي 120). |
| `KOU_SKIP_FONT_FETCH` | تعطيل الجلب. |

## التطوير

```bash
cargo check --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## الترخيص

SySL-1.0 (Synthetic Source License). راجع [LICENSE](../../LICENSE).
