//! Snapshot tests: feed raw ANSI text directly to a Screen, render to PNG,
//! and write to each repo's `res/` folder.  No bash, no printf, no quoting
//! issues — just `screen.feed(raw_bytes)` → `render_png_supersampled`.
//!
//! Run: `cargo test --test snapshots` (writes PNGs alongside the tests).

use kou::{FontCache, Screen, theme_by_name};
use std::io::Write;

/// Load a font cache from system fonts (or empty fallback).
fn fonts() -> FontCache {
    FontCache::from_system_fonts(32.0 * 3.0) // supersampled resolution
}

/// Render a screen to a PNG file.
fn snapshot(screen: &Screen, theme_name: &str, path: &str) {
    let theme = theme_by_name(theme_name);
    let f = fonts();
    let png = kou::render::render_png_supersampled(screen, &f, 32.0, 3, theme)
        .expect("render failed");
    let dir = std::path::Path::new(path).parent().unwrap();
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(path, &png).unwrap();
    eprintln!("  wrote {path}");
}

// ── helpers to build raw ANSI text ──────────────────────────────

fn sgr(params: &str, text: &str) -> String {
    format!("\x1b[{m}{t}\x1b[0m", m = params, t = text)
}

// ── kou snapshots ────────────────────────────────────────────────

#[test]
fn kou_themed_terminal_campbell() {
    let mut sc = Screen::new(65, 18);
    sc.feed(format!("\x1b[1;37m\x1b[44m\x1b[K  kou — vtty engine  \x1b[0m\n").as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {}\n", sgr("1", "Protocol Support")).as_bytes());
    sc.feed(format!("  {} Kitty2 APC       {}\n",
        sgr("32", "●"), sgr("36", "(ESC _ G … ST)")).as_bytes());
    sc.feed(format!("  {} iTerm2 OSC 1337   {}\n",
        sgr("32", "●"), sgr("36", "(ESC ] 1337 … BEL)")).as_bytes());
    sc.feed(format!("  {} Sixel DCS         {}\n",
        sgr("32", "●"), sgr("36", "(ESC P q … ST)")).as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {}\n", sgr("1", "Themes (from Windows Terminal)")).as_bytes());
    let palette = format!("{}{}{}{}{}{}{}",
        sgr("1;31","R"), sgr("1;32","G"), sgr("1;33","B"),
        sgr("1;34","C"), sgr("1;35","M"), sgr("1;36","Y"), sgr("1;37","K"));
    sc.feed(format!("  {}  ANSI palette test\n", palette).as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {} · {}\n",
        sgr("38;2;255;107;157","简体中文"), sgr("38;2;107;255;157","日本語")).as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed("  \x1b[48;2;30;30;40m\x1b[37m ┌──┬──┬──┐ \x1b[0m\n".as_bytes());
    sc.feed("  \x1b[48;2;30;30;40m\x1b[37m │  │  │  │ \x1b[0m\n".as_bytes());
    sc.feed("  \x1b[48;2;30;30;40m\x1b[37m └──┴──┴──┘ \x1b[0m\n".as_bytes());
    snapshot(&sc, "campbell", "res/themed_terminal_campbell.png");
}

#[test]
fn kou_themed_terminal_solarized() {
    let mut sc = Screen::new(65, 18);
    sc.feed(format!("\x1b[1;37m\x1b[44m\x1b[K  kou — vtty engine  \x1b[0m\n").as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {}\n", sgr("1", "Protocol Support")).as_bytes());
    sc.feed(format!("  {} Kitty2 APC       {}\n",
        sgr("32", "●"), sgr("36", "(ESC _ G … ST)")).as_bytes());
    sc.feed(format!("  {} iTerm2 OSC 1337   {}\n",
        sgr("32", "●"), sgr("36", "(ESC ] 1337 … BEL)")).as_bytes());
    sc.feed(format!("  {} Sixel DCS         {}\n",
        sgr("32", "●"), sgr("36", "(ESC P q … ST)")).as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {}\n", sgr("1", "15 Windows Terminal schemes")).as_bytes());
    let palette = format!("{}{}{}{}{}{}{}",
        sgr("1;31","R"), sgr("1;32","G"), sgr("1;33","B"),
        sgr("1;34","C"), sgr("1;35","M"), sgr("1;36","Y"), sgr("1;37","K"));
    sc.feed(format!("  {}  Campbell · Solarized · Tango\n", palette).as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {} · {}\n",
        sgr("38;2;255;107;157","简体中文"), sgr("38;2;107;255;157","日本語")).as_bytes());
    snapshot(&sc, "solarized-dark", "res/themed_terminal_solarized_dark.png");
}

// ── seia snapshots ───────────────────────────────────────────────

#[test]
fn seia_search_results() {
    let mut sc = Screen::new(68, 22);
    sc.feed("\x1b[1;37m\x1b[44m\x1b[K  seia — multi-engine web search  \x1b[0m\n".as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed("  \x1b[1m$\x1b[0m seia search \"rust async patterns\" --engine duckduckgo\n".as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {} DuckDuckGo  {} Results: 3  {} 0.42s\n",
        sgr("2;36","Engine:"), sgr("2","|"), sgr("2","|")).as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {} \x1b[4;34mAsynchronous Programming in Rust\x1b[0m\n", sgr("33","1.")).as_bytes());
    sc.feed("     \x1b[2mrust-lang.org/async-book\x1b[0m\n".as_bytes());
    sc.feed("     The official guide to async/await.\n".as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {} \x1b[4;34mTokio Tutorial\x1b[0m\n", sgr("33","2.")).as_bytes());
    sc.feed("     \x1b[2mtokio.rs/tokio/tutorial\x1b[0m\n".as_bytes());
    sc.feed("     Build async applications with Tokio.\n".as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {} \x1b[4;34mFutures Explained in 200 Lines\x1b[0m\n", sgr("33","3.")).as_bytes());
    sc.feed("     \x1b[2mcfsamson.github.io\x1b[0m\n".as_bytes());
    sc.feed("     Build a future executor from scratch.\n".as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {} Also tried: {} {} {}\n",
        sgr("32","●"), sgr("36","Bing"), sgr("2","(3)"), sgr("36","  Brave")).as_bytes());
    snapshot(&sc, "solarized-dark", "../seia/res/search_solarized_dark.png");
    snapshot(&sc, "campbell", "../seia/res/search_campbell.png");
}

// ── shirabe snapshots ────────────────────────────────────────────

#[test]
fn shirabe_debug_server() {
    let mut sc = Screen::new(68, 22);
    sc.feed("\x1b[1;37m\x1b[44m\x1b[K  shirabe — headless browser automation  \x1b[0m\n".as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed("  \x1b[1m$\x1b[0m shirabe serve --port 3001\n".as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {} Backend: {} {}\n",
        sgr("32","●"), sgr("36","Chromium 131.0"), sgr("2","(headless)")).as_bytes());
    sc.feed(format!("  {} Debug API: \x1b[4;34mhttp://localhost:3001\x1b[0m\n", sgr("32","●")).as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {}\n", sgr("1","Endpoints")).as_bytes());
    sc.feed("  \x1b[2mGET\x1b[0m  /health        Health check\n".as_bytes());
    sc.feed("  \x1b[2mGET\x1b[0m  /info          Browser info\n".as_bytes());
    sc.feed("  \x1b[2mPOST\x1b[0m /navigate       Navigate to URL\n".as_bytes());
    sc.feed("  \x1b[2mPOST\x1b[0m /screenshot      Capture PNG\n".as_bytes());
    sc.feed("  \x1b[2mPOST\x1b[0m /click           Click element\n".as_bytes());
    sc.feed("  \x1b[2mGET\x1b[0m  /dom            Query DOM\n".as_bytes());
    sc.feed("  \x1b[2mGET\x1b[0m  /a11y           Accessibility tree\n".as_bytes());
    sc.feed("\n".as_bytes());
    sc.feed(format!("  {} Zero-config: finds Chrome automatically\n", sgr("32","●")).as_bytes());
    snapshot(&sc, "solarized-dark", "../shirabe/res/debug_server_solarized_dark.png");
    snapshot(&sc, "one-half-dark", "../shirabe/res/debug_server_one_half_dark.png");
}
