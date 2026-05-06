//! Small Cornell-box render for CPU profiling.
//!
//! Same scene as `cornell_box`, but at a size that finishes in seconds so a
//! sampling profiler can record many short runs. Tunable via env vars:
//!   PROFILE_W, PROFILE_H, PROFILE_SPP, PROFILE_MAX_DEPTH

use raytracer::prelude::*;
use raytracer::scene::LightList;

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

fn main() {
    let red = Lambertian::new(Color::new(0.65, 0.05, 0.05));
    let white = Lambertian::new(Color::new(0.73, 0.73, 0.73));
    let green = Lambertian::new(Color::new(0.12, 0.45, 0.15));
    let light_mat = DiffuseLight::new(Color::new(15.0, 15.0, 15.0));

    let room_size = 555.0;
    let mut scene = World::new();

    scene = scene.add(Quad::new(point3(room_size, 0.0, 0.0), vec3(0.0, room_size, 0.0), vec3(0.0, 0.0, room_size), green));
    scene = scene.add(Quad::new(point3(0.0, 0.0, 0.0), vec3(0.0, room_size, 0.0), vec3(0.0, 0.0, room_size), red));
    scene = scene.add(Quad::new(point3(0.0, 0.0, 0.0), vec3(room_size, 0.0, 0.0), vec3(0.0, 0.0, room_size), white.clone()));
    scene = scene.add(Quad::new(point3(0.0, room_size, 0.0), vec3(room_size, 0.0, 0.0), vec3(0.0, 0.0, room_size), white.clone()));
    scene = scene.add(Quad::new(point3(0.0, 0.0, room_size), vec3(room_size, 0.0, 0.0), vec3(0.0, room_size, 0.0), white.clone()));

    let light_size = 130.0;
    let light_offset = (room_size - light_size) / 2.0;
    let ceiling_light = Quad::new(
        point3(light_offset, room_size - 1.0, light_offset),
        vec3(light_size, 0.0, 0.0),
        vec3(0.0, 0.0, light_size),
        light_mat,
    );
    scene = scene.add(ceiling_light.clone());

    scene = scene.add(BoxShape::new(point3(130.0, 0.0, 65.0), point3(295.0, 165.0, 230.0), white.clone()));
    scene = scene.add(BoxShape::new(point3(265.0, 0.0, 295.0), point3(430.0, 330.0, 460.0), white.clone()));

    let bvh = scene.build_bvh();
    let lights = LightList::new().add(ceiling_light);

    let camera = PerspectiveCamera::new(
        point3(278.0, 278.0, -800.0),
        point3(278.0, 278.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        40.0,
        1.0,
    );

    let width = env_u32("PROFILE_W", 256);
    let height = env_u32("PROFILE_H", 256);
    let spp = env_u32("PROFILE_SPP", 32);
    let max_depth = env_u32("PROFILE_MAX_DEPTH", 12);

    eprintln!("profile_nee: {}x{} spp={} max_depth={}", width, height, spp, max_depth);
    let t0 = std::time::Instant::now();
    let image = FlatRenderer::new(max_depth, spp)
        .with_background(Background::Black)
        .render_with_lights(&bvh, &lights, &camera, width, height);
    let dt = t0.elapsed();
    eprintln!("rendered in {:.2?} ({:.2} Mrays/s approx, ignoring secondary)",
              dt, (width * height * spp) as f64 / dt.as_secs_f64() / 1e6);

    image.save_ppm("/tmp/profile_nee.ppm").unwrap();
}
