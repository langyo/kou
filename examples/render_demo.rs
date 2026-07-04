//! Render a representative sample screen to PNG for visual QA.
//!
//! Run: `cargo run --example render_demo` (uses common system fonts when the
//! ort-style fetch is unavailable). Writes:
//!   /tmp/kou_render_demo.png      (3× supersampled — the crisp one)
//!   /tmp/kou_render_demo_1x.png   (1× reference)

use kou::{FontCache, FontSet, Screen, render_png_supersampled, theme_by_name};

fn main() -> anyhow::Result<()> {
    let paths = locate_system_fonts();

    // 3× supersampled render (load the cache at 3× the output px).
    let fonts_hi = FontCache::from_paths(&paths, 16.0 * 3.0);
    let screen = sample_screen();
    let theme = theme_by_name("Campbell");
    let png = render_png_supersampled(&screen, &fonts_hi, 16.0, 3, theme)?;
    std::fs::write("/tmp/kou_render_demo.png", &png)?;
    println!(
        "wrote /tmp/kou_render_demo.png ({} bytes, {} faces, sample=SystemError)",
        png.len(),
        fonts_hi.len()
    );

    // 1× reference for comparison.
    let fonts_lo = FontCache::from_paths(&paths, 16.0);
    let png1 = render_png_supersampled(&screen, &fonts_lo, 16.0, 1, theme)?;
    std::fs::write("/tmp/kou_render_demo_1x.png", &png1)?;
    println!("wrote /tmp/kou_render_demo_1x.png ({} bytes)", png1.len());

    // Silence unused warning when the fetch set isn't used.
    let _ = FontSet::from_env();
    Ok(())
}

fn locate_system_fonts() -> Vec<&'static std::path::Path> {
    let mut out: Vec<&'static std::path::Path> = Vec::new();
    // Latin monospace with full box-drawing coverage.
    for p in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/hack/Hack-Regular.ttf",
        "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
    ] {
        let path = std::path::Path::new(p);
        if path.exists() {
            out.push(path);
            break;
        }
    }
    // CJK fallback.
    for p in [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansSC-Regular.otf",
    ] {
        let path = std::path::Path::new(p);
        if path.exists() {
            out.push(path);
            break;
        }
    }
    out
}

fn sample_screen() -> Screen {
    // A 40×12 screen that exercises: box-drawing border, headings, an aligned
    // two-column table, colour, and CJK.
    let mut s = Screen::new(40, 12);
    let frame = "\
┌──────────────────────────────────────┐
│  kou render demo  ── 终端渲染演示    │
├──────────────────────────────────────┤
│  Name      Status     Notes          │
│  alpha     OK         started        │
│  bravo     WARN       standby        │
│  charlie   ERR        crashed        │
├──────────────────────────────────────┤
│  > box-drawing should connect        │
│  progress  [████████░░░░░░░░] 50%    │
└──────────────────────────────────────┘";
    s.feed(frame.as_bytes());

    // Colour the status cells: green OK, yellow WARN, red ERR.
    s.feed(b"\x1b[4;14H\x1b[32mOK\x1b[0m");
    s.feed(b"\x1b[5;14H\x1b[33mWARN\x1b[0m");
    s.feed(b"\x1b[6;14H\x1b[31mERR\x1b[0m");
    s
}
