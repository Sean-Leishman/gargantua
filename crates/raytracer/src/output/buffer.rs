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

    /// Build image from parallel row iterator
    pub fn from_rows(width: u32, height: u32, rows: Vec<Vec<Color>>) -> Self {
        let mut buffer = Self::new(width, height);

        for (y, row) in rows.into_iter().enumerate() {
            for (x, color) in row.into_iter().enumerate() {
                buffer.set_pixel_color(x, y, color);
            }
        }

        buffer
    }

    /// Save as PPM (no external dependencies)
    pub fn save_ppm(&self, path: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(path)?;

        writeln!(file, "P3")?;
        writeln!(file, "{} {}", self.width, self.height)?;
        writeln!(file, "255")?;

        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let [r, g, b] = self.get_pixel(x, y);
                writeln!(file, "{} {} {}", r, g, b)?;
            }
        }

        Ok(())
    }
}
