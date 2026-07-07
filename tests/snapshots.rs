//! Snapshot regression tests: feed raw ANSI text to a `Screen`, render to PNG,
//! and compare against a committed baseline under `res/`.
//!
//! # Why pixel-diff, not just "render and eyeball"
//!
//! These baselines are a **rendering regression alarm**. If a code change
//! (a new VT100 parser path, a render refactor, a font-metrics shift) silently
//! breaks what used to render correctly, the per-pixel comparison fails and
//! points at the culprit change. Two ways to resolve a failure:
//!
//! 1. **It's a real regression** → fix the code until the render matches again.
//! 2. **It's an intended change** (you deliberately altered rendering) → accept
//!    the new image as the baseline:
//!    ```bash
//!    KOU_ACCEPT_SNAPSHOTS=1 cargo test --test snapshots
//!    ```
//!    then commit the updated `res/*.png`.
//!
//! # Font determinism (the whole scheme hinges on this)
//!
//! The renderer is pixel-deterministic *given the same fonts* — `ab_glyph`
//! rasterisation and Lanczos3 downscale have no hidden state. The only source
//! of cross-machine drift is the **font files**. So tests MUST NOT use
//! `FontCache::from_system_fonts` (which picks Consolas on Windows, DejaVu on
//! Linux, Menlo on macOS — guaranteeing spurious diffs).
//!
//! Instead [`fonts`] loads a fixed `FontSet` (FiraCode primary + Source Han Sans
//! SC fallback) via the same resolution path the library uses in production
//! (`KOU_FONT_PATH` → kou cache → fetch). Tests run with `KOU_SKIP_FONT_FETCH=1`
//! so they never hit the network; the cache must be pre-populated:
//!   - locally: run any kou example once, or point `KOU_FONT_PATH`/`KOU_FONT_CJK_PATH`
//!     at the two files;
//!   - in CI: the workflow downloads a pinned `fonts-v1` asset from the GitHub
//!     Release into the kou cache before `cargo test`.

use base64::Engine as _;
use image::RgbaImage;
use kou::{FontCache, FontSet, Screen, theme_by_name};

/// Maximum fraction of pixels allowed to differ before a snapshot is flagged
/// as a regression. 0.1% tolerates PNG re-encoding noise while still catching
/// any meaningful glyph/layout change.
const DIFF_THRESHOLD: f64 = 0.001;

/// Load the **fixed** font set used by every snapshot. See the module docs for
/// why this must not use system fonts.
fn fonts() -> FontCache {
    FontCache::load(&FontSet::from_env(), 32.0 * 3.0)
}

/// Render `screen` in `theme` and compare against the baseline `res/{name}.png`.
///
/// - With `KOU_ACCEPT_SNAPSHOTS` set, the rendered image replaces the baseline
///   (use this to bless an intentional rendering change).
/// - Otherwise the two are diffed pixel-by-pixel; differing beyond
///   [`DIFF_THRESHOLD`] fails the test with guidance on how to accept.
fn assert_matches(screen: &Screen, theme: &str, name: &str) {
    let png = kou::render::render_png_supersampled(screen, &fonts(), 32.0, 3, theme_by_name(theme))
        .expect("render");
    let baseline = std::path::Path::new("res").join(format!("{name}.png"));

    if std::env::var_os("KOU_ACCEPT_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(baseline.parent().unwrap()).unwrap();
        std::fs::write(&baseline, &png).unwrap();
        eprintln!("  accepted {baseline:?}");
        return;
    }

    let new = image::load_from_memory(&png)
        .expect("decode rendered png")
        .to_rgba8();
    let base_bytes = std::fs::read(&baseline).unwrap_or_else(|e| {
        panic!(
            "baseline {baseline:?} missing ({e}); generate it with \
             `KOU_ACCEPT_SNAPSHOTS=1 cargo test --test snapshots`"
        )
    });
    let base = image::load_from_memory(&base_bytes)
        .expect("decode baseline png")
        .to_rgba8();

    assert_eq!(
        base.dimensions(),
        new.dimensions(),
        "{name}: dimensions changed ({}x{} -> {}x{}) — screen geometry or font metrics drifted",
        base.width(),
        base.height(),
        new.width(),
        new.height(),
    );

    let (ratio, max_delta) = pixel_diff(&base, &new);
    let pct = format!("{:.3}%", ratio * 100.0);
    assert!(
        ratio < DIFF_THRESHOLD,
        "{name}: {pct} of pixels differ (max channel delta {max_delta}) — \
         rendering regression? If this change is intended, bless it with \
         `KOU_ACCEPT_SNAPSHOTS=1 cargo test --test snapshots {name}`",
    );
}

/// Fraction of differing pixels and the largest single-channel delta between
/// two equally-sized RGBA images. A pixel "differs" when any channel differs
/// by more than a small tolerance (anti-aliasing edges can shift by ±1).
fn pixel_diff(base: &RgbaImage, other: &RgbaImage) -> (f64, u8) {
    debug_assert_eq!(base.dimensions(), other.dimensions());
    let (w, h) = base.dimensions();
    let total = (w as u64) * (h as u64);
    let mut differing = 0u64;
    let mut max_delta = 0u8;
    for (a, b) in base.pixels().zip(other.pixels()) {
        let dr = a[0].abs_diff(b[0]);
        let dg = a[1].abs_diff(b[1]);
        let db = a[2].abs_diff(b[2]);
        let da = a[3].abs_diff(b[3]);
        let local = dr.max(dg).max(db).max(da);
        max_delta = max_delta.max(local);
        // Ignore near-zero shifts (≤2 per channel) so 1px anti-alias jitter
        // doesn't trip the alarm.
        if local > 2 {
            differing += 1;
        }
    }
    (differing as f64 / total as f64, max_delta)
}

// ── 1. Neofetch showcase (120×60) ───────────────────────────────

#[test]
fn neofetch_showcase() {
    let mut sc = Screen::new(120, 60);
    let art = [
        "           ###########           ",
        "          ##         ##          ",
        "         ##           ##         ",
        "        ##             ##        ",
        "       ##               ##       ",
        "      ##    #  kou  #    ##      ",
        "     ##                   ##     ",
        "    ##                     ##    ",
        "    ##                     ##    ",
        "    ##                     ##    ",
        "     ##                   ##     ",
        "      ##                 ##      ",
        "       ##               ##       ",
        "        ##             ##        ",
        "         ##           ##         ",
        "          ##         ##          ",
        "           ###########           ",
    ];
    let info: [(&str, &str); 14] = [
        ("OS", "kou VTty Engine v0.1"),
        ("Host", "PTY + VT100 terminal emulator"),
        ("Kernel", "vte 0.13 / ab_glyph / image 0.25"),
        ("Shell", "kou::VttyManager"),
        ("Resolution", "120 columns × 60 rows"),
        ("Themes", "15 Windows Terminal schemes"),
        ("Fonts", "FiraCode · Source Han Sans SC · pinned"),
        ("Protocol", "Kitty2 (APC) + iTerm2 (OSC 1337) + Sixel (DCS)"),
        ("Render", "Lanczos3 supersampled 3× · contain-fit images"),
        (
            "Font fetch",
            "Async reqwest + OS font discovery (recursive)",
        ),
        ("CJK", "简体中文 · 日本語 · 한국어 — full-width OK"),
        ("PTY", "portable-pty (Unix forkpty / Windows ConPTY)"),
        ("Scrollback", "1000-line ring buffer"),
        ("Uptime", "since you started reading this"),
    ];

    let line0 = format!(
        "\x1b[1;36m{:<36}\x1b[0m  \x1b[1;37m{}\x1b[0m\x1b[2;37m@kou\x1b[0m\n",
        art[0], "langyo"
    );
    sc.feed(line0.as_bytes());
    for i in 0..info.len() {
        let al = if i + 1 < art.len() { art[i + 1] } else { "" };
        sc.feed(
            format!(
                "\x1b[1;36m{:<36}\x1b[0m  \x1b[1;33m{:<14}\x1b[0m \x1b[0;37m{}\n",
                al, info[i].0, info[i].1,
            )
            .as_bytes(),
        );
    }
    // Two rows of color palette
    sc.feed(b"\n  ");
    for c in 0u8..8 {
        sc.feed(format!("\x1b[4{}m    \x1b[0m", c).as_bytes());
    }
    sc.feed(b"\n  ");
    for c in 8u8..16 {
        sc.feed(format!("\x1b[4{}m    \x1b[0m", c).as_bytes());
    }
    sc.feed(b"\n");

    assert_matches(&sc, "solarized-dark", "neofetch_solarized_dark");
    assert_matches(&sc, "campbell", "neofetch_campbell");
    assert_matches(&sc, "one-half-dark", "neofetch_one_half_dark");
}

// ── 2. Rainbow gradient (80×40) ─────────────────────────────────

#[test]
fn rainbow_gradient() {
    let mut sc = Screen::new(80, 40);
    let colors = [
        31u8, 31, 33, 33, 32, 32, 36, 36, 34, 34, 35, 35, 31, 33, 32, 36, 34, 35,
    ];
    sc.feed(b"\n\n");
    for (row, &color) in colors.iter().enumerate() {
        let pad = " ".repeat(row * 2 + 10);
        let blocks = "#".repeat(row * 3 + 1);
        sc.feed(format!("{}\x1b[1;{}m{}\x1b[0m\n", pad, color, blocks).as_bytes());
    }
    sc.feed(
        "\n\n                  \x1b[2;37mkou — VTty engine · rainbow gradient test\x1b[0m\n"
            .as_bytes(),
    );
    assert_matches(&sc, "campbell", "rainbow_gradient_campbell");
}

// ── 3. Protocol comparison table (120×30) ───────────────────────

#[test]
fn protocol_comparison() {
    let mut sc = Screen::new(120, 30);
    sc.feed(
        "\x1b[1;37m\x1b[44m\x1b[K  Inline Image Protocols — Feature Comparison  \x1b[0m\n\n"
            .as_bytes(),
    );
    let rows: &[(&str, &str, &str, &str); 10] = &[
        (
            "Feature",
            "Kitty2 (APC)",
            "iTerm2 (OSC 1337)",
            "Sixel (DCS)",
        ),
        (
            "Direction",
            "Encode + Decode",
            "Encode + Decode",
            "Encode + Decode",
        ),
        ("Image fmt", "base64 PNG", "base64 PNG", "Raster language"),
        (
            "Chunking",
            "4096-byte chunks",
            "Single frame",
            "RLE compressed",
        ),
        (
            "Cross-feed",
            "Sliding apc_buf",
            "Sliding apc_buf",
            "Sliding apc_buf",
        ),
        (
            "Aspect",
            "Contain-fit centred",
            "Contain-fit centred",
            "Pixel-fit",
        ),
        (
            "Max size",
            "Unlimited (chunked)",
            "~512 KB (single frame)",
            "Unlimited (RLE)",
        ),
        ("Dependency", "Built-in", "Built-in", "icy_sixel crate"),
        ("Cargo feat", "default", "default", "sixel"),
        (
            "Terminals",
            "kitty, wezterm, ghostty",
            "iTerm2, wezterm",
            "xterm, mlterm, mintty",
        ),
    ];
    for (i, (label, k, it, sx)) in rows.iter().enumerate() {
        let (lc, kc, ic, xc) = if i == 0 {
            ("1;37", "1;36", "1;33", "1;35")
        } else {
            ("0;37", "36", "33", "35")
        };
        sc.feed(format!(
            "  \x1b[{lc}m{:<14}\x1b[0m│ \x1b[{kc}m{:<24}\x1b[0m│ \x1b[{ic}m{:<24}\x1b[0m│ \x1b[{xc}m{}\x1b[0m\n",
            label, k, it, sx, lc=lc, kc=kc, ic=ic, xc=xc,
        ).as_bytes());
    }
    assert_matches(&sc, "one-half-dark", "protocol_comparison");
}

// ── 4. Inline image rendering (kitty2) — logo via APC ───────────

#[test]
fn inline_image_kitty2() {
    let mut sc = Screen::new(120, 60);
    sc.feed(
        "\x1b[1;37m\x1b[44m\x1b[K  Inline Image Rendering — Kitty2 APC Protocol  \x1b[0m\n\n"
            .as_bytes(),
    );

    // Load the kou logo, build a kitty APC, feed it.
    let logo_png = std::fs::read("docs/logo.webp")
        .ok()
        .and_then(|webp| {
            // webp → png via image crate
            let img = image::load_from_memory(&webp).ok()?;
            let mut buf = Vec::new();
            use image::ImageEncoder;
            image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(
                    img.as_rgba8()?.as_raw(),
                    img.width(),
                    img.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .ok()?;
            Some(buf)
        })
        .unwrap_or_else(|| {
            // Fallback: generate a simple colored PNG
            use image::{ImageBuffer, ImageEncoder, Rgba};
            let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
                ImageBuffer::from_pixel(64, 64, Rgba([100, 150, 200, 255]));
            let mut buf = Vec::new();
            image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(img.as_raw(), 64, 64, image::ExtendedColorType::Rgba8)
                .unwrap();
            buf
        });

    let b64 = base64::engine::general_purpose::STANDARD.encode(&logo_png);
    // Place at row 3, col 8 — 20×20 cells
    sc.feed(b"\x1b[4;8H");
    let kitty = format!("\x1b_Ga=t,f=100,c=20,r=20;{}\x1b\\", b64);
    sc.feed(kitty.as_bytes());

    // Annotation
    sc.feed("\x1b[26;8H\x1b[36mkitty2 APC\x1b[0m  \x1b[2m(ESC _ G … ST)\x1b[0m".as_bytes());
    sc.feed(
        "\x1b[28;8H\x1b[32m● Decoded from PTY stream → InlineImageStore → rendered\x1b[0m"
            .as_bytes(),
    );
    sc.feed("\x1b[30;8H\x1b[32m● Contain-fit: aspect ratio preserved, centred\x1b[0m".as_bytes());
    sc.feed(
        "\x1b[32;8H\x1b[32m● Cross-feed sliding buffer handles chunked transfers\x1b[0m".as_bytes(),
    );
    sc.feed("\x1b[36;8H\x1b[38;2;255;107;157m简体中文\x1b[0m · \x1b[38;2;107;255;157m日本語\x1b[0m · \x1b[38;2;107;157;255m한국어\x1b[0m".as_bytes());
    sc.feed("\x1b[40;8H\x1b[2;37mLogo: kou/docs/logo.webp → PNG → kitty APC → Screen::feed → render\x1b[0m".as_bytes());

    assert_matches(&sc, "solarized-dark", "inline_image_kitty2_solarized_dark");
    assert_matches(&sc, "campbell", "inline_image_kitty2_campbell");
}

// ── 5. Inline image rendering (iTerm2) — logo via OSC 1337 ──────

#[test]
fn inline_image_iterm2() {
    let mut sc = Screen::new(120, 60);
    sc.feed(
        "\x1b[1;37m\x1b[44m\x1b[K  Inline Image Rendering — iTerm2 OSC 1337  \x1b[0m\n\n"
            .as_bytes(),
    );

    let logo_png = std::fs::read("docs/logo.webp")
        .ok()
        .and_then(|webp| {
            let img = image::load_from_memory(&webp).ok()?;
            let mut buf = Vec::new();
            use image::ImageEncoder;
            image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(
                    img.as_rgba8()?.as_raw(),
                    img.width(),
                    img.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .ok()?;
            Some(buf)
        })
        .unwrap_or_else(|| {
            use image::{ImageBuffer, ImageEncoder, Rgba};
            let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
                ImageBuffer::from_pixel(64, 64, Rgba([200, 100, 150, 255]));
            let mut buf = Vec::new();
            image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(img.as_raw(), 64, 64, image::ExtendedColorType::Rgba8)
                .unwrap();
            buf
        });

    let b64 = base64::engine::general_purpose::STANDARD.encode(&logo_png);
    sc.feed(b"\x1b[4;8H");
    let iterm = format!(
        "\x1b]1337;File=inline=1;width=20cells;height=20cells:{}\x07",
        b64
    );
    sc.feed(iterm.as_bytes());

    sc.feed(
        "\x1b[26;8H\x1b[33miTerm2 OSC 1337\x1b[0m  \x1b[2m(ESC ] 1337;File= … BEL)\x1b[0m"
            .as_bytes(),
    );
    sc.feed("\x1b[28;8H\x1b[32m● Pre-extracted from byte stream (vte can't accumulate large OSC)\x1b[0m".as_bytes());
    sc.feed("\x1b[30;8H\x1b[32m● Same contain-fit render path as kitty2\x1b[0m".as_bytes());
    sc.feed("\x1b[34;8H\x1b[2;37mLogo placed via iTerm2 inline-image protocol\x1b[0m".as_bytes());

    assert_matches(&sc, "solarized-dark", "inline_image_iterm2_solarized_dark");
}

// ── 6. Classic 80×24 terminal demo ──────────────────────────────

#[test]
fn classic_terminal() {
    let mut sc = Screen::new(80, 24);
    sc.feed("\x1b[1;37m\x1b[44m\x1b[K  kou — classic 80×24 terminal  \x1b[0m\n".as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed("  \x1b[1mProtocol Support\x1b[0m\n".as_bytes());
    sc.feed("  \x1b[32m●\x1b[0m Kitty2 APC       \x1b[36m(ESC _ G … ST)\x1b[0m\n".as_bytes());
    sc.feed("  \x1b[32m●\x1b[0m iTerm2 OSC 1337   \x1b[36m(ESC ] 1337 … BEL)\x1b[0m\n".as_bytes());
    sc.feed("  \x1b[32m●\x1b[0m Sixel DCS         \x1b[36m(ESC P q … ST)\x1b[0m\n".as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed("  \x1b[1mThemes\x1b[0m\n".as_bytes());
    let pal = (0..7)
        .map(|c| format!("\x1b[3{}m█\x1b[0m", c))
        .collect::<String>();
    sc.feed(format!("  {}  R G B C M Y K\n", pal).as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(
        "  \x1b[38;2;255;107;157m简体中文\x1b[0m · \x1b[38;2;107;255;157m日本語\x1b[0m\n"
            .as_bytes(),
    );
    sc.feed("\n".as_bytes());
    sc.feed("  \x1b[48;2;30;30;40m\x1b[37m ┌──┬──┬──┬──┐ \x1b[0m\n".as_bytes());
    sc.feed("  \x1b[48;2;30;30;40m\x1b[37m │  │  │  │  │ \x1b[0m\n".as_bytes());
    sc.feed("  \x1b[48;2;30;30;40m\x1b[37m └──┴──┴──┴──┘ \x1b[0m\n".as_bytes());

    assert_matches(&sc, "campbell", "classic_80x24_campbell");
}
