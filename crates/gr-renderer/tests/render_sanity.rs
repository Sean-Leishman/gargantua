use gr_core::Schwarzschild;
use gr_renderer::{RenderOptions, render};
use gr_tracer::Camera;
use nalgebra::{Vector3, Vector4};
use std::f64::consts::PI;

fn default_camera(width: u32, height: u32) -> Camera {
    Camera {
        position: Vector4::new(0.0, 30.0, PI / 2.0, 0.0),
        look_at: Vector3::new(-1.0, 0.0, 0.0),
        up: Vector3::new(0.0, 0.0, 1.0),
        fov_y_radians: 60.0_f64.to_radians(),
        aspect: width as f64 / height as f64,
    }
}

#[test]
fn horizon_shadow_and_escape() {
    let metric = Schwarzschild::new(1.0);
    let camera = default_camera(32, 24);
    let img = render(&metric, &camera, 32, 24, RenderOptions::default());

    let has_black = img.pixels().any(|p| p.0 == [0, 0, 0]);
    let has_non_black = img.pixels().any(|p| p.0 != [0, 0, 0]);

    assert!(has_black, "expected at least one black pixel (horizon shadow)");
    assert!(has_non_black, "expected at least one non-black pixel (escaped rays)");
}
