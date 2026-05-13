//! Sky / environment shading for the curved renderer.
//!
//! `RayOutcome::Escaped` photons need a color from the sky direction. The
//! default is a cheap procedural starfield (`ProceduralSky`); for real
//! lensing demos use `HdriSky`, an equirectangular environment map loaded
//! from a Radiance `.hdr` (or any HDR format `image::open` recognises).

use nalgebra::Vector3;
use std::path::Path;

/// Linear-space color for an escaped photon direction (unit vector).
///
/// Implementations must be cheap to call from the hot path — one sample
/// per pixel-sample per `Escaped` outcome.
pub trait Sky: Send + Sync {
    fn sample(&self, dir: &Vector3<f64>) -> [f64; 3];
}

/// Hashed starfield over a horizon→zenith blue gradient. The original
/// `sky_color` lives here unchanged so renders without an HDRI loaded
/// look identical to before this module existed.
#[derive(Clone, Copy, Default)]
pub struct ProceduralSky;

impl Sky for ProceduralSky {
    fn sample(&self, dir: &Vector3<f64>) -> [f64; 3] {
        let hash = |x: f64| -> u64 {
            let bits = x.to_bits();
            bits.wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407)
        };
        let h = hash(dir[0]) ^ hash(dir[1]).rotate_left(17) ^ hash(dir[2]).rotate_left(34);
        if h % 512 < 3 {
            let b = (180 + (h % 75) as u32) as f64 / 255.0;
            [b, b, b]
        } else {
            let v = dir[2].abs() * 15.0 / 255.0;
            [0.0, 0.0, 10.0 / 255.0 + v]
        }
    }
}

/// Equirectangular HDR environment map.
///
/// Texel `(0, 0)` corresponds to direction `(0, 0, +1)` (the +z pole at
/// the top of the image), columns wrap in azimuth and rows run from the
/// north pole to the south pole. Linear-space `f32` pixels, bilinear
/// sampled with wrap-x / clamp-y.
pub struct HdriSky {
    width: usize,
    height: usize,
    pixels: Vec<[f32; 3]>,
    intensity: f32,
}

impl HdriSky {
    /// Load via `image::open` (Radiance `.hdr` works out of the box). The
    /// image is converted to `Rgb32F` so values stay linear and unclamped.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, image::ImageError> {
        let img = image::open(path)?.into_rgb32f();
        let (w, h) = img.dimensions();
        let raw = img.into_raw();
        let pixels: Vec<[f32; 3]> = raw.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
        Ok(Self {
            width: w as usize,
            height: h as usize,
            pixels,
            intensity: 1.0,
        })
    }

    /// Build from already-decoded pixels (row-major, row 0 = +z pole).
    /// Mainly here so tests can construct a tiny sky without a file.
    pub fn from_pixels(width: usize, height: usize, pixels: Vec<[f32; 3]>) -> Self {
        assert_eq!(pixels.len(), width * height, "pixel buffer size mismatch");
        Self { width, height, pixels, intensity: 1.0 }
    }

    /// Scale all samples by `intensity` (linear). Default is 1.0.
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    fn fetch(&self, u: f64, v: f64) -> [f32; 3] {
        let w = self.width as f64;
        let h = self.height as f64;
        let fx = (u * w).rem_euclid(w);
        let fy = (v * h).clamp(0.0, h - 1.0);
        let x0 = (fx.floor() as usize) % self.width;
        let y0 = fy.floor() as usize;
        let x1 = (x0 + 1) % self.width;
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = (fx - fx.floor()) as f32;
        let ty = (fy - fy.floor()) as f32;
        let idx = |y: usize, x: usize| y * self.width + x;
        let p00 = self.pixels[idx(y0, x0)];
        let p10 = self.pixels[idx(y0, x1)];
        let p01 = self.pixels[idx(y1, x0)];
        let p11 = self.pixels[idx(y1, x1)];
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let r = lerp(lerp(p00[0], p10[0], tx), lerp(p01[0], p11[0], tx), ty);
        let g = lerp(lerp(p00[1], p10[1], tx), lerp(p01[1], p11[1], tx), ty);
        let b = lerp(lerp(p00[2], p10[2], tx), lerp(p01[2], p11[2], tx), ty);
        [r, g, b]
    }
}

impl Sky for HdriSky {
    fn sample(&self, dir: &Vector3<f64>) -> [f64; 3] {
        // Camera "up" is +z. Equirectangular mapping:
        //   theta = acos(z), 0 at +z pole (top row), π at -z pole.
        //   phi   = atan2(y, x); u wraps around the azimuth.
        let z = dir[2].clamp(-1.0, 1.0);
        let theta = z.acos();
        let phi = dir[1].atan2(dir[0]);
        let u = phi * std::f64::consts::FRAC_1_PI * 0.5 + 0.5;
        let v = theta * std::f64::consts::FRAC_1_PI;
        let px = self.fetch(u, v);
        let s = self.intensity;
        [(px[0] * s) as f64, (px[1] * s) as f64, (px[2] * s) as f64]
    }
}

/// `'static` `ProceduralSky` the renderer falls back to when no sky is
/// set on `RenderOptions`. Lets `shade_outcome_linear` hand back a
/// `&'static dyn Sky` without allocating per call.
pub static DEFAULT_SKY: ProceduralSky = ProceduralSky;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdri_sky_constant_color() {
        let pixels = vec![[0.4_f32, 0.6, 0.8]; 4 * 2];
        let sky = HdriSky::from_pixels(4, 2, pixels);
        let c = sky.sample(&Vector3::new(1.0, 0.0, 0.0));
        assert!((c[0] - 0.4).abs() < 1e-4);
        assert!((c[1] - 0.6).abs() < 1e-4);
        assert!((c[2] - 0.8).abs() < 1e-4);
    }

    #[test]
    fn hdri_sky_pole_is_top_row() {
        // Top row red, bottom row blue. Direction +z should sample red.
        let mut pixels = vec![[0.0_f32; 3]; 8];
        for p in &mut pixels[..4] { *p = [1.0, 0.0, 0.0]; }
        for p in &mut pixels[4..] { *p = [0.0, 0.0, 1.0]; }
        let sky = HdriSky::from_pixels(4, 2, pixels);
        let top = sky.sample(&Vector3::new(0.0, 0.0, 1.0));
        assert!(top[0] > 0.9 && top[2] < 0.1, "+z should hit the red top row, got {top:?}");
        let bot = sky.sample(&Vector3::new(0.0, 0.0, -1.0));
        assert!(bot[2] > 0.9 && bot[0] < 0.1, "-z should hit the blue bottom row, got {bot:?}");
    }

    #[test]
    fn hdri_sky_intensity_scales() {
        let sky = HdriSky::from_pixels(2, 2, vec![[0.5_f32, 0.25, 0.1]; 4]).with_intensity(2.0);
        let c = sky.sample(&Vector3::new(1.0, 0.0, 0.0));
        assert!((c[0] - 1.0).abs() < 1e-4, "got {}", c[0]);
        assert!((c[1] - 0.5).abs() < 1e-4);
    }
}
