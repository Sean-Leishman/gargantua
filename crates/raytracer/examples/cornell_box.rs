//! Cornell Box - Classic ray tracing test scene
//!
//! The Cornell Box is a standard test scene for physically based renderers.
//! It consists of a box-shaped room with colored walls and two boxes inside,
//! illuminated by a rectangular light on the ceiling.
//!
//! This example demonstrates:
//! - BVH acceleration structure for O(log n) intersection
//! - Tile-based parallel rendering
//! - Russian Roulette path termination
//! - Next Event Estimation (NEE) with MIS for reduced noise

use raytracer::prelude::*;
use raytracer::scene::LightList;

fn main() {
    // Materials
    let red = Lambertian::new(Color::new(0.65, 0.05, 0.05));
    let white = Lambertian::new(Color::new(0.73, 0.73, 0.73));
    let green = Lambertian::new(Color::new(0.12, 0.45, 0.15));
    let light_mat = DiffuseLight::new(Color::new(15.0, 15.0, 15.0));

    // Room dimensions (classic Cornell Box is 555x555x555)
    let room_size = 555.0;

    // Build scene with walls
    let mut scene = World::new();

    // Left wall (green)
    scene = scene.add(Quad::new(
        point3(room_size, 0.0, 0.0),
        vec3(0.0, room_size, 0.0),
        vec3(0.0, 0.0, room_size),
        green,
    ));

    // Right wall (red)
    scene = scene.add(Quad::new(
        point3(0.0, 0.0, 0.0),
        vec3(0.0, room_size, 0.0),
        vec3(0.0, 0.0, room_size),
        red,
    ));

    // Floor (white)
    scene = scene.add(Quad::new(
        point3(0.0, 0.0, 0.0),
        vec3(room_size, 0.0, 0.0),
        vec3(0.0, 0.0, room_size),
        white.clone(),
    ));

    // Ceiling (white)
    scene = scene.add(Quad::new(
        point3(0.0, room_size, 0.0),
        vec3(room_size, 0.0, 0.0),
        vec3(0.0, 0.0, room_size),
        white.clone(),
    ));

    // Back wall (white)
    scene = scene.add(Quad::new(
        point3(0.0, 0.0, room_size),
        vec3(room_size, 0.0, 0.0),
        vec3(0.0, room_size, 0.0),
        white.clone(),
    ));

    // Light on ceiling (slightly smaller than ceiling, centered)
    let light_size = 130.0;
    let light_offset = (room_size - light_size) / 2.0;
    let ceiling_light = Quad::new(
        point3(light_offset, room_size - 1.0, light_offset),
        vec3(light_size, 0.0, 0.0),
        vec3(0.0, 0.0, light_size),
        light_mat,
    );
    scene = scene.add(ceiling_light.clone());

    // Tall box (left side)
    scene = scene.add(BoxShape::new(
        point3(130.0, 0.0, 65.0),
        point3(295.0, 165.0, 230.0),
        white.clone(),
    ));

    // Short box (right side)
    scene = scene.add(BoxShape::new(
        point3(265.0, 0.0, 295.0),
        point3(430.0, 330.0, 460.0),
        white.clone(),
    ));

    // Build BVH for accelerated rendering
    println!("Building BVH...");
    let bvh = scene.build_bvh();

    // Create light list for NEE
    let lights = LightList::new().add(ceiling_light);

    // Camera setup - looking into the box from the front
    let camera = PerspectiveCamera::new(
        point3(278.0, 278.0, -800.0), // Camera position (in front of the box)
        point3(278.0, 278.0, 0.0),    // Look at center of back wall
        vec3(0.0, 1.0, 0.0),          // Up vector
        40.0,                         // Field of view
        1.0,                          // Aspect ratio (square image)
    );

    // Render settings
    let width = 600;
    let height = 600;
    let samples_per_pixel = 200;
    let max_depth = 50;

    println!("Cornell Box Renderer");
    println!("====================");
    println!(
        "Rendering {}x{} with {} samples per pixel...",
        width, height, samples_per_pixel
    );
    println!("Using: BVH, tile-based rendering, Russian Roulette, NEE+MIS");

    let start = std::time::Instant::now();

    // Use black background since it's an enclosed scene
    let renderer =
        FlatRenderer::new(max_depth, samples_per_pixel).with_background(Background::Black);

    // Render with NEE for reduced noise
    let image = renderer.render_with_lights(&bvh, &lights, &camera, width, height);

    let elapsed = start.elapsed();
    println!("Rendered in {:.2?}", elapsed);

    // Save
    image.save_ppm("cornell_box.ppm").unwrap();
    println!("Saved to cornell_box.ppm");
    println!("\nConvert to PNG with: convert cornell_box.ppm cornell_box.png");
}
