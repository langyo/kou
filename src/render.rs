//! Screen rendering.
//!
//! Two backends share one cell grid:
//!
//! - [`render_png`] rasterises the screen to PNG bytes using a [`FontCache`]
//!   (proper glyph outlines, an xterm-like 16-colour palette, CJK fallback).
//!   This replaces the old "draw a grey rectangle per non-empty cell" stub.
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

/// xterm-style 16-colour palette (indices 0..=15). A screen cell stores a `u8`
/// colour index; the low nibble selects the colour, the palette covers both the
/// normal and bright halves so callers can set either.
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

/// Render the screen to a PNG byte buffer.
///
/// `font_px` is the requested pixel size. Cell dimensions come from the loaded
/// font's metrics (advance of `M`, line height); with no font loaded, glyphs
/// degrade to solid blocks sized from `font_px`.
pub fn render_png(screen: &Screen, fonts: &FontCache, font_px: f32) -> Result<Vec<u8>> {
    let (cell_w, cell_h) = cell_metrics(fonts, font_px);
    let width = screen.cols as u32 * cell_w;
    let height = screen.rows as u32 * cell_h;

    let bg = Rgba([24, 24, 28, 255]);
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(width, height, bg);

    for row in 0..screen.rows {
        for col in 0..screen.cols {
            let cell = &screen.cells[row * screen.cols + col];
            let ch = if cell.ch == '\0' { ' ' } else { cell.ch };
            let x0 = col as u32 * cell_w;
            let y0 = row as u32 * cell_h;

            if cell.bg != 0 {
                let (r, g, b) = palette(cell.bg);
                fill_rect(&mut img, x0, y0, cell_w, cell_h, Rgba([r, g, b, 255]));
            }

            if ch == ' ' {
                continue;
            }

            let (r, g, b) = palette(cell.fg);
            let fg = Rgba([r, g, b, 255]);

            let drew = fonts.draw_char(ch, x0 as f32, y0 as f32, |px, py, cov| {
                if cov <= 0.0 {
                    return;
                }
                let px = x0 + px;
                let py = y0 + py;
                if px < width && py < height {
                    blend(img.get_pixel_mut(px, py), fg, cov);
                }
            });
            if !drew {
                // No font / no coverage for the codepoint: leave a visible block.
                fill_rect(&mut img, x0, y0, cell_w, cell_h, fg);
            }
        }
    }

    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    use image::ImageEncoder;
    encoder.write_image(img.as_raw(), width, height, image::ExtendedColorType::Rgba8)?;
    Ok(buf)
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
    let (cell_w, cell_h) = cell_metrics(fonts, font_px);
    let req = GraphicsRequest {
        bytes: &png,
        pixel_w: screen.cols as u32 * cell_w,
        pixel_h: screen.rows as u32 * cell_h,
        cells_w: screen.cols as u32,
        cells_h: screen.rows as u32,
    };
    graphics::encode(protocol, &req)
}

fn cell_metrics(fonts: &FontCache, font_px: f32) -> (u32, u32) {
    use ab_glyph::ScaleFont;
    if let Some(face) = fonts.primary() {
        let cell_h = face.height().ceil().max(8.0) as u32;
        let cell_w = face.h_advance(face.glyph_id('M')).ceil().max(4.0) as u32;
        (cell_w, cell_h)
    } else {
        let h = (font_px.round().max(8.0) as u32) * 2;
        (h / 2, h)
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
