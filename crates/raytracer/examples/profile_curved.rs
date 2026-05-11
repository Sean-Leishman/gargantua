//! Small curved-space scene for profiling.
//!
//! Mirrors `curved_scene` but at a size that completes in seconds so a
//! sampling profiler can record many short runs. Tunable via env vars:
//!   PROFILE_W, PROFILE_H, PROFILE_SPA, PROFILE_DISK ("1" to enable)

use gr_core::{Schwarzschild, SpacetimePoint};
use nalgebra::Vector3;
use raytracer::core::{Color, point3};
use raytracer::curved::{
    AccretionDisk, Camera, RenderOptions, render_with_disk_and_scene, render_with_scene,
};
use raytracer::material::{DiffuseLight, Lambertian};
use raytracer::scene::World;
use raytracer::shape::Sphere;
use std::f64::consts::FRAC_PI_2;

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

fn main() {
    let metric = Schwarzschild::new(1.0);

    let world = World::new()
        .add(Sphere::new(
            point3(-60.0, 0.0, 0.0),
            4.0,
            DiffuseLight::new(Color::new(2.0, 1.6, 0.8)),
        ))
        .add(Sphere::new(
            point3(-40.0, 14.0, 0.0),
            3.5,
            Lambertian::new(Color::new(0.85, 0.10, 0.10)),
        ))
        .add(Sphere::new(
            point3(-40.0, -14.0, 0.0),
            3.5,
            Lambertian::new(Color::new(0.10, 0.75, 0.20)),
        ))
        .add(Sphere::new(
            point3(-40.0, 0.0, 14.0),
            3.5,
            Lambertian::new(Color::new(0.15, 0.30, 0.90)),
        ))
        .add(Sphere::new(
            point3(-40.0, 0.0, -14.0),
            3.5,
            Lambertian::new(Color::new(0.95, 0.85, 0.20)),
        ));
    let bvh = world.build_bvh();

    let width = env_u32("PROFILE_W", 96);
    let height = env_u32("PROFILE_H", 72);
    let spa = env_u32("PROFILE_SPA", 1);
    let with_disk = env_u32("PROFILE_DISK", 0) == 1;

    let camera = Camera {
        position: SpacetimePoint::new(0.0, 30.0, FRAC_PI_2, 0.0),
        look_at: Vector3::new(-1.0, 0.0, 0.0),
        up: Vector3::new(0.0, 0.0, 1.0),
        fov_y_radians: 60.0_f64.to_radians(),
        aspect: width as f64 / height as f64,
    };

    let opts = RenderOptions { samples_per_axis: spa, show_progress: false };

    eprintln!(
        "profile_curved: {}x{} spa={} disk={}",
        width, height, spa, with_disk
    );
    let t0 = std::time::Instant::now();
    let img = if with_disk {
        let disk = AccretionDisk::default();
        render_with_disk_and_scene(&metric, &camera, &disk, &bvh, width, height, opts)
    } else {
        render_with_scene(&metric, &camera, &bvh, width, height, opts)
    };
    eprintln!("rendered in {:.2?}", t0.elapsed());
    img.save("/tmp/profile_curved.png").ok();
}
