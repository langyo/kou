//! Screen rendering.
//!
//! Two backends share one cell grid:
//!
//! - [`render_png`] rasterises the screen to PNG bytes using a [`FontCache`]
//!   (proper glyph outlines, a 16-colour ANSI palette, CJK fallback).
//!   Glyph positions are derived from each face's positioned bounding box, so
//!   characters land exactly where they should.
//! - [`render_png_supersampled`] renders at an integer factor then downscales
//!   with Lanczos3, for a crisp, anti-aliased result where box-drawing glyphs
//!   (`─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼`) connect seamlessly across cells instead of looking
//!   like the blocky old-terminal tiles.
//! - [`render_graphics`] emits an inband graphics-protocol escape sequence
//!   (kitty / iTerm2) so a capable terminal renders the pixels inline.
//!
//! Colour comes from a selectable [`Theme`] — a 16-slot ANSI palette plus a
//! background/foreground, ported verbatim from Windows Terminal's built-in
//! `defaults.json` schemes (Campbell, One Half, Solarized, Tango, …). Pick one
//! with [`theme_by_name`]; unknown names fall back to Campbell (the WT default).
//!
//! The renderer is font-driven: pass a [`FontCache`] loaded from the
//! [`crate::font`] module. When the cache is empty it falls back to drawing a
//! solid block per glyph, so kou still produces *something* without fonts.

use crate::font::FontCache;
use crate::graphics::{self, GraphicsProtocol, GraphicsRequest};
use crate::screen::Screen;
use anyhow::Result;
use image::{ImageBuffer, Rgba};

// ── Colour themes ───────────────────────────────────────────────

/// A 16-colour ANSI terminal theme: the background/foreground plus the
/// `black..white` / `brightBlack..brightWhite` palette slots. The values are
/// ported from Microsoft Terminal's `defaults.json` colour schemes.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    /// Canvas / default background.
    pub background: [u8; 3],
    /// Default foreground (stored verbatim from the scheme; the renderer paints
    /// text through the `ansi` palette, so cell index 7 = the scheme's `white`).
    pub foreground: [u8; 3],
    /// ANSI slots 0..16: black, red, green, yellow, blue, magenta, cyan, white,
    /// then brightBlack..brightWhite.
    pub ansi: [[u8; 3]; 16],
}

/// Decode a 6-hex-digit `&[u8; 6]` (e.g. `b"0C0C0C"`) to an RGB triple at
/// compile time, so theme tables are pure data with no runtime parsing.
const fn dn(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}
const fn hx(bytes: &[u8; 6]) -> [u8; 3] {
    [
        dn(bytes[0]) * 16 + dn(bytes[1]),
        dn(bytes[2]) * 16 + dn(bytes[3]),
        dn(bytes[4]) * 16 + dn(bytes[5]),
    ]
}

/// All built-in themes, Campbell first (it is the default fallback).
pub static THEMES: &[Theme] = &[
    Theme {
        name: "Campbell",
        background: hx(b"0C0C0C"),
        foreground: hx(b"CCCCCC"),
        ansi: [
            hx(b"0C0C0C"),
            hx(b"C50F1F"),
            hx(b"13A10E"),
            hx(b"C19C00"),
            hx(b"0037DA"),
            hx(b"881798"),
            hx(b"3A96DD"),
            hx(b"CCCCCC"),
            hx(b"767676"),
            hx(b"E74856"),
            hx(b"16C60C"),
            hx(b"F9F1A5"),
            hx(b"3B78FF"),
            hx(b"B4009E"),
            hx(b"61D6D6"),
            hx(b"F2F2F2"),
        ],
    },
    Theme {
        name: "Campbell Powershell",
        background: hx(b"012456"),
        foreground: hx(b"CCCCCC"),
        ansi: [
            hx(b"0C0C0C"),
            hx(b"C50F1F"),
            hx(b"13A10E"),
            hx(b"C19C00"),
            hx(b"0037DA"),
            hx(b"881798"),
            hx(b"3A96DD"),
            hx(b"CCCCCC"),
            hx(b"767676"),
            hx(b"E74856"),
            hx(b"16C60C"),
            hx(b"F9F1A5"),
            hx(b"3B78FF"),
            hx(b"B4009E"),
            hx(b"61D6D6"),
            hx(b"F2F2F2"),
        ],
    },
    Theme {
        name: "Vintage",
        background: hx(b"000000"),
        foreground: hx(b"C0C0C0"),
        ansi: [
            hx(b"000000"),
            hx(b"800000"),
            hx(b"008000"),
            hx(b"808000"),
            hx(b"000080"),
            hx(b"800080"),
            hx(b"008080"),
            hx(b"C0C0C0"),
            hx(b"808080"),
            hx(b"FF0000"),
            hx(b"00FF00"),
            hx(b"FFFF00"),
            hx(b"0000FF"),
            hx(b"FF00FF"),
            hx(b"00FFFF"),
            hx(b"FFFFFF"),
        ],
    },
    Theme {
        name: "One Half Dark",
        background: hx(b"282C34"),
        foreground: hx(b"DCDFE4"),
        ansi: [
            hx(b"282C34"),
            hx(b"E06C75"),
            hx(b"98C379"),
            hx(b"E5C07B"),
            hx(b"61AFEF"),
            hx(b"C678DD"),
            hx(b"56B6C2"),
            hx(b"DCDFE4"),
            hx(b"5A6374"),
            hx(b"E06C75"),
            hx(b"98C379"),
            hx(b"E5C07B"),
            hx(b"61AFEF"),
            hx(b"C678DD"),
            hx(b"56B6C2"),
            hx(b"DCDFE4"),
        ],
    },
    Theme {
        name: "One Half Light",
        background: hx(b"FAFAFA"),
        foreground: hx(b"383A42"),
        ansi: [
            hx(b"383A42"),
            hx(b"E45649"),
            hx(b"50A14F"),
            hx(b"C18301"),
            hx(b"0184BC"),
            hx(b"A626A4"),
            hx(b"0997B3"),
            hx(b"FAFAFA"),
            hx(b"4F525D"),
            hx(b"DF6C75"),
            hx(b"98C379"),
            hx(b"E4C07A"),
            hx(b"61AFEF"),
            hx(b"C577DD"),
            hx(b"56B5C1"),
            hx(b"FFFFFF"),
        ],
    },
    Theme {
        name: "Solarized Dark",
        background: hx(b"002B36"),
        foreground: hx(b"839496"),
        ansi: [
            hx(b"002B36"),
            hx(b"DC322F"),
            hx(b"859900"),
            hx(b"B58900"),
            hx(b"268BD2"),
            hx(b"D33682"),
            hx(b"2AA198"),
            hx(b"EEE8D5"),
            hx(b"073642"),
            hx(b"CB4B16"),
            hx(b"586E75"),
            hx(b"657B83"),
            hx(b"839496"),
            hx(b"6C71C4"),
            hx(b"93A1A1"),
            hx(b"FDF6E3"),
        ],
    },
    Theme {
        name: "Solarized Light",
        background: hx(b"FDF6E3"),
        foreground: hx(b"657B83"),
        ansi: [
            hx(b"002B36"),
            hx(b"DC322F"),
            hx(b"859900"),
            hx(b"B58900"),
            hx(b"268BD2"),
            hx(b"D33682"),
            hx(b"2AA198"),
            hx(b"EEE8D5"),
            hx(b"073642"),
            hx(b"CB4B16"),
            hx(b"586E75"),
            hx(b"657B83"),
            hx(b"839496"),
            hx(b"6C71C4"),
            hx(b"93A1A1"),
            hx(b"FDF6E3"),
        ],
    },
    Theme {
        name: "Tango Dark",
        background: hx(b"000000"),
        foreground: hx(b"D3D7CF"),
        ansi: [
            hx(b"000000"),
            hx(b"CC0000"),
            hx(b"4E9A06"),
            hx(b"C4A000"),
            hx(b"3465A4"),
            hx(b"75507B"),
            hx(b"06989A"),
            hx(b"D3D7CF"),
            hx(b"555753"),
            hx(b"EF2929"),
            hx(b"8AE234"),
            hx(b"FCE94F"),
            hx(b"729FCF"),
            hx(b"AD7FA8"),
            hx(b"34E2E2"),
            hx(b"EEEEEC"),
        ],
    },
    Theme {
        name: "Tango Light",
        background: hx(b"FFFFFF"),
        foreground: hx(b"555753"),
        ansi: [
            hx(b"000000"),
            hx(b"CC0000"),
            hx(b"4E9A06"),
            hx(b"C4A000"),
            hx(b"3465A4"),
            hx(b"75507B"),
            hx(b"06989A"),
            hx(b"D3D7CF"),
            hx(b"555753"),
            hx(b"EF2929"),
            hx(b"8AE234"),
            hx(b"FCE94F"),
            hx(b"729FCF"),
            hx(b"AD7FA8"),
            hx(b"34E2E2"),
            hx(b"EEEEEC"),
        ],
    },
    Theme {
        name: "Dimidium",
        background: hx(b"141414"),
        foreground: hx(b"BAB7B6"),
        ansi: [
            hx(b"000000"),
            hx(b"CF494C"),
            hx(b"60B442"),
            hx(b"DB9C11"),
            hx(b"0575D8"),
            hx(b"AF5ED2"),
            hx(b"1DB6BB"),
            hx(b"BAB7B6"),
            hx(b"817E7E"),
            hx(b"FF643B"),
            hx(b"37E57B"),
            hx(b"FCCD1A"),
            hx(b"688DFD"),
            hx(b"ED6FE9"),
            hx(b"32E0FB"),
            hx(b"DEE3E4"),
        ],
    },
    Theme {
        name: "Ottosson",
        background: hx(b"000000"),
        foreground: hx(b"BEBEBE"),
        ansi: [
            hx(b"000000"),
            hx(b"BE2C21"),
            hx(b"3FAE3A"),
            hx(b"BE9A4A"),
            hx(b"204DBE"),
            hx(b"BB54BE"),
            hx(b"00A7B2"),
            hx(b"BEBEBE"),
            hx(b"808080"),
            hx(b"FF3E30"),
            hx(b"58EA51"),
            hx(b"FFC944"),
            hx(b"2F6AFF"),
            hx(b"FC74FF"),
            hx(b"00E1F0"),
            hx(b"FFFFFF"),
        ],
    },
    Theme {
        name: "Dark+",
        background: hx(b"1E1E1E"),
        foreground: hx(b"CCCCCC"),
        ansi: [
            hx(b"000000"),
            hx(b"CD3131"),
            hx(b"0DBC79"),
            hx(b"E5E510"),
            hx(b"2472C8"),
            hx(b"BC3FBC"),
            hx(b"11A8CD"),
            hx(b"E5E5E5"),
            hx(b"666666"),
            hx(b"F14C4C"),
            hx(b"23D18B"),
            hx(b"F5F543"),
            hx(b"3B8EEA"),
            hx(b"D670D6"),
            hx(b"29B8DB"),
            hx(b"E5E5E5"),
        ],
    },
    Theme {
        name: "CGA",
        background: hx(b"000000"),
        foreground: hx(b"AAAAAA"),
        ansi: [
            hx(b"000000"),
            hx(b"AA0000"),
            hx(b"00AA00"),
            hx(b"AA5500"),
            hx(b"0000AA"),
            hx(b"AA00AA"),
            hx(b"00AAAA"),
            hx(b"AAAAAA"),
            hx(b"555555"),
            hx(b"FF5555"),
            hx(b"55FF55"),
            hx(b"FFFF55"),
            hx(b"5555FF"),
            hx(b"FF55FF"),
            hx(b"55FFFF"),
            hx(b"FFFFFF"),
        ],
    },
    Theme {
        name: "IBM 5153",
        background: hx(b"000000"),
        foreground: hx(b"AAAAAA"),
        ansi: [
            hx(b"000000"),
            hx(b"AA0000"),
            hx(b"00AA00"),
            hx(b"C47E00"),
            hx(b"0000AA"),
            hx(b"AA00AA"),
            hx(b"00AAAA"),
            hx(b"AAAAAA"),
            hx(b"555555"),
            hx(b"FF5555"),
            hx(b"55FF55"),
            hx(b"FFFF55"),
            hx(b"5555FF"),
            hx(b"FF55FF"),
            hx(b"55FFFF"),
            hx(b"FFFFFF"),
        ],
    },
    // The original kou xterm palette — kept as an explicit theme so callers that
    // want the pre-theming look (or a no-frills classic) can ask for it.
    Theme {
        name: "xterm",
        background: hx(b"18181C"),
        foreground: hx(b"E5E5E5"),
        ansi: [
            hx(b"000000"),
            hx(b"CD0000"),
            hx(b"00CD00"),
            hx(b"CDCD00"),
            hx(b"0000EE"),
            hx(b"CD00CD"),
            hx(b"00CDCD"),
            hx(b"E5E5E5"),
            hx(b"7F7F7F"),
            hx(b"FF0000"),
            hx(b"00FF00"),
            hx(b"FFFF00"),
            hx(b"5C5CFF"),
            hx(b"FF00FF"),
            hx(b"00FFFF"),
            hx(b"FFFFFF"),
        ],
    },
];

/// Lower-case, alphanumeric-only key for fuzzy theme-name matching
/// (`"Solarized Dark"`, `"solarized-dark"`, `"solarized_dark"` all collapse to
/// `"solarizeddark"`).
fn key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Resolve a theme by name (case- and separator-insensitive). `"default"`
/// aliases the classic xterm palette; any other unknown name falls back to
/// Campbell (the Windows Terminal default).
pub fn theme_by_name(name: &str) -> &'static Theme {
    let k = key(name);
    if k == "default" {
        return THEMES
            .iter()
            .find(|t| t.name == "xterm")
            .expect("xterm theme present");
    }
    THEMES
        .iter()
        .find(|t| key(t.name) == k)
        .unwrap_or(&THEMES[0])
}

// ── PNG rendering ───────────────────────────────────────────────

/// Render the screen to PNG bytes at the font's natural scale (1×).
///
/// `fonts` should be loaded at `font_px`. Cell width is the rounded advance of
/// `M`; cell height is the rounded em (`ascent − descent`) — so box-drawing
/// glyphs, which fill exactly one advance × one em, line up cell-to-cell.
pub fn render_png(
    screen: &Screen,
    fonts: &FontCache,
    font_px: f32,
    theme: &Theme,
) -> Result<Vec<u8>> {
    let buf = render_buffer(screen, fonts, font_px, theme)?;
    encode_png(buf.image, buf.width, buf.height)
}

/// Render at `supersample`× then downscale with Lanczos3. `fonts` must be loaded
/// at `font_px * supersample`. The higher internal resolution removes the ≤1px
/// seams between box-drawing glyphs and yields crisp, anti-aliased text — this
/// is the path to use when the PNG is meant to be looked at.
pub fn render_png_supersampled(
    screen: &Screen,
    fonts: &FontCache,
    font_px: f32,
    supersample: u32,
    theme: &Theme,
) -> Result<Vec<u8>> {
    let ss = supersample.max(1);
    let buf = render_buffer(screen, fonts, font_px * ss as f32, theme)?;
    if ss == 1 {
        return encode_png(buf.image, buf.width, buf.height);
    }
    let out_w = buf.width / ss;
    let out_h = buf.height / ss;
    let hi = image::RgbaImage::from_raw(buf.width, buf.height, buf.image).ok_or_else(|| {
        anyhow::anyhow!("rendered buffer dims {}x{} mismatch", buf.width, buf.height)
    })?;
    let lo = image::imageops::resize(&hi, out_w, out_h, image::imageops::FilterType::Lanczos3);
    encode_png(lo.into_raw(), out_w, out_h)
}

/// Render the screen to a graphics-protocol escape string for `protocol`, or
/// `None` when the protocol is unsupported / off (caller should render a PNG).
pub fn render_graphics(
    screen: &Screen,
    fonts: &FontCache,
    font_px: f32,
    protocol: GraphicsProtocol,
    theme: &Theme,
) -> Option<String> {
    if !protocol.supported() {
        return None;
    }
    let png = render_png(screen, fonts, font_px, theme).ok()?;
    let (cell_w, cell_h) = cell_metrics(fonts);
    let req = GraphicsRequest {
        bytes: &png,
        pixel_w: screen.cols as u32 * cell_w,
        pixel_h: screen.rows as u32 * cell_h,
        cells_w: screen.cols as u32,
        cells_h: screen.rows as u32,
    };
    graphics::encode(protocol, &req)
}

struct Buffer {
    image: Vec<u8>,
    width: u32,
    height: u32,
}

fn render_buffer(
    screen: &Screen,
    fonts: &FontCache,
    font_px: f32,
    theme: &Theme,
) -> Result<Buffer> {
    let (cell_w, cell_h) = cell_metrics(fonts);
    let width = screen.cols as u32 * cell_w;
    let height = screen.rows as u32 * cell_h;

    let [br, bg, bb] = theme.background;
    let canvas = Rgba([br, bg, bb, 255]);
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(width, height, canvas);

    for row in 0..screen.rows {
        for col in 0..screen.cols {
            let cell = &screen.cells[row * screen.cols + col];
            // Right-hand half of a wide (CJK) glyph: already drawn from the
            // left cell — skip it so we don't overwrite it.
            if cell.wide_cont {
                continue;
            }
            let ch = if cell.ch == '\0' { ' ' } else { cell.ch };
            let x0 = col as u32 * cell_w;
            let y0 = row as u32 * cell_h;
            // Wide glyphs span two cells (Latin = 1, CJK/full-width = 2).
            let span = if crate::screen::char_width(ch) == 2 {
                2
            } else {
                1
            };

            // Background fill — only for cells that set an explicit non-default
            // bg (index 0 == "no bg", i.e. the canvas colour shows through).
            if cell.bg != 0 {
                let [r, g, b] = theme.ansi[(cell.bg & 0x0f) as usize];
                fill_rect(
                    &mut img,
                    x0,
                    y0,
                    cell_w * span,
                    cell_h,
                    Rgba([r, g, b, 255]),
                );
            }

            if ch == ' ' {
                continue;
            }

            let [r, g, b] = theme.ansi[(cell.fg & 0x0f) as usize];
            let fg = Rgba([r, g, b, 255]);

            // draw_char emits absolute pixel coords (already positioned), so no
            // extra offset here. A wide CJK glyph has ~2× the advance and so
            // naturally fills the double-wide span when drawn from x0.
            let drew = fonts.draw_char(ch, x0 as f32, y0 as f32, |px, py, cov| {
                if cov <= 0.0 || px >= width || py >= height {
                    return;
                }
                blend(img.get_pixel_mut(px, py), fg, cov);
            });
            if !drew {
                fill_rect(&mut img, x0, y0, cell_w * span, cell_h, fg);
            }
        }
    }

    // ── inline images (kitty / iTerm2 / Sixel) ──────────────────
    for p in screen.image_store.placements() {
        if let Some(inline) = screen.image_store.image(p.image_id) {
            let Ok(decoded) = image::load_from_memory(&inline.data) else {
                continue;
            };
            // The cell region the kitty/iTerm2 protocol asked us to fill.
            // Sixel placements use pixel dimensions (cells=1); honour those
            // when they look like real pixel hints rather than placeholders.
            let (region_w, region_h) = if p.pixel_w > 0 && p.pixel_h > 0 && p.cells_w <= 1 {
                // Sixel: use source pixel size scaled to the cell grid.
                let scale_w = cell_w as f64;
                let scale_h = cell_h as f64;
                (
                    ((p.pixel_w as f64 / 8.0).ceil() * scale_w) as u32, // assume ~8px/char
                    ((p.pixel_h as f64 / 16.0).ceil() * scale_h) as u32,
                )
            } else {
                ((p.cells_w * cell_w) as u32, (p.cells_h * cell_h) as u32)
            };
            if region_w == 0 || region_h == 0 {
                continue;
            }
            let x = (p.col as u32) * cell_w;
            let y = (p.row as u32) * cell_h;
            if x >= width || y >= height {
                continue;
            }
            // Contain-fit: scale the source image to the largest size that
            // fits inside the cell region WITHOUT distorting the aspect ratio,
            // then centre it. Terminal cells are not square (e.g. 48×64 px),
            // so c=10,r=10 is not a square pixel area — a naive stretch makes
            // logos look squished.
            let src_w = decoded.width();
            let src_h = decoded.height();
            let scale = (region_w as f64 / src_w as f64)
                .min(region_h as f64 / src_h as f64);
            let fit_w = ((src_w as f64) * scale).round().max(1.0) as u32;
            let fit_h = ((src_h as f64) * scale).round().max(1.0) as u32;
            let off_x = x as i64 + ((region_w - fit_w) / 2) as i64;
            let off_y = y as i64 + ((region_h - fit_h) / 2) as i64;
            let resized = if src_w == fit_w && src_h == fit_h {
                decoded
            } else {
                image::DynamicImage::ImageRgba8(image::imageops::resize(
                    &decoded,
                    fit_w,
                    fit_h,
                    image::imageops::FilterType::Lanczos3,
                ))
            };
            image::imageops::overlay(&mut img, &resized, off_x, off_y);
        }
    }

    // The `font_px` arg is only used to size the fallback block path; the real
    // metrics come from the loaded FontCache.
    let _ = font_px;

    Ok(Buffer {
        image: img.into_raw(),
        width,
        height,
    })
}

fn encode_png(raw: Vec<u8>, width: u32, height: u32) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    use image::ImageEncoder;
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgba8)?;
    Ok(buf)
}

/// Cell width = rounded advance of `M`; cell height = rounded em
/// (`ascent − descent`). With no font, fall back to a block derived from `px`.
fn cell_metrics(fonts: &FontCache) -> (u32, u32) {
    use ab_glyph::ScaleFont;
    if let Some(face) = fonts.primary() {
        let cell_w = face.h_advance(face.glyph_id('M')).round().max(4.0) as u32;
        let cell_h = (face.ascent() - face.descent()).round().max(8.0) as u32;
        (cell_w, cell_h)
    } else {
        (8, 16)
    }
}

fn fill_rect(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x0: u32,
    y0: u32,
    w: u32,
    h: u32,
    color: Rgba<u8>,
) {
    let (width, height) = img.dimensions();
    let x_max = (x0 + w).min(width);
    let y_max = (y0 + h).min(height);
    for y in y0..y_max {
        for x in x0..x_max {
            *img.get_pixel_mut(x, y) = color;
        }
    }
}

fn blend(dst: &mut Rgba<u8>, src: Rgba<u8>, cov: f32) {
    let a = cov.clamp(0.0, 1.0);
    dst.0[0] = (dst.0[0] as f32 * (1.0 - a) + src.0[0] as f32 * a) as u8;
    dst.0[1] = (dst.0[1] as f32 * (1.0 - a) + src.0[1] as f32 * a) as u8;
    dst.0[2] = (dst.0[2] as f32 * (1.0 - a) + src.0[2] as f32 * a) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_lookup_is_separator_insensitive() {
        assert_eq!(theme_by_name("Solarized Dark").name, "Solarized Dark");
        assert_eq!(theme_by_name("solarized-dark").name, "Solarized Dark");
        assert_eq!(theme_by_name("SOLARIZED_DARK").name, "Solarized Dark");
        assert_eq!(theme_by_name("one-half-dark").name, "One Half Dark");
        assert_eq!(theme_by_name("onehalfdark").name, "One Half Dark");
    }

    #[test]
    fn unknown_theme_falls_back_to_campbell() {
        assert_eq!(theme_by_name("nope-not-a-theme").name, "Campbell");
        assert_eq!(theme_by_name("").name, "Campbell");
    }

    #[test]
    fn default_and_ibm_aliases_resolve() {
        assert_eq!(theme_by_name("default").name, "xterm");
        assert_eq!(theme_by_name("ibm-5153").name, "IBM 5153");
        assert_eq!(theme_by_name("ibm5153").name, "IBM 5153");
    }

    #[test]
    fn themes_have_distinct_backgrounds() {
        // Sanity: the palette table actually varies between schemes, so theme
        // selection has a visible effect (catches accidental copy-paste).
        let mut bgs: Vec<[u8; 3]> = THEMES.iter().map(|t| t.background).collect();
        let total = bgs.len();
        bgs.sort();
        bgs.dedup();
        // Several themes share a #000000 background (Vintage, Tango Dark,
        // CGA, IBM 5153, Ottosson…), so we only assert *some* uniqueness.
        assert!(bgs.len() > total / 2, "themes collapsed: {bgs:?}");
    }
}
