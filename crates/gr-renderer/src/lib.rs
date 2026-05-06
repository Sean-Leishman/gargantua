use gr_core::{Metric, RK45Integrator};
use gr_tracer::{AccretionDisk, Camera, Ray, RayOutcome, trace_ray, tracer::trace_ray_with_disk};
use image::{Rgb, RgbImage};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use nalgebra::Vector3;
use rayon::prelude::*;

#[derive(Clone, Copy)]
pub struct RenderOptions {
    /// Per-axis supersampling factor (1 = no AA, 2 = 4 spp, 3 = 9 spp, ...).
    pub samples_per_axis: u32,
    /// Show a progress bar over rows.
    pub show_progress: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self { samples_per_axis: 1, show_progress: false }
    }
}

pub fn shade_outcome(outcome: &RayOutcome) -> Rgb<u8> {
    encode_srgb(shade_outcome_linear(outcome))
}

pub fn shade_outcome_linear(outcome: &RayOutcome) -> [f64; 3] {
    match outcome {
        RayOutcome::Horizon => [0.0, 0.0, 0.0],
        RayOutcome::Escaped { final_direction } => sky_color(final_direction),
        RayOutcome::Disk { intensity, .. } => disk_color(*intensity),
        RayOutcome::MaxSteps => [1.0, 0.0, 1.0],
    }
}

fn sky_color(dir: &Vector3<f64>) -> [f64; 3] {
    let hash = |x: f64| -> u64 {
        let bits = x.to_bits();
        bits.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
    };
    let h = hash(dir[0]) ^ hash(dir[1]).rotate_left(17) ^ hash(dir[2]).rotate_left(34);
    if h % 512 < 3 {
        let b = (180 + (h % 75) as u32) as f64 / 255.0;
        [b, b, b]
    } else {
        let v = dir[1].abs() * 15.0 / 255.0;
        [0.0, 0.0, 10.0 / 255.0 + v]
    }
}

fn disk_color(intensity: f64) -> [f64; 3] {
    // Reinhard tone-map then warm-body ramp (red → orange → white). Linear 0..1.
    let x = intensity / (1.0 + intensity);
    [x.powf(0.5), x.powf(1.2), x.powf(2.5)]
}

/// Linear 0..1 → sRGB-encoded 8-bit.
fn encode_srgb(linear: [f64; 3]) -> Rgb<u8> {
    let to_u8 = |c: f64| -> u8 {
        let c = c.clamp(0.0, 1.0);
        let s = if c <= 0.003_130_8 {
            12.92 * c
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u8
    };
    Rgb([to_u8(linear[0]), to_u8(linear[1]), to_u8(linear[2])])
}

pub fn render<M: Metric + Sync>(
    metric: &M,
    camera: &Camera,
    width: u32,
    height: u32,
    opts: RenderOptions,
) -> RgbImage {
    render_inner(metric, camera, None, width, height, opts)
}

pub fn render_with_disk<M: Metric + Sync>(
    metric: &M,
    camera: &Camera,
    disk: &AccretionDisk,
    width: u32,
    height: u32,
    opts: RenderOptions,
) -> RgbImage {
    render_inner(metric, camera, Some(disk), width, height, opts)
}

fn render_inner<M: Metric + Sync>(
    metric: &M,
    camera: &Camera,
    disk: Option<&AccretionDisk>,
    width: u32,
    height: u32,
    opts: RenderOptions,
) -> RgbImage {
    let integrator = RK45Integrator { max_radius: 200.0, ..RK45Integrator::default() };
    let max_steps = 5000;
    let observer = camera.position;
    let spa = opts.samples_per_axis.max(1);
    let sub_w = width * spa;
    let sub_h = height * spa;
    let inv_spp = 1.0 / (spa * spa) as f64;

    let progress = if opts.show_progress {
        let pb = ProgressBar::new(height as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner} rows {pos}/{len} [{elapsed_precise}] [{wide_bar}] eta {eta}",
            )
            .unwrap(),
        );
        Some(pb)
    } else {
        None
    };

    let row_iter = (0..height).into_par_iter();
    let rows: Vec<Vec<Rgb<u8>>> = if let Some(pb) = progress.as_ref() {
        row_iter
            .progress_with(pb.clone())
            .map(|py| render_row(metric, camera, disk, &integrator, &observer, max_steps, py, width, sub_w, sub_h, spa, inv_spp))
            .collect()
    } else {
        row_iter
            .map(|py| render_row(metric, camera, disk, &integrator, &observer, max_steps, py, width, sub_w, sub_h, spa, inv_spp))
            .collect()
    };
    if let Some(pb) = progress { pb.finish_and_clear(); }

    let mut img = RgbImage::new(width, height);
    for (py, row) in rows.into_iter().enumerate() {
        for (px, color) in row.into_iter().enumerate() {
            img.put_pixel(px as u32, py as u32, color);
        }
    }
    img
}

#[allow(clippy::too_many_arguments)]
fn render_row<M: Metric + Sync>(
    metric: &M,
    camera: &Camera,
    disk: Option<&AccretionDisk>,
    integrator: &RK45Integrator,
    observer: &gr_core::SpacetimePoint,
    max_steps: usize,
    py: u32,
    width: u32,
    sub_w: u32,
    sub_h: u32,
    spa: u32,
    inv_spp: f64,
) -> Vec<Rgb<u8>> {
    (0..width)
        .map(|px| {
            let mut acc = [0.0_f64; 3];
            for sy in 0..spa {
                for sx in 0..spa {
                    let sub_px = px * spa + sx;
                    let sub_py = py * spa + sy;
                    let mut ray = Ray::from_camera(metric, camera, sub_px, sub_py, sub_w, sub_h);
                    let outcome = match disk {
                        Some(d) => trace_ray_with_disk(metric, &mut ray, d, observer, integrator, max_steps),
                        None => trace_ray(metric, &mut ray, integrator, max_steps),
                    };
                    let lin = shade_outcome_linear(&outcome);
                    acc[0] += lin[0];
                    acc[1] += lin[1];
                    acc[2] += lin[2];
                }
            }
            encode_srgb([acc[0] * inv_spp, acc[1] * inv_spp, acc[2] * inv_spp])
        })
        .collect()
}
