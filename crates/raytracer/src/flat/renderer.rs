use rand::Rng;
use rayon::prelude::*;

use crate::camera::Camera;
use crate::core::{Color, Hittable, Interval, Ray};
use crate::output::{HdrBuffer, ImageBuffer, ToneMap};
use crate::scene::LightList;

/// Background type for the renderer
#[derive(Clone, Copy, Debug)]
pub enum Background {
    /// Sky gradient from white to blue
    Sky,
    /// Solid black (for enclosed scenes like Cornell Box)
    Black,
    /// Custom solid color
    Solid(Color),
}

/// A rectangular tile for parallel rendering
#[derive(Clone, Copy, Debug)]
struct Tile {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Tile {
    /// Generate tiles covering the entire image
    fn generate(image_width: u32, image_height: u32, tile_size: u32) -> Vec<Tile> {
        let mut tiles = Vec::new();
        let mut y = 0;
        while y < image_height {
            let tile_height = tile_size.min(image_height - y);
            let mut x = 0;
            while x < image_width {
                let tile_width = tile_size.min(image_width - x);
                tiles.push(Tile {
                    x,
                    y,
                    width: tile_width,
                    height: tile_height,
                });
                x += tile_size;
            }
            y += tile_size;
        }
        tiles
    }

    /// Generate tiles in Morton (Z-order) curve for better cache locality
    fn generate_morton(image_width: u32, image_height: u32, tile_size: u32) -> Vec<Tile> {
        let mut tiles = Self::generate(image_width, image_height, tile_size);

        // Sort by Morton code for better spatial locality
        tiles.sort_by_key(|tile| {
            let tx = tile.x / tile_size;
            let ty = tile.y / tile_size;
            morton_encode(tx, ty)
        });

        tiles
    }
}

/// Balance heuristic for multiple importance sampling.
#[inline]
fn mis_balance(p_a: f64, p_b: f64) -> f64 {
    let s = p_a + p_b;
    if s > 0.0 { p_a / s } else { 0.0 }
}

/// Encode 2D coordinates to Morton code (Z-order curve)
#[inline]
fn morton_encode(x: u32, y: u32) -> u64 {
    fn spread_bits(v: u32) -> u64 {
        let mut v = v as u64;
        v = (v | (v << 16)) & 0x0000FFFF0000FFFF;
        v = (v | (v << 8)) & 0x00FF00FF00FF00FF;
        v = (v | (v << 4)) & 0x0F0F0F0F0F0F0F0F;
        v = (v | (v << 2)) & 0x3333333333333333;
        v = (v | (v << 1)) & 0x5555555555555555;
        v
    }
    spread_bits(x) | (spread_bits(y) << 1)
}

/// Sampling strategy for anti-aliasing
#[derive(Clone, Copy, Debug, Default)]
pub enum SamplingStrategy {
    /// Pure random sampling (default)
    #[default]
    Random,
    /// Stratified (jittered) sampling - divides pixel into grid, samples within each cell
    Stratified,
}

/// Flat (Euclidean) ray tracer using path tracing
pub struct FlatRenderer {
    max_depth: u32,
    samples_per_pixel: u32,
    background: Background,
    tile_size: u32,
    sampling: SamplingStrategy,
    use_morton_order: bool,
    /// Exposure (stops, EV). 0 = no change.
    exposure: f64,
    /// Tone-mapping operator applied when producing an LDR ImageBuffer.
    tonemap: ToneMap,
}

impl FlatRenderer {
    pub fn new(max_depth: u32, samples_per_pixel: u32) -> Self {
        Self {
            max_depth,
            samples_per_pixel,
            background: Background::Sky,
            tile_size: 32,
            use_morton_order: true, // Default to Morton for better cache
            sampling: SamplingStrategy::Random,
            exposure: 0.0,
            tonemap: ToneMap::Aces,
        }
    }

    /// Set exposure compensation in stops (EV). Applied before tone mapping.
    pub fn with_exposure(mut self, ev: f64) -> Self {
        self.exposure = ev;
        self
    }

    /// Set the tone-mapping operator used by `render*` paths that emit an
    /// LDR `ImageBuffer`. Defaults to ACES. Pass `ToneMap::None` to keep
    /// linear output (still sRGB-encoded; values > 1 will clamp).
    pub fn with_tonemap(mut self, tm: ToneMap) -> Self {
        self.tonemap = tm;
        self
    }

    /// Set the background type
    pub fn with_background(mut self, bg: Background) -> Self {
        self.background = bg;
        self
    }

    /// Set the tile size for parallel rendering
    pub fn with_tile_size(mut self, tile_size: u32) -> Self {
        self.tile_size = tile_size;
        self
    }

    /// Disable Morton curve tile ordering (uses row order instead)
    pub fn without_morton_order(mut self) -> Self {
        self.use_morton_order = false;
        self
    }

    /// Use stratified (jittered) sampling for better convergence
    pub fn with_stratified_sampling(mut self) -> Self {
        self.sampling = SamplingStrategy::Stratified;
        self
    }

    /// Set sampling strategy
    pub fn with_sampling(mut self, strategy: SamplingStrategy) -> Self {
        self.sampling = strategy;
        self
    }

    /// Generate tiles with optional Morton ordering
    fn get_tiles(&self, width: u32, height: u32) -> Vec<Tile> {
        if self.use_morton_order {
            Tile::generate_morton(width, height, self.tile_size)
        } else {
            Tile::generate(width, height, self.tile_size)
        }
    }

    /// Render a scene to an image buffer using tile-based parallelism
    pub fn render<S: Hittable + Sync, C: Camera + Sync>(
        &self,
        scene: &S,
        camera: &C,
        width: u32,
        height: u32,
    ) -> ImageBuffer {
        let tiles = self.get_tiles(width, height);

        // Precompute stratified grid if needed
        let (grid_size, cell_size) = match self.sampling {
            SamplingStrategy::Stratified => {
                let g = (self.samples_per_pixel as f64).sqrt().ceil() as u32;
                (g, 1.0 / g as f64)
            }
            SamplingStrategy::Random => (0, 0.0),
        };

        // Render tiles in parallel
        let rendered: Vec<(Tile, Vec<Color>)> = tiles
            .into_par_iter()
            .map(|tile| {
                let mut rng = rand::thread_rng();
                let mut pixels = Vec::with_capacity((tile.width * tile.height) as usize);

                for local_y in 0..tile.height {
                    for local_x in 0..tile.width {
                        let i = tile.x + local_x;
                        let j = tile.y + local_y;

                        let mut color = Color::BLACK;
                        let num_samples;

                        match self.sampling {
                            SamplingStrategy::Random => {
                                num_samples = self.samples_per_pixel;
                                for _ in 0..self.samples_per_pixel {
                                    let u = (i as f64 + rng.r#gen::<f64>()) / (width - 1) as f64;
                                    let v = ((height - 1 - j) as f64 + rng.r#gen::<f64>()) / (height - 1) as f64;
                                    let ray = camera.get_ray(u, v);
                                    color += self.trace_with_rng(ray, scene, self.max_depth, &mut rng);
                                }
                            }
                            SamplingStrategy::Stratified => {
                                num_samples = grid_size * grid_size;
                                for sy in 0..grid_size {
                                    for sx in 0..grid_size {
                                        let su = (sx as f64 + rng.r#gen::<f64>()) * cell_size;
                                        let sv = (sy as f64 + rng.r#gen::<f64>()) * cell_size;
                                        let u = (i as f64 + su) / (width - 1) as f64;
                                        let v = ((height - 1 - j) as f64 + sv) / (height - 1) as f64;
                                        let ray = camera.get_ray(u, v);
                                        color += self.trace_with_rng(ray, scene, self.max_depth, &mut rng);
                                    }
                                }
                            }
                        }
                        pixels.push(color / num_samples as f64);
                    }
                }

                (tile, pixels)
            })
            .collect();

        // Assemble tiles into final image
        self.finalize_tiles(width, height, rendered)
    }

    /// Assemble rendered HDR tiles, then exposure → tone-map → sRGB encode.
    fn finalize_tiles(&self, width: u32, height: u32, tiles: Vec<(Tile, Vec<Color>)>) -> ImageBuffer {
        let rows = Self::assemble_rows(width, height, tiles);
        let hdr = HdrBuffer::from_rows(width, height, rows);
        hdr.finalize(self.exposure, self.tonemap)
    }

    /// Assemble rendered tiles into rows of colors (for HDR)
    fn assemble_rows(width: u32, height: u32, tiles: Vec<(Tile, Vec<Color>)>) -> Vec<Vec<Color>> {
        let mut rows: Vec<Vec<Color>> = (0..height).map(|_| vec![Color::BLACK; width as usize]).collect();

        for (tile, pixels) in tiles {
            let mut idx = 0;
            for local_y in 0..tile.height {
                for local_x in 0..tile.width {
                    let x = (tile.x + local_x) as usize;
                    let y = (tile.y + local_y) as usize;
                    rows[y][x] = pixels[idx];
                    idx += 1;
                }
            }
        }

        rows
    }

    /// Render to HDR buffer for post-processing (bloom, tone mapping)
    pub fn render_hdr<S: Hittable + Sync, C: Camera + Sync>(
        &self,
        scene: &S,
        camera: &C,
        width: u32,
        height: u32,
    ) -> HdrBuffer {
        let tiles = self.get_tiles(width, height);

        let rendered: Vec<(Tile, Vec<Color>)> = tiles
            .into_par_iter()
            .map(|tile| {
                let mut rng = rand::thread_rng();
                let mut pixels = Vec::with_capacity((tile.width * tile.height) as usize);

                for local_y in 0..tile.height {
                    for local_x in 0..tile.width {
                        let i = tile.x + local_x;
                        let j = tile.y + local_y;

                        let mut color = Color::BLACK;
                        for _ in 0..self.samples_per_pixel {
                            let u = (i as f64 + rng.r#gen::<f64>()) / (width - 1) as f64;
                            let v =
                                ((height - 1 - j) as f64 + rng.r#gen::<f64>()) / (height - 1) as f64;

                            let ray = camera.get_ray(u, v);
                            color += self.trace_with_rng(ray, scene, self.max_depth, &mut rng);
                        }
                        pixels.push(color / self.samples_per_pixel as f64);
                    }
                }

                (tile, pixels)
            })
            .collect();

        let rows = Self::assemble_rows(width, height, rendered);
        HdrBuffer::from_rows(width, height, rows)
    }

    /// Render to HDR buffer with NEE for post-processing
    pub fn render_hdr_with_lights<S: Hittable + Sync, C: Camera + Sync>(
        &self,
        scene: &S,
        lights: &LightList,
        camera: &C,
        width: u32,
        height: u32,
    ) -> HdrBuffer {
        let tiles = Tile::generate(width, height, self.tile_size);

        let rendered: Vec<(Tile, Vec<Color>)> = tiles
            .into_par_iter()
            .map(|tile| {
                let mut rng = rand::thread_rng();
                let mut pixels = Vec::with_capacity((tile.width * tile.height) as usize);

                for local_y in 0..tile.height {
                    for local_x in 0..tile.width {
                        let i = tile.x + local_x;
                        let j = tile.y + local_y;

                        let mut color = Color::BLACK;
                        for _ in 0..self.samples_per_pixel {
                            let u = (i as f64 + rng.r#gen::<f64>()) / (width - 1) as f64;
                            let v =
                                ((height - 1 - j) as f64 + rng.r#gen::<f64>()) / (height - 1) as f64;

                            let ray = camera.get_ray(u, v);
                            color += self.trace_nee(ray, scene, lights, self.max_depth);
                        }
                        pixels.push(color / self.samples_per_pixel as f64);
                    }
                }

                (tile, pixels)
            })
            .collect();

        let rows = Self::assemble_rows(width, height, rendered);
        HdrBuffer::from_rows(width, height, rows)
    }

    /// Render a scene with explicit light sampling (Next Event Estimation)
    /// This produces lower noise for scenes with small, bright light sources
    pub fn render_with_lights<S: Hittable + Sync, C: Camera + Sync>(
        &self,
        scene: &S,
        lights: &LightList,
        camera: &C,
        width: u32,
        height: u32,
    ) -> ImageBuffer {
        let tiles = Tile::generate(width, height, self.tile_size);

        // Render tiles in parallel
        let rendered: Vec<(Tile, Vec<Color>)> = tiles
            .into_par_iter()
            .map(|tile| {
                let mut rng = rand::thread_rng();
                let mut pixels = Vec::with_capacity((tile.width * tile.height) as usize);

                for local_y in 0..tile.height {
                    for local_x in 0..tile.width {
                        let i = tile.x + local_x;
                        let j = tile.y + local_y;

                        let mut color = Color::BLACK;
                        for _ in 0..self.samples_per_pixel {
                            let u = (i as f64 + rng.r#gen::<f64>()) / (width - 1) as f64;
                            let v =
                                ((height - 1 - j) as f64 + rng.r#gen::<f64>()) / (height - 1) as f64;

                            let ray = camera.get_ray(u, v);
                            color += self.trace_nee(ray, scene, lights, self.max_depth);
                        }
                        pixels.push(color / self.samples_per_pixel as f64);
                    }
                }

                (tile, pixels)
            })
            .collect();

        self.finalize_tiles(width, height, rendered)
    }

    /// Render with adaptive sampling - more samples in noisy areas
    ///
    /// # Arguments
    /// * `min_samples` - Minimum samples per pixel
    /// * `max_samples` - Maximum samples per pixel
    /// * `noise_threshold` - Variance threshold to add more samples (lower = more aggressive)
    pub fn render_adaptive<S: Hittable + Sync, C: Camera + Sync>(
        &self,
        scene: &S,
        camera: &C,
        width: u32,
        height: u32,
        min_samples: u32,
        max_samples: u32,
        noise_threshold: f64,
    ) -> ImageBuffer {
        let tiles = self.get_tiles(width, height);

        let rendered: Vec<(Tile, Vec<Color>)> = tiles
            .into_par_iter()
            .map(|tile| {
                let mut rng = rand::thread_rng();
                let mut pixels = Vec::with_capacity((tile.width * tile.height) as usize);

                for local_y in 0..tile.height {
                    for local_x in 0..tile.width {
                        let i = tile.x + local_x;
                        let j = tile.y + local_y;

                        // Start with minimum samples
                        let mut sum = Color::BLACK;
                        let mut sum_sq = Color::BLACK;
                        let mut count = 0u32;

                        // Track luminance moments separately for a scalar stop criterion.
                        let mut sum_lum = 0.0_f64;
                        let mut sum_lum_sq = 0.0_f64;

                        let mut take_sample = |sum: &mut Color, sum_sq: &mut Color,
                                               sum_lum: &mut f64, sum_lum_sq: &mut f64,
                                               count: &mut u32, rng: &mut rand::rngs::ThreadRng| {
                            let u = (i as f64 + rng.r#gen::<f64>()) / (width - 1) as f64;
                            let v = ((height - 1 - j) as f64 + rng.r#gen::<f64>()) / (height - 1) as f64;
                            let ray = camera.get_ray(u, v);
                            let sample = self.trace_with_rng(ray, scene, self.max_depth, rng);
                            *sum = *sum + sample;
                            *sum_sq = *sum_sq + sample * sample;
                            let lum = sample.luminance();
                            *sum_lum += lum;
                            *sum_lum_sq += lum * lum;
                            *count += 1;
                        };

                        for _ in 0..min_samples {
                            take_sample(&mut sum, &mut sum_sq, &mut sum_lum, &mut sum_lum_sq, &mut count, &mut rng);
                        }

                        // Continue until standard error of the mean (in luminance)
                        // drops below `noise_threshold`, or we hit `max_samples`.
                        while count < max_samples {
                            let n = count as f64;
                            let mean_lum = sum_lum / n;
                            // Bessel-corrected sample variance.
                            let var_lum = if count > 1 {
                                ((sum_lum_sq - n * mean_lum * mean_lum) / (n - 1.0)).max(0.0)
                            } else {
                                f64::INFINITY
                            };
                            let stderr = (var_lum / n).sqrt();
                            if stderr < noise_threshold {
                                break;
                            }

                            let batch = ((max_samples - count) / 4).max(1).min(16);
                            for _ in 0..batch {
                                take_sample(&mut sum, &mut sum_sq, &mut sum_lum, &mut sum_lum_sq, &mut count, &mut rng);
                            }
                        }

                        pixels.push(sum / count as f64);
                    }
                }

                (tile, pixels)
            })
            .collect();

        self.finalize_tiles(width, height, rendered)
    }

    /// Trace with Next Event Estimation (NEE) and balance-heuristic MIS.
    ///
    /// At each non-specular vertex we form two estimators of direct lighting:
    /// a *light sample* (an explicit shadow ray, the classical NEE shadow ray)
    /// and a *BSDF sample* (the same ray we use for the indirect bounce).
    /// Each is weighted by the balance heuristic. Emission found by the BSDF
    /// sample is counted with its MIS weight here, and the recursive call is
    /// told not to count that vertex's emission again (`count_emission=false`),
    /// avoiding the classic NEE+BSDF double-count.
    fn trace_nee<S: Hittable>(
        &self,
        ray: Ray,
        scene: &S,
        lights: &LightList,
        depth: u32,
    ) -> Color {
        self.trace_nee_inner(ray, scene, lights, depth, true)
    }

    fn trace_nee_inner<S: Hittable>(
        &self,
        ray: Ray,
        scene: &S,
        lights: &LightList,
        depth: u32,
        count_emission: bool,
    ) -> Color {
        if depth == 0 {
            return Color::BLACK;
        }
        let t_min = 0.001;

        let hit = match scene.hit(&ray, Interval::new(t_min, f64::INFINITY)) {
            Some(h) => h,
            None => return if count_emission { self.background_color(&ray) } else { Color::BLACK },
        };

        let emitted = if count_emission {
            hit.material.emitted(hit.uv.0, hit.uv.1, hit.point)
        } else {
            Color::BLACK
        };

        let scatter = match hit.material.scatter_pdf(&ray, &hit) {
            Some(s) => s,
            None => return emitted,
        };

        // ---------- Specular: pure pass-through, next vertex counts emission ----------
        if scatter.is_specular {
            let specular_ray = match scatter.specular_ray {
                Some(r) => r,
                None => return emitted,
            };
            let mut atten = scatter.attenuation;
            let bounces = self.max_depth - depth;
            if bounces >= 3 {
                let q = atten.luminance().clamp(0.05, 0.95);
                if rand::thread_rng().r#gen::<f64>() > q {
                    return emitted;
                }
                atten = atten / q;
            }
            return emitted
                + atten * self.trace_nee_inner(specular_ray, scene, lights, depth - 1, true);
        }

        let bsdf_pdf = match &scatter.pdf {
            Some(p) => p,
            None => return emitted,
        };

        // ---------- Direct: light-sampled estimator (NEE shadow ray) ----------
        let mut direct = Color::BLACK;
        if !lights.is_empty() {
            // Sample the light list directly — avoids constructing a HittablePdf
            // (which previously cloned an Arc<dyn Hittable> per non-specular
            // bounce; profile showed ~14% wall time in Arc clone+drop).
            let wi_l = lights.random_direction(hit.point);
            let p_l = lights.pdf_value(hit.point, wi_l);
            if p_l > 1e-8 {
                let shadow = Ray::new(hit.point, wi_l);
                if let Some(lh) = scene.hit(&shadow, Interval::new(t_min, f64::INFINITY)) {
                    let le = lh.material.emitted(lh.uv.0, lh.uv.1, lh.point);
                    if le.luminance() > 0.0 {
                        let p_b = bsdf_pdf.value(wi_l);
                        let scat_pdf = hit.material.scattering_pdf(&ray, &hit, &shadow);
                        if scat_pdf > 0.0 {
                            let w = mis_balance(p_l, p_b);
                            direct = direct + scatter.attenuation * le * (scat_pdf / p_l) * w;
                        }
                    }
                }
            }
        }

        // ---------- BSDF-sampled estimator (indirect + MIS-weighted emission) ----------
        let bounces = self.max_depth - depth;
        let mut atten = scatter.attenuation;
        if bounces >= 3 {
            let q = atten.luminance().clamp(0.05, 0.95);
            if rand::thread_rng().r#gen::<f64>() > q {
                return emitted + direct;
            }
            atten = atten / q;
        }

        let wi_b = bsdf_pdf.generate();
        let p_b = bsdf_pdf.value(wi_b);
        if p_b <= 1e-8 {
            return emitted + direct;
        }
        let bsdf_ray = Ray::new(hit.point, wi_b);
        let scat_pdf = hit.material.scattering_pdf(&ray, &hit, &bsdf_ray);
        if scat_pdf <= 1e-8 {
            return emitted + direct;
        }
        let throughput = atten * (scat_pdf / p_b);

        let indirect = match scene.hit(&bsdf_ray, Interval::new(t_min, f64::INFINITY)) {
            Some(nh) => {
                let le = nh.material.emitted(nh.uv.0, nh.uv.1, nh.point);
                let bsdf_emission = if le.luminance() > 0.0 {
                    let p_l = if lights.is_empty() { 0.0 } else { lights.pdf_value(hit.point, wi_b) };
                    let w = if p_l > 0.0 { mis_balance(p_b, p_l) } else { 1.0 };
                    throughput * le * w
                } else {
                    Color::BLACK
                };
                // Recurse with emission suppressed at the next vertex — already
                // accounted for in `bsdf_emission` (or, if no emission, the term
                // is just zero so the suppression is a no-op).
                let downstream =
                    throughput * self.trace_nee_inner(bsdf_ray, scene, lights, depth - 1, false);
                bsdf_emission + downstream
            }
            None => throughput * self.background_color(&bsdf_ray),
        };

        emitted + direct + indirect
    }

    /// Recursively trace a ray through the scene with Russian Roulette termination
    /// Trace with external RNG (faster - avoids thread_rng() overhead)
    fn trace_with_rng<S: Hittable, R: Rng>(
        &self,
        ray: Ray,
        scene: &S,
        depth: u32,
        rng: &mut R,
    ) -> Color {
        if depth == 0 {
            return Color::BLACK;
        }

        let t_min = 0.001;

        if let Some(hit) = scene.hit(&ray, Interval::new(t_min, f64::INFINITY)) {
            let emitted = hit.material.emitted(hit.uv.0, hit.uv.1, hit.point);

            if let Some((attenuation, scattered)) =
                hit.material
                    .scatter(&ray, hit.point, hit.normal, hit.front_face)
            {
                // Russian Roulette after 3 bounces
                let bounces_so_far = self.max_depth - depth;
                if bounces_so_far >= 3 {
                    let q = attenuation.luminance().clamp(0.05, 0.95);
                    if rng.r#gen::<f64>() > q {
                        return emitted;
                    }
                    let compensated = attenuation / q;
                    return emitted + compensated * self.trace_with_rng(scattered, scene, depth - 1, rng);
                }

                return emitted + attenuation * self.trace_with_rng(scattered, scene, depth - 1, rng);
            }

            return emitted;
        }

        self.background_color(&ray)
    }

    /// Legacy trace (uses thread_rng internally)
    #[allow(dead_code)]
    fn trace<S: Hittable>(&self, ray: Ray, scene: &S, depth: u32) -> Color {
        self.trace_with_rng(ray, scene, depth, &mut rand::thread_rng())
    }

    /// Get background color for rays that don't hit anything
    fn background_color(&self, ray: &Ray) -> Color {
        match self.background {
            Background::Sky => {
                let unit_dir = ray.direction.normalize();
                let t = 0.5 * (unit_dir.y + 1.0);
                Color::WHITE.lerp(Color::new(0.5, 0.7, 1.0), t)
            }
            Background::Black => Color::BLACK,
            Background::Solid(color) => color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::PerspectiveCamera;
    use crate::material::{DiffuseLight, Lambertian};
    use crate::scene::{LightList, World};
    use crate::shape::Sphere;
    use crate::core::Point3;
    use std::sync::Arc;

    #[test]
    fn nee_smoke_emits_light_through_diffuse_bounce() {
        // One emissive sphere overhead, one diffuse "floor" sphere below the
        // camera. NEE should produce non-black, finite pixels without panic.
        let light_mat = DiffuseLight::new(Color::new(15.0, 15.0, 15.0));
        let light_sphere = Sphere::new(Point3::new(0.0, 3.0, 0.0), 0.5, light_mat);
        let floor_mat = Lambertian::new(Color::new(0.7, 0.7, 0.7));
        let floor = Sphere::new(Point3::new(0.0, -100.5, 0.0), 100.0, floor_mat);

        let scene = World::new().add(light_sphere.clone()).add(floor);
        let lights = LightList::new().add_arc(Arc::new(light_sphere));

        let camera = PerspectiveCamera::new(
            Point3::new(0.0, 0.5, 3.0),
            Point3::new(0.0, 0.0, 0.0),
            crate::core::Vec3::new(0.0, 1.0, 0.0),
            45.0,
            1.0,
        );

        let renderer = FlatRenderer::new(8, 4)
            .with_background(Background::Black)
            .with_tonemap(ToneMap::None);
        let img = renderer.render_with_lights(&scene, &lights, &camera, 16, 16);

        let mut any_nonblack = false;
        for y in 0..img.height() as usize {
            for x in 0..img.width() as usize {
                let [r, g, b] = img.get_pixel(x, y);
                if r > 0 || g > 0 || b > 0 { any_nonblack = true; }
            }
        }
        assert!(any_nonblack, "NEE produced an entirely black image");
    }
}

impl Default for FlatRenderer {
    fn default() -> Self {
        Self {
            max_depth: 50,
            samples_per_pixel: 100,
            background: Background::Sky,
            tile_size: 32,
            sampling: SamplingStrategy::Random,
            use_morton_order: true,
            exposure: 0.0,
            tonemap: ToneMap::Aces,
        }
    }
}
