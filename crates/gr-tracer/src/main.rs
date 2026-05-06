use gr_tracer::Camera;
use nalgebra::{Vector3, Vector4};
use std::f64::consts::PI;

fn main() {
    let _camera = Camera {
        position: Vector4::new(0.0, 30.0, PI / 2.0, 0.0),
        look_at: Vector3::new(-1.0, 0.0, 0.0),
        up: Vector3::new(0.0, 1.0, 0.0),
        fov_y_radians: PI / 3.0,
        aspect: 16.0 / 9.0,
    };
    println!("gr-tracer ready");
}
