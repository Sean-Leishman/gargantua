use clap::Parser;
use gr_core::{Kerr, Metric, Schwarzschild};
use image::RgbImage;
use nalgebra::{Vector3, Vector4};
use raytracer::accel::BvhNode;
use raytracer::core::{Color, point3};
use raytracer::curved::{
    AccretionDisk, Camera, RenderOptions, render, render_with_disk, render_with_disk_and_scene,
    render_with_scene,
};
use raytracer::material::{DiffuseLight, Lambertian};
use raytracer::scene::World;
use raytracer::shape::Sphere;
use std::f64::consts::PI;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "gr-renderer", about = "General-relativity ray renderer")]
struct Args {
    #[arg(long, default_value = "schwarzschild")]
    metric: String,

    #[arg(long, default_value_t = 1.0)]
    mass: f64,

    #[arg(long, default_value_t = 0.9)]
    spin: f64,

    #[arg(long, default_value_t = 800)]
    width: u32,

    #[arg(long, default_value_t = 600)]
    height: u32,

    #[arg(long)]
    disk: bool,

    /// Scene preset to render geodesics against. `none` (default) renders
    /// only sky / disk / horizon. `lensed-spheres` places four colored
    /// Lambertian spheres + a luminous backdrop behind the hole — the
    /// canonical lensing demo (see `examples/curved_scene.rs`).
    #[arg(long, default_value = "none")]
    scene: String,

    /// Disk inner radius (geometric units). Default: 6M (Schwarzschild ISCO).
    #[arg(long, default_value_t = 6.0)]
    r_inner: f64,

    /// Disk outer radius.
    #[arg(long, default_value_t = 20.0)]
    r_outer: f64,

    /// Disk Gaussian vertical scale height.
    #[arg(long, default_value_t = 1.0)]
    scale_height: f64,

    /// Per-axis supersampling (1 = no AA, 2 = 4 spp, 3 = 9 spp).
    #[arg(long, default_value_t = 1)]
    samples: u32,

    /// Suppress the progress bar.
    #[arg(long)]
    quiet: bool,

    #[arg(long, default_value = "out.png")]
    output: String,
}

fn main() {
    let args = Args::parse();

    let camera = Camera {
        position: Vector4::new(0.0, 30.0, PI / 2.0, 0.0),
        look_at: Vector3::new(-1.0, 0.0, 0.0),
        up: Vector3::new(0.0, 0.0, 1.0),
        fov_y_radians: 60.0_f64.to_radians(),
        aspect: args.width as f64 / args.height as f64,
    };

    let t0 = Instant::now();

    let opts = RenderOptions {
        samples_per_axis: args.samples.max(1),
        show_progress: !args.quiet,
    };
    let disk = AccretionDisk {
        r_inner: args.r_inner,
        r_outer: args.r_outer,
        scale_height: args.scale_height,
        ..AccretionDisk::default()
    };

    let scene = match args.scene.as_str() {
        "none" => None,
        "lensed-spheres" => Some(build_lensed_spheres()),
        other => {
            eprintln!("Unknown scene '{}'. Supported: none, lensed-spheres", other);
            std::process::exit(1);
        }
    };

    let img: RgbImage = match args.metric.as_str() {
        "schwarzschild" => {
            let m = Schwarzschild::new(args.mass);
            render_dispatch(&m, &camera, args.disk, &disk, scene.as_ref(), args.width, args.height, opts)
        }
        "kerr" => {
            let m = Kerr::new(args.mass, args.spin);
            render_dispatch(&m, &camera, args.disk, &disk, scene.as_ref(), args.width, args.height, opts)
        }
        other => {
            eprintln!("Unknown metric '{}'. Supported: schwarzschild, kerr", other);
            std::process::exit(1);
        }
    };

    img.save(&args.output).expect("failed to save image");
    let mut tags = Vec::new();
    if args.disk { tags.push("disk"); }
    if scene.is_some() { tags.push(args.scene.as_str()); }
    let suffix = if tags.is_empty() { String::new() } else { format!("+{}", tags.join("+")) };
    println!(
        "Rendered {}x{} ({}{}) in {:.2}s → {}",
        args.width,
        args.height,
        args.metric,
        suffix,
        t0.elapsed().as_secs_f64(),
        args.output
    );
}

fn render_dispatch<M: Metric + Sync>(
    metric: &M,
    camera: &Camera,
    use_disk: bool,
    disk: &AccretionDisk,
    scene: Option<&BvhNode>,
    width: u32,
    height: u32,
    opts: RenderOptions,
) -> RgbImage {
    match (use_disk, scene) {
        (true, Some(s)) => render_with_disk_and_scene(metric, camera, disk, s, width, height, opts),
        (true, None) => render_with_disk(metric, camera, disk, width, height, opts),
        (false, Some(s)) => render_with_scene(metric, camera, s, width, height, opts),
        (false, None) => render(metric, camera, width, height, opts),
    }
}

/// Same scene as `examples/curved_scene.rs`: a luminous backdrop sphere
/// directly behind the BH plus four colored Lambertian spheres in a
/// cross pattern, all at r ≈ 40–60 along the camera's line of sight.
fn build_lensed_spheres() -> BvhNode {
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
    world.build_bvh()
}
