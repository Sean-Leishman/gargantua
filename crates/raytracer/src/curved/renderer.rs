use crate::core::Hittable;
use crate::curved::camera::Camera;
use crate::curved::disk::AccretionDisk;
use crate::curved::outcome::RayOutcome;
use crate::curved::ray::GeodesicRay;
use crate::curved::tracer::{
    trace_ray, trace_ray_with_disk, trace_ray_with_disk_and_scene, trace_ray_with_scene,
};
use gr_core::{Metric, RK45Integrator};
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
        RayOutcome::Scene { color } => *color,
        // Photon still in flight at step cap — treat as deep sky (no direction available).
        RayOutcome::MaxSteps => [0.0, 0.0, 10.0 / 255.0],
    }
}

pub fn sky_color(dir: &Vector3<f64>) -> [f64; 3] {
    let hash = |x: f64| -> u64 {
        let bits = x.to_bits();
        bits.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
    };
    let h = hash(dir[0]) ^ hash(dir[1]).rotate_left(17) ^ hash(dir[2]).rotate_left(34);
    if h % 512 < 3 {
        let b = (180 + (h % 75) as u32) as f64 / 255.0;
        [b, b, b]
    } else {
        // Camera "up" is +z, so use the z-component for the horizon→zenith gradient.
        let v = dir[2].abs() * 15.0 / 255.0;
        [0.0, 0.0, 10.0 / 255.0 + v]
    }
}

pub fn disk_color(intensity: f64) -> [f64; 3] {
    // Reinhard tone-map then warm-body ramp (red → orange → white). Linear 0..1.
    let x = intensity / (1.0 + intensity);
    [x.powf(0.5), x.powf(1.2), x.powf(2.5)]
}

/// Linear 0..1 → sRGB-encoded 8-bit.
pub fn encode_srgb(linear: [f64; 3]) -> Rgb<u8> {
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

/// Render a curved-space scene where geodesics test against arbitrary
/// `Hittable` objects (a Cornell box, glass spheres, anything from the
/// flat-space `shape` module). The path through the metric is integrated
/// with RK45; between steps the chord is tested against the scene's BVH.
///
/// To combine the scene with a volumetric accretion disk, use
/// `render_with_disk_and_scene` instead.
pub fn render_with_scene<M: Metric + Sync, S: Hittable + Sync>(
    metric: &M,
    camera: &Camera,
    scene: &S,
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
    let render_one_row = |py: u32| -> Vec<Rgb<u8>> {
        (0..width)
            .map(|px| {
                let mut acc = [0.0_f64; 3];
                for sy in 0..spa {
                    for sx in 0..spa {
                        let sub_px = px * spa + sx;
                        let sub_py = py * spa + sy;
                        let mut ray =
                            GeodesicRay::from_camera(metric, camera, sub_px, sub_py, sub_w, sub_h);
                        let outcome = trace_ray_with_scene(
                            metric, &mut ray, scene, &observer, &integrator, max_steps,
                        );
                        let lin = shade_outcome_linear(&outcome);
                        acc[0] += lin[0];
                        acc[1] += lin[1];
                        acc[2] += lin[2];
                    }
                }
                encode_srgb([acc[0] * inv_spp, acc[1] * inv_spp, acc[2] * inv_spp])
            })
            .collect()
    };

    let rows: Vec<Vec<Rgb<u8>>> = if let Some(pb) = progress.as_ref() {
        row_iter.progress_with(pb.clone()).map(render_one_row).collect()
    } else {
        row_iter.map(render_one_row).collect()
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

/// Render a curved-space scene composed of *both* a volumetric accretion
/// disk and `Hittable` geometry. See `trace_ray_with_disk_and_scene` for
/// the per-step composition.
pub fn render_with_disk_and_scene<M: Metric + Sync, S: Hittable + Sync>(
    metric: &M,
    camera: &Camera,
    disk: &AccretionDisk,
    scene: &S,
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
    let render_one_row = |py: u32| -> Vec<Rgb<u8>> {
        (0..width)
            .map(|px| {
                let mut acc = [0.0_f64; 3];
                for sy in 0..spa {
                    for sx in 0..spa {
                        let sub_px = px * spa + sx;
                        let sub_py = py * spa + sy;
                        let mut ray =
                            GeodesicRay::from_camera(metric, camera, sub_px, sub_py, sub_w, sub_h);
                        let outcome = trace_ray_with_disk_and_scene(
                            metric, &mut ray, disk, scene, &observer, &integrator, max_steps,
                        );
                        let lin = shade_outcome_linear(&outcome);
                        acc[0] += lin[0];
                        acc[1] += lin[1];
                        acc[2] += lin[2];
                    }
                }
                encode_srgb([acc[0] * inv_spp, acc[1] * inv_spp, acc[2] * inv_spp])
            })
            .collect()
    };

    let rows: Vec<Vec<Rgb<u8>>> = if let Some(pb) = progress.as_ref() {
        row_iter.progress_with(pb.clone()).map(render_one_row).collect()
    } else {
        row_iter.map(render_one_row).collect()
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
                    let mut ray = GeodesicRay::from_camera(metric, camera, sub_px, sub_py, sub_w, sub_h);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Color;
    use crate::material::Lambertian;
    use crate::scene::World;
    use crate::shape::Sphere;
    use gr_core::{Schwarzschild, SpacetimePoint};

    #[test]
    fn render_with_scene_produces_some_object_pixels() {
        // A bright Lambertian sphere in front of the camera, well outside
        // the horizon. With a large radius it should occupy most of the
        // frame and most rays should land on it (RayOutcome::Scene), not
        // escape (sky) or fall into the horizon.
        let mat = Lambertian::new(Color::new(0.9, 0.1, 0.1));
        let sphere = Sphere::new(crate::core::point3(20.0, 0.0, 0.0), 8.0, mat);
        let world = World::new().add(sphere);
        let bvh = world.build_bvh();

        let metric = Schwarzschild::new(1.0);
        // Camera at r = 50, on the equatorial plane (theta = pi/2), phi = pi
        // so it looks toward the sphere placed at (+x, 0, 0) in cartesian.
        let camera = Camera {
            position: SpacetimePoint::new(0.0, 50.0, std::f64::consts::FRAC_PI_2, std::f64::consts::PI),
            look_at: Vector3::new(1.0, 0.0, 0.0),
            up: Vector3::new(0.0, 0.0, 1.0),
            fov_y_radians: 0.6,
            aspect: 1.0,
        };

        let img = render_with_scene(&metric, &camera, &bvh, 16, 16, RenderOptions::default());

        let mut red_pixels = 0;
        for px in img.pixels() {
            // The Lambertian albedo is (0.9, 0.1, 0.1) → after facing-ratio +
            // sRGB encode, red dominates green/blue significantly.
            let r = px.0[0] as i32;
            let g = px.0[1] as i32;
            if r > 50 && r > g + 20 {
                red_pixels += 1;
            }
        }
        assert!(red_pixels > 5, "expected curved renderer to hit the sphere on at least a few pixels (got {})", red_pixels);
    }
}

