use rayon::prelude::*;

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

    /// Build from parallel row iterator. Flattens row Vecs into the
    /// internal flat buffer in parallel (avoids per-pixel index math).
    pub fn from_rows(width: u32, height: u32, rows: Vec<Vec<Color>>) -> Self {
        let pixels: Vec<Color> = rows.into_par_iter().flatten().collect();
        debug_assert_eq!(pixels.len(), (width * height) as usize);
        Self { width, height, pixels }
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
        self.pixels
            .par_iter_mut()
            .zip(blurred.par_iter())
            .for_each(|(p, b)| *p = *p + *b * intensity);
    }

    /// Extract pixels above brightness threshold
    fn extract_bright(&self, threshold: f64) -> Vec<Color> {
        self.pixels
            .par_iter()
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

        // Horizontal pass — clamp x. Each output row depends only on its
        // own input row, so rows are independent and we parallelize them.
        let mut temp = vec![Color::BLACK; pixels.len()];
        let row_stride = w as usize;
        temp.par_chunks_mut(row_stride)
            .enumerate()
            .for_each(|(y, out_row)| {
                let in_row = &pixels[y * row_stride..(y + 1) * row_stride];
                for x in 0..w {
                    let mut sum = Color::BLACK;
                    for (i, &k) in kernel.iter().enumerate() {
                        let dx = i as i32 - r;
                        let nx = (x + dx).clamp(0, w - 1);
                        sum = sum + in_row[nx as usize] * k;
                    }
                    out_row[x as usize] = sum;
                }
            });

        // Vertical pass — clamp y. Output rows are again independent of
        // each other (each reads a vertical span of `temp`).
        let mut result = vec![Color::BLACK; pixels.len()];
        result
            .par_chunks_mut(row_stride)
            .enumerate()
            .for_each(|(y, out_row)| {
                let y = y as i32;
                for x in 0..w {
                    let mut sum = Color::BLACK;
                    for (i, &k) in kernel.iter().enumerate() {
                        let dy = i as i32 - r;
                        let ny = (y + dy).clamp(0, h - 1);
                        sum = sum + temp[(ny * w + x) as usize] * k;
                    }
                    out_row[x as usize] = sum;
                }
            });
        result
    }

    /// Apply ACES tone mapping for better HDR handling
    pub fn apply_aces_tonemapping(&mut self) {
        self.pixels.par_iter_mut().for_each(|p| *p = aces_tonemap(*p));
    }

    /// Apply Reinhard tone mapping
    pub fn apply_reinhard_tonemapping(&mut self) {
        self.pixels.par_iter_mut().for_each(|p| {
            p.r = p.r / (1.0 + p.r);
            p.g = p.g / (1.0 + p.g);
            p.b = p.b / (1.0 + p.b);
        });
    }

    /// Apply an exposure adjustment in stops: linear scale by 2^ev.
    /// Conventionally applied before tone mapping.
    pub fn apply_exposure(&mut self, ev: f64) {
        let scale = 2.0_f64.powf(ev);
        self.pixels.par_iter_mut().for_each(|p| *p = *p * scale);
    }

    /// Convert to LDR ImageBuffer with the legacy `pow(1/gamma)` curve.
    /// Prefer `to_image_buffer_srgb` for new code.
    pub fn to_image_buffer(&self, gamma: f64) -> super::ImageBuffer {
        let pixels: Vec<[u8; 3]> = self
            .pixels
            .par_iter()
            .map(|c| c.to_rgb_gamma(gamma))
            .collect();
        super::ImageBuffer::from_pixels(self.width, self.height, pixels)
    }

    /// Convert to LDR using proper sRGB OETF. Assumes pixels are already
    /// tone-mapped into roughly [0,1].
    pub fn to_image_buffer_srgb(&self) -> super::ImageBuffer {
        let pixels: Vec<[u8; 3]> = self
            .pixels
            .par_iter()
            .map(|c| c.to_srgb_u8())
            .collect();
        super::ImageBuffer::from_pixels(self.width, self.height, pixels)
    }

    /// Run Intel Open Image Denoise over the HDR color buffer in place.
    /// Color-only mode (no albedo/normal AOVs). HDR-aware: pass `true`
    /// for `hdr` so OIDN treats values > 1 as physical radiance rather
    /// than display-referred. Requires the `denoise` cargo feature.
    #[cfg(feature = "denoise")]
    pub fn denoise(&mut self, hdr: bool) {
        let n = (self.width * self.height) as usize;
        // OIDN takes interleaved f32 RGB.
        let mut input: Vec<f32> = Vec::with_capacity(n * 3);
        for c in &self.pixels {
            input.push(c.r as f32);
            input.push(c.g as f32);
            input.push(c.b as f32);
        }
        let mut output: Vec<f32> = vec![0.0; n * 3];

        let device = oidn::Device::new();
        let mut filter = oidn::RayTracing::new(&device);
        filter
            .image_dimensions(self.width as usize, self.height as usize)
            .hdr(hdr)
            .clean_aux(false);
        filter
            .filter(&input, &mut output)
            .expect("OIDN filter failed");

        for (i, c) in self.pixels.iter_mut().enumerate() {
            let base = i * 3;
            c.r = output[base] as f64;
            c.g = output[base + 1] as f64;
            c.b = output[base + 2] as f64;
        }
    }

    /// No-op stub when the `denoise` feature is off.
    #[cfg(not(feature = "denoise"))]
    pub fn denoise(&mut self, _hdr: bool) {}

    /// One-shot finalize: exposure → tone map → sRGB encode.
    /// Does not mutate `self`.
    pub fn finalize(&self, exposure_ev: f64, tonemap: ToneMap) -> super::ImageBuffer {
        let scale = 2.0_f64.powf(exposure_ev);
        let pixels: Vec<[u8; 3]> = self
            .pixels
            .par_iter()
            .map(|c| {
                let c = *c * scale;
                let c = match tonemap {
                    ToneMap::None => c,
                    ToneMap::Reinhard => Color::new(
                        c.r / (1.0 + c.r),
                        c.g / (1.0 + c.g),
                        c.b / (1.0 + c.b),
                    ),
                    ToneMap::Aces => aces_tonemap(c),
                };
                c.to_srgb_u8()
            })
            .collect();
        super::ImageBuffer::from_pixels(self.width, self.height, pixels)
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
