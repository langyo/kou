//! Snapshot tests: feed raw ANSI text directly to a Screen, render to PNG.
//! Large screens (120×60+) for most; 80×24 only for the classic-size test.

use base64::Engine as _;
use kou::{FontCache, Screen, theme_by_name};

fn fonts() -> FontCache {
    FontCache::from_system_fonts(32.0 * 3.0)
}

fn snapshot(screen: &Screen, theme: &str, path: &str) {
    let png = kou::render::render_png_supersampled(screen, &fonts(), 32.0, 3, theme_by_name(theme))
        .expect("render");
    let dir = std::path::Path::new(path).parent().unwrap();
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(path, &png).unwrap();
    eprintln!("  wrote {path}");
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
        ("Fonts", "DejaVuSansMono · NotoSansCJK · system"),
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

    snapshot(&sc, "solarized-dark", "res/neofetch_solarized_dark.png");
    snapshot(&sc, "campbell", "res/neofetch_campbell.png");
    snapshot(&sc, "one-half-dark", "res/neofetch_one_half_dark.png");
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
    snapshot(&sc, "campbell", "res/rainbow_gradient_campbell.png");
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
    snapshot(&sc, "one-half-dark", "res/protocol_comparison.png");
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

    snapshot(
        &sc,
        "solarized-dark",
        "res/inline_image_kitty2_solarized_dark.png",
    );
    snapshot(&sc, "campbell", "res/inline_image_kitty2_campbell.png");
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

    snapshot(
        &sc,
        "solarized-dark",
        "res/inline_image_iterm2_solarized_dark.png",
    );
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

    snapshot(&sc, "campbell", "res/classic_80x24_campbell.png");
}

// ── 7. seia search results (120×50) ─────────────────────────────

#[test]
fn seia_search_results() {
    let mut sc = Screen::new(110, 50);
    sc.feed("\x1b[1;37m\x1b[44m\x1b[K  seia — multi-engine web search  \x1b[0m\n\n".as_bytes());
    sc.feed(
        "  \x1b[1m$\x1b[0m seia search \"rust async patterns\" --engine duckduckgo --limit 8\n"
            .as_bytes(),
    );
    sc.feed("  \x1b[2;36mEngine:\x1b[0m \x1b[36mDuckDuckGo\x1b[0m  \x1b[2m| Results: 8 | Time: 0.42s\x1b[0m\n\n".as_bytes());
    let results = [
        (
            "Asynchronous Programming in Rust",
            "rust-lang.org/async-book",
            "The official guide to async/await. Covers futures, tokio, async-std, and common patterns for building concurrent Rust applications.",
        ),
        (
            "Tokio Tutorial",
            "tokio.rs/tokio/tutorial",
            "Learn how to build asynchronous applications with Tokio, the most popular async runtime for Rust. Includes channels, spawning, and I/O.",
        ),
        (
            "Futures Explained in 200 Lines of Rust",
            "cfsamson.github.io/books-futures-explained",
            "Build a future executor from scratch to understand how async/await works under the hood. Clear, concise, and code-heavy.",
        ),
        (
            "Pin and Unpin in Rust",
            "blog.cloudflare.com/pin-and-unpin-in-rust",
            "A deep dive into self-referential structs, the Pin type, and why async Rust needs them. By Cloudflare's engineering team.",
        ),
        (
            "async-std Documentation",
            "async.rs",
            "An asynchronous version of the Rust standard library. Provides async file I/O, networking, timers, and more.",
        ),
        (
            "Smol — a small and fast async runtime",
            "github.com/smol-rs/smol",
            "A minimal, correct, and fast async runtime. Smol is designed to be simple to understand and easy to build on.",
        ),
        (
            "Async Rust Is Hard",
            "jacko.io/async-rust-is-hard",
            "An honest discussion of the pain points of async Rust, with practical advice for navigating them in real projects.",
        ),
        (
            "The Why of Async/Await in Rust",
            "yatima.io/blog/Async-Await",
            "Historical context and design rationale behind Rust's async/await syntax and the tokio ecosystem.",
        ),
    ];
    for (i, (title, url, desc)) in results.iter().enumerate() {
        sc.feed(format!("  \x1b[33m{:2}.\x1b[0m \x1b[4;34m{}\x1b[0m\n", i + 1, title).as_bytes());
        sc.feed(format!("      \x1b[2m{}\x1b[0m\n", url).as_bytes());
        sc.feed(format!("      {}\n\n", desc).as_bytes());
    }
    sc.feed("  \x1b[32m●\x1b[0m Also tried: \x1b[36mBing (5)\x1b[0m  \x1b[36mBrave (3)\x1b[0m  \x1b[36mWikipedia (1)\x1b[0m\n".as_bytes());
    snapshot(
        &sc,
        "solarized-dark",
        "../seia/res/search_solarized_dark.png",
    );
    snapshot(&sc, "campbell", "../seia/res/search_campbell.png");
}

// ── 8. shirabe debug server (110×50) ────────────────────────────

#[test]
fn shirabe_debug_server() {
    let mut sc = Screen::new(100, 50);
    sc.feed(
        "\x1b[1;37m\x1b[44m\x1b[K  shirabe — headless browser automation  \x1b[0m\n\n".as_bytes(),
    );
    sc.feed("  \x1b[1m$\x1b[0m shirabe serve --port 3001\n\n".as_bytes());
    sc.feed("  \x1b[32m●\x1b[0m Backend:  \x1b[36mChromium 131.0\x1b[0m \x1b[2m(headless, auto-detected)\x1b[0m\n".as_bytes());
    sc.feed("  \x1b[32m●\x1b[0m Debug API: \x1b[4;34mhttp://localhost:3001\x1b[0m\n".as_bytes());
    sc.feed(
        "  \x1b[32m●\x1b[0m CDP:      \x1b[4;34mws://localhost:3001/devtools\x1b[0m\n\n".as_bytes(),
    );
    sc.feed("  \x1b[1mHTTP API Endpoints\x1b[0m\n\n".as_bytes());
    let endpoints = [
        ("GET", "/health", "Health check — returns {\"ok\":true}"),
        (
            "GET",
            "/info",
            "Browser version, viewport dimensions, current URL",
        ),
        (
            "POST",
            "/navigate",
            "Navigate to a URL: {\"url\":\"https://…\"}",
        ),
        ("POST", "/back", "Go back in browser history"),
        ("POST", "/forward", "Go forward in browser history"),
        (
            "POST",
            "/screenshot",
            "Capture viewport as base64 PNG: {\"selector\":\"…\",\"full_page\":true}",
        ),
        (
            "POST",
            "/click",
            "Click an element: {\"selector\":\"#button\"}",
        ),
        (
            "POST",
            "/type",
            "Type text: {\"selector\":\"input\",\"text\":\"hello\",\"submit\":true}",
        ),
        ("POST", "/press", "Press a key: {\"key\":\"Enter\"}"),
        (
            "POST",
            "/evaluate",
            "Execute JavaScript: {\"expression\":\"document.title\"}",
        ),
        (
            "GET",
            "/dom",
            "Query DOM: ?selector=div&attribute=class&computed=true",
        ),
        ("GET", "/a11y", "Accessibility tree snapshot: ?selector=nav"),
        ("GET", "/console", "Console log entries: ?level=error"),
        (
            "POST",
            "/resize",
            "Resize viewport: {\"width\":1280,\"height\":720}",
        ),
    ];
    for (method, path, desc) in &endpoints {
        let mc = if *method == "GET" { "2;37" } else { "2;33" };
        sc.feed(
            format!(
                "  \x1b[{}m{:<5}\x1b[0m \x1b[36m{:<16}\x1b[0m {}\n",
                mc, method, path, desc
            )
            .as_bytes(),
        );
    }
    sc.feed(
        "\n  \x1b[32m●\x1b[0m Zero-config: auto-discovers Chrome / Chromium / Edge\n".as_bytes(),
    );
    sc.feed("  \x1b[32m●\x1b[0m Backends: Chrome for Testing fetch, system Chrome, foreign-engine (FF/Servo)\n".as_bytes());
    sc.feed(
        "  \x1b[32m●\x1b[0m Extracted from tairitsu packager — hardened to stand alone\n"
            .as_bytes(),
    );
    snapshot(
        &sc,
        "solarized-dark",
        "../shirabe/res/debug_server_solarized_dark.png",
    );
    snapshot(
        &sc,
        "one-half-dark",
        "../shirabe/res/debug_server_one_half_dark.png",
    );
}
