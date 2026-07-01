//! PNG rendering of terminal screen.

use crate::screen::Screen;
use anyhow::Result;
use image::{ImageBuffer, Rgba};

pub struct Renderer {
    cell_width: u32,
    cell_height: u32,
    font_size: f32,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            cell_width: 8,
            cell_height: 16,
            font_size: 14.0,
        }
    }
}

impl Renderer {
    pub fn render_png(&self, screen: &Screen) -> Result<Vec<u8>> {
        let width = screen.cols as u32 * self.cell_width;
        let height = screen.rows as u32 * self.cell_height;
        
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(width, height, Rgba([30, 30, 30, 255]));

        // Draw text (simplified — full rendering uses ab_glyph)
        for row in 0..screen.rows {
            for col in 0..screen.cols {
                let cell = &screen.cells[row * screen.cols + col];
                if cell.ch != '\0' && cell.ch != ' ' {
                    let x = col as u32 * self.cell_width;
                    let y = row as u32 * self.cell_height;
                    // Simple pixel marker (TODO: proper font rendering)
                    for dx in 0..self.cell_width.min(6) {
                        for dy in 0..self.cell_height.min(12) {
                            img.put_pixel(x + dx, y + dy, Rgba([200, 200, 200, 255]));
                        }
                    }
                }
            }
        }

        let mut buf = Vec::new();
        image::write_buffer_with_encoder(
            &mut buf,
            img.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
            image::ImageFormat::Png,
        )?;
        Ok(buf)
    }
}
