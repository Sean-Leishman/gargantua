use crate::core::Color;

/// HDR image buffer for post-processing
pub struct HdrBuffer {
    width: u32,
    height: u32,
    pixels: Vec<Color>,
}

impl HdrBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![Color::BLACK; (width * height) as usize],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let idx = y * self.width as usize + x;
        self.pixels[idx] = color;
    }

    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        let idx = y * self.width as usize + x;
        self.pixels[idx]
    }

    /// Build from parallel row iterator
    pub fn from_rows(width: u32, height: u32, rows: Vec<Vec<Color>>) -> Self {
        let mut buffer = Self::new(width, height);
        for (y, row) in rows.into_iter().enumerate() {
            for (x, color) in row.into_iter().enumerate() {
                buffer.set_pixel(x, y, color);
            }
        }
        buffer
    }

    /// Apply bloom effect
    ///
    /// # Arguments
    /// * `threshold` - Minimum brightness to bloom (0.0 - 1.0, typically 0.8-1.0)
    /// * `intensity` - Bloom intensity (0.0 - 1.0, typically 0.3-0.5)
    /// * `radius` - Bloom radius in pixels
    pub fn apply_bloom(&mut self, threshold: f64, intensity: f64, radius: u32) {
        // Extract bright pixels
        let bright = self.extract_bright(threshold);

        // Blur the bright pixels (box blur for simplicity, could use Gaussian)
        let blurred = Self::blur(&bright, self.width, self.height, radius);

        // Add blurred bright pixels back to original
        for i in 0..self.pixels.len() {
            self.pixels[i] = self.pixels[i] + blurred[i] * intensity;
        }
    }

    /// Extract pixels above brightness threshold
    fn extract_bright(&self, threshold: f64) -> Vec<Color> {
        self.pixels
            .iter()
            .map(|c| {
                let brightness = c.luminance();
                if brightness > threshold {
                    // Keep the excess brightness
                    let excess = brightness - threshold;
                    *c * (excess / brightness)
                } else {
                    Color::BLACK
                }
            })
            .collect()
    }

    /// Separable Gaussian blur. `radius` sets σ ≈ radius/2 and the kernel
    /// half-width to `radius`. Edges use clamp-to-edge addressing so the
    /// kernel weights always sum to 1 (no brightness drop at borders).
    fn blur(pixels: &[Color], width: u32, height: u32, radius: u32) -> Vec<Color> {
        if radius == 0 {
            return pixels.to_vec();
        }
        let w = width as i32;
        let h = height as i32;
        let r = radius as i32;

        // σ ≈ radius/2 — the standard "radius covers ~2σ" convention.
        let sigma = (radius as f64 * 0.5).max(1e-3);
        let inv_two_sigma_sq = 1.0 / (2.0 * sigma * sigma);
        let mut kernel: Vec<f64> = (-r..=r)
            .map(|i| (-(i * i) as f64 * inv_two_sigma_sq).exp())
            .collect();
        let ksum: f64 = kernel.iter().sum();
        for k in &mut kernel { *k /= ksum; }

        let mut temp = vec![Color::BLACK; pixels.len()];
        // Horizontal pass — clamp x.
        for y in 0..h {
            let row = (y * w) as usize;
            for x in 0..w {
                let mut sum = Color::BLACK;
                for (i, &k) in kernel.iter().enumerate() {
                    let dx = i as i32 - r;
                    let nx = (x + dx).clamp(0, w - 1);
                    sum = sum + pixels[row + nx as usize] * k;
                }
                temp[row + x as usize] = sum;
            }
        }

        let mut result = vec![Color::BLACK; pixels.len()];
        // Vertical pass — clamp y.
        for y in 0..h {
            for x in 0..w {
                let mut sum = Color::BLACK;
                for (i, &k) in kernel.iter().enumerate() {
                    let dy = i as i32 - r;
                    let ny = (y + dy).clamp(0, h - 1);
                    sum = sum + temp[(ny * w + x) as usize] * k;
                }
                result[(y * w + x) as usize] = sum;
            }
        }
        result
    }

    /// Apply ACES tone mapping for better HDR handling
    pub fn apply_aces_tonemapping(&mut self) {
        for pixel in &mut self.pixels {
            *pixel = aces_tonemap(*pixel);
        }
    }

    /// Apply Reinhard tone mapping
    pub fn apply_reinhard_tonemapping(&mut self) {
        for pixel in &mut self.pixels {
            pixel.r = pixel.r / (1.0 + pixel.r);
            pixel.g = pixel.g / (1.0 + pixel.g);
            pixel.b = pixel.b / (1.0 + pixel.b);
        }
    }

    /// Apply an exposure adjustment in stops: linear scale by 2^ev.
    /// Conventionally applied before tone mapping.
    pub fn apply_exposure(&mut self, ev: f64) {
        let scale = 2.0_f64.powf(ev);
        for pixel in &mut self.pixels {
            *pixel = *pixel * scale;
        }
    }

    /// Convert to LDR ImageBuffer with the legacy `pow(1/gamma)` curve.
    /// Prefer `to_image_buffer_srgb` for new code.
    pub fn to_image_buffer(&self, gamma: f64) -> super::ImageBuffer {
        let mut buffer = super::ImageBuffer::new(self.width, self.height);
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let color = self.get_pixel(x, y);
                buffer.set_pixel(x, y, color.to_rgb_gamma(gamma));
            }
        }
        buffer
    }

    /// Convert to LDR using proper sRGB OETF. Assumes pixels are already
    /// tone-mapped into roughly [0,1].
    pub fn to_image_buffer_srgb(&self) -> super::ImageBuffer {
        let mut buffer = super::ImageBuffer::new(self.width, self.height);
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let color = self.get_pixel(x, y);
                buffer.set_pixel(x, y, color.to_srgb_u8());
            }
        }
        buffer
    }

    /// One-shot finalize: exposure → tone map → sRGB encode.
    /// Does not mutate `self`.
    pub fn finalize(&self, exposure_ev: f64, tonemap: ToneMap) -> super::ImageBuffer {
        let scale = 2.0_f64.powf(exposure_ev);
        let mut buffer = super::ImageBuffer::new(self.width, self.height);
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let mut c = self.get_pixel(x, y) * scale;
                c = match tonemap {
                    ToneMap::None => c,
                    ToneMap::Reinhard => Color::new(
                        c.r / (1.0 + c.r),
                        c.g / (1.0 + c.g),
                        c.b / (1.0 + c.b),
                    ),
                    ToneMap::Aces => aces_tonemap(c),
                };
                buffer.set_pixel(x, y, c.to_srgb_u8());
            }
        }
        buffer
    }
}

/// Tone mapping operator selector.
#[derive(Clone, Copy, Debug, Default)]
pub enum ToneMap {
    None,
    Reinhard,
    #[default]
    Aces,
}

/// ACES Filmic Tone Mapping
fn aces_tonemap(color: Color) -> Color {
    const A: f64 = 2.51;
    const B: f64 = 0.03;
    const C: f64 = 2.43;
    const D: f64 = 0.59;
    const E: f64 = 0.14;

    let tonemap = |x: f64| -> f64 {
        let x = x.max(0.0);
        ((x * (A * x + B)) / (x * (C * x + D) + E)).clamp(0.0, 1.0)
    };

    Color::new(tonemap(color.r), tonemap(color.g), tonemap(color.b))
}
