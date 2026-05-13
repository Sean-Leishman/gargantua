//! Schwarzschild-lensed scene of `Hittable` spheres.
//!
//! The camera sits on the +x axis at r=30, looking toward the black hole
//! at the origin (rs=2 with M=1). Spheres are placed *behind* the hole
//! along -x so that geodesics bending around the horizon visibly distort
//! their projected positions. Off-axis spheres at ±y and ±z give the
//! Einstein-ring-ish silhouette some structure.
//!
//! Run with:
//!   cargo run --release --features curved --example curved_scene -p raytracer

use gr_core::{Schwarzschild, SpacetimePoint};
use nalgebra::Vector3;
use raytracer::core::{Color, point3};
use raytracer::curved::{Camera, RenderOptions, render_with_scene};
use raytracer::material::{DiffuseLight, Lambertian};
use raytracer::scene::World;
use raytracer::shape::Sphere;
use std::f64::consts::FRAC_PI_2;
use std::time::Instant;

fn main() {
    let metric = Schwarzschild::new(1.0); // rs = 2

    // Backdrop: a large luminous sphere far behind the BH, plus four
    // colored Lambertian spheres in a cross pattern at roughly the same
    // distance. The luminous backdrop becomes the "sky disk" warped by
    // the photon sphere; the colored spheres show how off-axis pixels
    // bend toward the hole.
    let backdrop = Sphere::new(
        point3(-60.0, 0.0, 0.0),
        4.0,
        DiffuseLight::new(Color::new(2.0, 1.6, 0.8)),
    );
    let red = Sphere::new(
        point3(-40.0, 14.0, 0.0),
        3.5,
        Lambertian::new(Color::new(0.85, 0.10, 0.10)),
    );
    let green = Sphere::new(
        point3(-40.0, -14.0, 0.0),
        3.5,
        Lambertian::new(Color::new(0.10, 0.75, 0.20)),
    );
    let blue = Sphere::new(
        point3(-40.0, 0.0, 14.0),
        3.5,
        Lambertian::new(Color::new(0.15, 0.30, 0.90)),
    );
    let yellow = Sphere::new(
        point3(-40.0, 0.0, -14.0),
        3.5,
        Lambertian::new(Color::new(0.95, 0.85, 0.20)),
    );

    let world = World::new()
        .add(backdrop)
        .add(red)
        .add(green)
        .add(blue)
        .add(yellow);
    let bvh = world.build_bvh();

    let width = 480u32;
    let height = 360u32;
    let camera = Camera {
        // Equatorial plane (θ=π/2), φ=0 puts the camera on the +x axis at r=30.
        position: SpacetimePoint::new(0.0, 30.0, FRAC_PI_2, 0.0),
        look_at: Vector3::new(-1.0, 0.0, 0.0),
        up: Vector3::new(0.0, 0.0, 1.0),
        fov_y_radians: 60.0_f64.to_radians(),
        aspect: width as f64 / height as f64,
    };

    let opts = RenderOptions {
        samples_per_axis: 2, // 4 spp — enough to soften the horizon edge.
        show_progress: true,
        ..Default::default()
    };

    eprintln!("curved_scene: {}x{} spp={}", width, height, opts.samples_per_axis * opts.samples_per_axis);
    let t0 = Instant::now();
    let img = render_with_scene(&metric, &camera, &bvh, width, height, opts);
    eprintln!("rendered in {:.2?}", t0.elapsed());

    let out = "curved_scene.png";
    img.save(out).expect("save png");
    eprintln!("wrote {}", out);
}
