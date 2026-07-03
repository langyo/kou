//! Snapshot tests: feed raw ANSI text directly to a Screen, render to PNG.

use kou::{FontCache, Screen, theme_by_name};

fn fonts() -> FontCache { FontCache::from_system_fonts(32.0 * 3.0) }

fn snapshot(screen: &Screen, theme: &str, path: &str) {
    let png = kou::render::render_png_supersampled(screen, &fonts(), 32.0, 3, theme_by_name(theme))
        .expect("render");
    let dir = std::path::Path::new(path).parent().unwrap();
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(path, &png).unwrap();
    eprintln!("  wrote {path}");
}

// ── 1. Neofetch-style showcase ──────────────────────────────────

#[test]
fn neofetch_showcase() {
    let mut sc = Screen::new(80, 26);
    let art = [
        "       #######       ",
        "      ##     ##      ",
        "     ##       ##     ",
        "    ##         ##    ",
        "   ##           ##   ",
        "  ##  #  kou  #  ##  ",
        "  ##             ##  ",
        "  ##             ##  ",
        "   ##           ##   ",
        "    ##         ##    ",
        "     ##       ##     ",
        "      ##     ##      ",
        "       #######       ",
    ];
    let info: [(&str, &str); 10] = [
        ("OS",       "kou VTty Engine v0.1"),
        ("Host",     "PTY + VT100 emulator"),
        ("Kernel",   "vte 0.13 / ab_glyph"),
        ("Shell",    "kou::VttyManager"),
        ("Themes",   "15 Windows Terminal schemes"),
        ("Fonts",    "DejaVu / Noto / system"),
        ("Protocol", "Kitty2 + iTerm2 + Sixel"),
        ("Render",   "Lanczos3 supersampled"),
        ("CJK",      "简体中文 · 日本語 OK"),
        ("Uptime",   "since you started reading"),
    ];

    // Feed each row as one complete format string.
    let line0 = format!("\x1b[1;36m{:<24}\x1b[0m \x1b[1;37m{}\x1b[0m\x1b[2;37m@kou\x1b[0m\n", art[0], "langyo");
    sc.feed(line0.as_bytes());
    for i in 0..info.len() {
        let al = if i + 1 < art.len() { art[i + 1] } else { "" };
        let line = format!("\x1b[1;36m{:<24}\x1b[0m \x1b[1;33m{:<10}\x1b[0m{}\n", al, info[i].0, info[i].1);
        sc.feed(line.as_bytes());
    }
    // Color palette strip
    sc.feed(b"\n");
    let mut pal = String::from("  ");
    for c in 0u8..8 {
        pal.push_str(&format!("\x1b[4{}m  \x1b[0m", c));
    }
    pal.push('\n');
    sc.feed(pal.as_bytes());

    snapshot(&sc, "solarized-dark", "res/neofetch_solarized_dark.png");
    snapshot(&sc, "campbell",       "res/neofetch_campbell.png");
    snapshot(&sc, "one-half-dark",  "res/neofetch_one_half_dark.png");
}

// ── 2. Rainbow gradient (visual flair) ──────────────────────────

#[test]
fn rainbow_gradient() {
    let mut sc = Screen::new(48, 20);
    let colors = [31u8, 33, 32, 36, 34, 35, 31, 33, 32, 36, 34, 35];
    sc.feed(b"\n");
    for (row, &color) in colors.iter().enumerate() {
        let spaces = " ".repeat(row + 8);
        let blocks = "#".repeat(row * 2 + 1);
        sc.feed(format!("{}\x1b[1;{}m{}\x1b[0m\n", spaces, color, blocks).as_bytes());
    }
    sc.feed(format!("\n  \x1b[2;37mkou — VTty engine · rainbow gradient\x1b[0m\n").as_bytes());
    snapshot(&sc, "campbell", "res/rainbow_gradient_campbell.png");
}

// ── 3. Protocol comparison table ────────────────────────────────

#[test]
fn protocol_comparison() {
    let mut sc = Screen::new(72, 14);
    sc.feed("\x1b[1;37m\x1b[44m\x1b[K  Inline Image Protocols  \x1b[0m\n\n".as_bytes());
    let rows: &[(&str, &str, &str, &str); 8] = &[
        ("Feature",  "Kitty2 (APC)",       "iTerm2 (OSC 1337)",  "Sixel (DCS)"),
        ("Direction", "Encode + Decode",    "Encode + Decode",    "Encode + Decode"),
        ("Format",    "base64 PNG",         "base64 PNG",         "Raster language"),
        ("Chunking",  "4096-byte chunks",   "Single frame",       "RLE compressed"),
        ("Cross-feed","Sliding buffer",     "Sliding buffer",     "Sliding buffer"),
        ("Aspect",    "Contain-fit",        "Contain-fit",        "Pixel-fit"),
        ("Dependency","Built-in",           "Built-in",           "icy_sixel crate"),
        ("Terminals", "kitty, wezterm",     "iTerm2, wezterm",    "xterm, mlterm"),
    ];
    for (i, (label, k, it, sx)) in rows.iter().enumerate() {
        let lc = if i == 0 { "1;37" } else { "0" };
        let kc = if i == 0 { "1;36" } else { "36" };
        let ic = if i == 0 { "1;33" } else { "33" };
        let xc = if i == 0 { "1;35" } else { "35" };
        sc.feed(format!("\x1b[{lc}m{:<14}\x1b[0m \x1b[{kc}m{:<20}\x1b[0m \x1b[{ic}m{:<20}\x1b[0m \x1b[{xc}m{}\x1b[0m\n",
            label, k, it, sx, lc=lc, kc=kc, ic=ic, xc=xc).as_bytes());
    }
    snapshot(&sc, "one-half-dark", "res/protocol_comparison.png");
}

// ── 4. seia search results ──────────────────────────────────────

#[test]
fn seia_search_results() {
    let mut sc = Screen::new(62, 24);
    sc.feed("\x1b[1;37m\x1b[44m\x1b[K  seia — multi-engine web search  \x1b[0m\n\n".as_bytes());
    sc.feed("  \x1b[1m$\x1b[0m seia search \"rust async patterns\"\n".as_bytes());
    sc.feed("  \x1b[2;36mEngine:\x1b[0m \x1b[36mDuckDuckGo\x1b[0m \x1b[2m| Results: 5 | Time: 0.42s\x1b[0m\n\n".as_bytes());
    let results = [
        ("Asynchronous Programming in Rust", "rust-lang.org/async-book", "The official guide to async/await."),
        ("Tokio Tutorial", "tokio.rs/tokio/tutorial", "Build async apps with the Tokio runtime."),
        ("async-std Documentation", "async.rs", "Async version of the standard library."),
        ("Pin and Unpin in Rust", "blog.cloudflare.com/pin-and-unpin", "Self-referential structs explained."),
        ("Futures Explained in 200 Lines", "cfsamson.github.io", "Build a future executor from scratch."),
    ];
    for (i, (title, url, desc)) in results.iter().enumerate() {
        sc.feed(format!("  \x1b[33m{}.\x1b[0m \x1b[4;34m{}\x1b[0m\n", i + 1, title).as_bytes());
        sc.feed(format!("     \x1b[2m{}\x1b[0m\n", url).as_bytes());
        sc.feed(format!("     {}\n\n", desc).as_bytes());
    }
    sc.feed("  \x1b[32m●\x1b[0m Also tried: \x1b[36mBing (3)\x1b[0m  \x1b[36mBrave (2)\x1b[0m\n".as_bytes());
    snapshot(&sc, "solarized-dark", "../seia/res/search_solarized_dark.png");
    snapshot(&sc, "campbell",       "../seia/res/search_campbell.png");
}

// ── 5. shirabe debug server ─────────────────────────────────────

#[test]
fn shirabe_debug_server() {
    let mut sc = Screen::new(58, 22);
    sc.feed("\x1b[1;37m\x1b[44m\x1b[K  shirabe — headless browser automation  \x1b[0m\n\n".as_bytes());
    sc.feed("  \x1b[1m$\x1b[0m shirabe serve --port 3001\n\n".as_bytes());
    sc.feed("  \x1b[32m●\x1b[0m Backend: \x1b[36mChromium 131.0\x1b[0m \x1b[2m(headless)\x1b[0m\n".as_bytes());
    sc.feed("  \x1b[32m●\x1b[0m Debug API: \x1b[4;34mhttp://localhost:3001\x1b[0m\n\n".as_bytes());
    sc.feed("  \x1b[1mHTTP API\x1b[0m\n\n".as_bytes());
    let endpoints = [
        ("GET",  "/health",     "Health check"),
        ("GET",  "/info",       "Browser + viewport info"),
        ("POST", "/navigate",   "Navigate to a URL"),
        ("POST", "/screenshot", "Capture viewport as PNG"),
        ("POST", "/click",      "Click element by selector"),
        ("POST", "/type",       "Type text into a field"),
        ("POST", "/evaluate",   "Execute JavaScript"),
        ("GET",  "/dom",        "Query DOM by CSS selector"),
        ("GET",  "/a11y",       "Accessibility tree snapshot"),
    ];
    for (method, path, desc) in &endpoints {
        let mc = if *method == "GET" { "2;37" } else { "2;33" };
        sc.feed(format!("  \x1b[{}m{:<4}\x1b[0m \x1b[36m{:<14}\x1b[0m {}\n", mc, method, path, desc).as_bytes());
    }
    sc.feed("\n  \x1b[32m●\x1b[0m Zero-config: auto-discovers Chrome / Chromium / Edge\n".as_bytes());
    snapshot(&sc, "solarized-dark", "../shirabe/res/debug_server_solarized_dark.png");
    snapshot(&sc, "one-half-dark",  "../shirabe/res/debug_server_one_half_dark.png");
}
