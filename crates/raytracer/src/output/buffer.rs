use rayon::prelude::*;

use crate::core::Color;

/// A simple image buffer that stores RGB pixels
#[derive(Clone)]
pub struct ImageBuffer {
    width: u32,
    height: u32,
    pixels: Vec<[u8; 3]>,
}

impl ImageBuffer {
    /// Create a new black image buffer
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![[0, 0, 0]; (width * height) as usize],
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Set pixel at (x, y) - origin is top-left
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: [u8; 3]) {
        let idx = y * self.width as usize + x;
        self.pixels[idx] = rgb;
    }

    /// Get pixel at (x, y)
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let idx = y * self.width as usize + x;
        self.pixels[idx]
    }

    /// Set pixel from a Color (applies gamma correction)
    #[inline]
    pub fn set_pixel_color(&mut self, x: usize, y: usize, color: Color) {
        self.set_pixel(x, y, color.to_rgb_gamma(2.2));
    }

    /// Construct directly from a flat pixel buffer. `pixels.len()` must equal `width * height`.
    pub fn from_pixels(width: u32, height: u32, pixels: Vec<[u8; 3]>) -> Self {
        debug_assert_eq!(pixels.len(), (width * height) as usize);
        Self { width, height, pixels }
    }

    pub(crate) fn pixels_mut(&mut self) -> &mut Vec<[u8; 3]> {
        &mut self.pixels
    }

    /// Build image from per-row HDR colors via parallel gamma encode.
    pub fn from_rows(width: u32, height: u32, rows: Vec<Vec<Color>>) -> Self {
        let pixels: Vec<[u8; 3]> = rows
            .into_par_iter()
            .flat_map_iter(|row| row.into_iter().map(|c| c.to_rgb_gamma(2.2)))
            .collect();
        Self::from_pixels(width, height, pixels)
    }

    /// Save as PNG (or any format inferred from the file extension by the `image` crate).
    pub fn save_png(&self, path: &str) -> image::ImageResult<()> {
        // Flatten our [u8;3] vec into the contiguous Vec<u8> the `image` crate
        // wants. `from_raw` is O(1) when the buffer is already the right size,
        // so this avoids the per-pixel `put_pixel` call we used to do.
        let mut raw: Vec<u8> = Vec::with_capacity(self.pixels.len() * 3);
        for px in &self.pixels {
            raw.extend_from_slice(px);
        }
        let buf = image::RgbImage::from_raw(self.width, self.height, raw)
            .expect("pixel buffer size mismatch");
        buf.save(path)
    }

    /// Save as PPM (no external dependencies)
    pub fn save_ppm(&self, path: &str) -> std::io::Result<()> {
        use std::io::{BufWriter, Write};
        let file = std::fs::File::create(path)?;
        let mut w = BufWriter::new(file);

        writeln!(w, "P3")?;
        writeln!(w, "{} {}", self.width, self.height)?;
        writeln!(w, "255")?;

        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let [r, g, b] = self.get_pixel(x, y);
                writeln!(w, "{} {} {}", r, g, b)?;
            }
        }

        w.flush()
    }
}
