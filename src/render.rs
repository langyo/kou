//! Screen rendering.
//!
//! Two backends share one cell grid:
//!
//! - [`render_png`] rasterises the screen to PNG bytes using a [`FontCache`]
//!   (proper glyph outlines, an xterm-like 16-colour palette, CJK fallback).
//!   Glyph positions are derived from each face's positioned bounding box, so
//!   characters land exactly where they should.
//! - [`render_png_supersampled`] renders at an integer factor then downscales
//!   with Lanczos3, for a crisp, anti-aliased result where box-drawing glyphs
//!   (`─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼`) connect seamlessly across cells instead of looking
//!   like the blocky old-terminal tiles.
//! - [`render_graphics`] emits an inband graphics-protocol escape sequence
//!   (kitty / iTerm2) so a capable terminal renders the pixels inline.
//!
//! The renderer is font-driven: pass a [`FontCache`] loaded from the
//! [`crate::font`] module. When the cache is empty it falls back to drawing a
//! solid block per glyph, so kou still produces *something* without fonts.

use crate::font::FontCache;
use crate::graphics::{self, GraphicsProtocol, GraphicsRequest};
use crate::screen::Screen;
use anyhow::Result;
use image::{ImageBuffer, Rgba};

/// xterm-style 16-colour palette (indices 0..=15).
const PALETTE: [(u8, u8, u8); 16] = [
    (0, 0, 0),       // 0 black
    (205, 0, 0),     // 1 red
    (0, 205, 0),     // 2 green
    (205, 205, 0),   // 3 yellow
    (0, 0, 238),     // 4 blue
    (205, 0, 205),   // 5 magenta
    (0, 205, 205),   // 6 cyan
    (229, 229, 229), // 7 white
    (127, 127, 127), // 8 bright black (grey)
    (255, 0, 0),     // 9 bright red
    (0, 255, 0),     // 10 bright green
    (255, 255, 0),   // 11 bright yellow
    (92, 92, 255),   // 12 bright blue
    (255, 0, 255),   // 13 bright magenta
    (0, 255, 255),   // 14 bright cyan
    (255, 255, 255), // 15 bright white
];

fn palette(idx: u8) -> (u8, u8, u8) {
    PALETTE[(idx & 0x0f) as usize]
}

/// Render the screen to PNG bytes at the font's natural scale (1×).
///
/// `fonts` should be loaded at `font_px`. Cell width is the rounded advance of
/// `M`; cell height is the rounded em (`ascent − descent`) — so box-drawing
/// glyphs, which fill exactly one advance × one em, line up cell-to-cell.
pub fn render_png(screen: &Screen, fonts: &FontCache, font_px: f32) -> Result<Vec<u8>> {
    let buf = render_buffer(screen, fonts, font_px)?;
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
) -> Result<Vec<u8>> {
    let ss = supersample.max(1);
    let buf = render_buffer(screen, fonts, font_px * ss as f32)?;
    if ss == 1 {
        return encode_png(buf.image, buf.width, buf.height);
    }
    let out_w = buf.width / ss;
    let out_h = buf.height / ss;
    let hi = image::RgbaImage::from_raw(buf.width, buf.height, buf.image).unwrap();
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
) -> Option<String> {
    if !protocol.supported() {
        return None;
    }
    let png = render_png(screen, fonts, font_px).ok()?;
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

fn render_buffer(screen: &Screen, fonts: &FontCache, font_px: f32) -> Result<Buffer> {
    let (cell_w, cell_h) = cell_metrics(fonts);
    let width = screen.cols as u32 * cell_w;
    let height = screen.rows as u32 * cell_h;

    let bg = Rgba([24, 24, 28, 255]);
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(width, height, bg);

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

            if cell.bg != 0 {
                let (r, g, b) = palette(cell.bg);
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

            let (r, g, b) = palette(cell.fg);
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
