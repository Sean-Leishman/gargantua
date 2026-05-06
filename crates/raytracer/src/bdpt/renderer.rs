//! BDPT Renderer with tile-based parallel rendering

use super::{connect_paths, generate_camera_path, generate_light_path};
use crate::camera::Camera;
use crate::core::{Color, Hittable};
use crate::flat::Background;
use crate::output::{HdrBuffer, ImageBuffer};
use crate::scene::LightList;
use rand::Rng;
use rayon::prelude::*;

/// A rectangular tile for parallel rendering
#[derive(Clone, Copy, Debug)]
struct Tile {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Tile {
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
}

/// Bidirectional Path Tracing renderer
pub struct BdptRenderer {
    /// Maximum path depth
    pub max_depth: u32,
    /// Samples per pixel
    pub samples_per_pixel: u32,
    /// Background type
    pub background: Background,
    /// Tile size for parallel rendering
    tile_size: u32,
    /// Maximum luminance for firefly clamping
    max_luminance: f64,
}

impl BdptRenderer {
    /// Create a new BDPT renderer
    pub fn new(max_depth: u32, samples_per_pixel: u32) -> Self {
        Self {
            max_depth,
            samples_per_pixel,
            background: Background::Black,
            tile_size: 32,
            max_luminance: 100.0,
        }
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

    /// Set the maximum luminance for firefly clamping
    pub fn with_max_luminance(mut self, max_luminance: f64) -> Self {
        self.max_luminance = max_luminance;
        self
    }

    /// Render the scene using BDPT
    pub fn render<S, C>(
        &self,
        scene: &S,
        lights: &LightList,
        camera: &C,
        width: u32,
        height: u32,
    ) -> ImageBuffer
    where
        S: Hittable + Sync,
        C: Camera + Sync,
    {
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
                            let v = ((height - 1 - j) as f64 + rng.r#gen::<f64>()) / (height - 1) as f64;

                            color += self.sample_pixel(scene, lights, camera, u, v, &mut rng);
                        }
                        pixels.push(color / self.samples_per_pixel as f64);
                    }
                }

                (tile, pixels)
            })
            .collect();

        Self::assemble_tiles(width, height, rendered)
    }

    /// Render to HDR buffer for post-processing
    pub fn render_hdr<S, C>(
        &self,
        scene: &S,
        lights: &LightList,
        camera: &C,
        width: u32,
        height: u32,
    ) -> HdrBuffer
    where
        S: Hittable + Sync,
        C: Camera + Sync,
    {
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
                            let v = ((height - 1 - j) as f64 + rng.r#gen::<f64>()) / (height - 1) as f64;

                            color += self.sample_pixel(scene, lights, camera, u, v, &mut rng);
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

    /// Sample a single pixel using BDPT
    fn sample_pixel<S, C, R>(
        &self,
        scene: &S,
        lights: &LightList,
        camera: &C,
        u: f64,
        v: f64,
        rng: &mut R,
    ) -> Color
    where
        S: Hittable,
        C: Camera,
        R: Rng,
    {
        // Generate camera subpath
        let camera_path = generate_camera_path(
            scene,
            camera,
            u,
            v,
            self.max_depth as usize,
            rng,
        );

        // Generate light subpath
        let light_path = generate_light_path(
            scene,
            lights,
            self.max_depth as usize,
            rng,
        );

        // Connect paths and compute contribution
        let mut contribution = connect_paths(scene, &light_path, &camera_path, lights);

        // Clamp fireflies
        let lum = contribution.luminance();
        if lum > self.max_luminance {
            contribution = contribution * (self.max_luminance / lum);
        }

        // Ensure no negative values
        contribution.r = contribution.r.max(0.0);
        contribution.g = contribution.g.max(0.0);
        contribution.b = contribution.b.max(0.0);

        contribution
    }

    /// Assemble tiles into final image
    fn assemble_tiles(width: u32, height: u32, tiles: Vec<(Tile, Vec<Color>)>) -> ImageBuffer {
        let rows = Self::assemble_rows(width, height, tiles);
        ImageBuffer::from_rows(width, height, rows)
    }

    /// Assemble tiles into rows
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
}

impl Default for BdptRenderer {
    fn default() -> Self {
        Self {
            max_depth: 10,
            samples_per_pixel: 100,
            background: Background::Black,
            tile_size: 32,
            max_luminance: 100.0,
        }
    }
}
