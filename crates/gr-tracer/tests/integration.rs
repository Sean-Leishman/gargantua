use std::f64::consts::PI;

use gr_core::{RK45Integrator, Schwarzschild, SpacetimePoint};
use gr_tracer::{AccretionDisk, Camera, Ray, RayOutcome, trace_ray, tracer::trace_ray_with_disk};
use nalgebra::{Vector3, Vector4};

fn make_camera(r: f64) -> Camera {
    Camera {
        // Camera at r = 30, θ = π/2, φ = 0 (equatorial plane)
        position: Vector4::new(0.0, r, PI / 2.0, 0.0),
        // Look toward the BH center: negative-r direction in the local frame
        look_at: Vector3::new(-1.0, 0.0, 0.0),
        up: Vector3::new(0.0, 1.0, 0.0),
        fov_y_radians: PI / 3.0, // 60°
        aspect: 1.0,
    }
}

fn make_integrator() -> RK45Integrator {
    RK45Integrator {
        max_radius: 200.0,
        ..Default::default()
    }
}

#[test]
fn test_ray_into_horizon_falls_in() {
    let metric = Schwarzschild::new(1.0); // rs = 2
    let camera = make_camera(30.0);
    let integrator = make_integrator();

    // Center pixel of a 1x1 image → straight ahead toward BH
    let mut ray = Ray::from_camera(&metric, &camera, 0, 0, 1, 1);
    let outcome = trace_ray(&metric, &mut ray, &integrator, 20_000);

    assert!(
        matches!(outcome, RayOutcome::Horizon),
        "expected Horizon, got {outcome:?}"
    );
}

#[test]
fn test_ray_aimed_far_escapes() {
    let metric = Schwarzschild::new(1.0); // rs = 2
    let camera = make_camera(30.0);
    let integrator = make_integrator();

    // Far corner pixel of a 2x2 image (u = 1, v = 1 → strongly off-axis)
    let mut ray = Ray::from_camera(&metric, &camera, 1, 0, 2, 2);
    let outcome = trace_ray(&metric, &mut ray, &integrator, 20_000);

    assert!(
        matches!(outcome, RayOutcome::Escaped { .. }),
        "expected Escaped, got {outcome:?}"
    );
}

#[test]
fn test_disk_density_zero_outside_band() {
    let disk = AccretionDisk::default();
    let inside = SpacetimePoint::new(0.0, 10.0, PI / 2.0, 0.0);
    let too_close = SpacetimePoint::new(0.0, 2.0, PI / 2.0, 0.0);
    let too_far = SpacetimePoint::new(0.0, 50.0, PI / 2.0, 0.0);
    assert!(disk.density(&inside) > 0.0);
    assert_eq!(disk.density(&too_close), 0.0);
    assert_eq!(disk.density(&too_far), 0.0);
}

#[test]
fn test_disk_density_falls_off_vertically() {
    let disk = AccretionDisk::default();
    let mid = SpacetimePoint::new(0.0, 10.0, PI / 2.0, 0.0);
    let off = SpacetimePoint::new(0.0, 10.0, PI / 2.0 + 0.5, 0.0);
    assert!(disk.density(&mid) > disk.density(&off));
}

#[test]
fn test_ray_through_disk_accumulates_intensity() {
    let metric = Schwarzschild::new(1.0);
    let camera = make_camera(30.0);
    let observer = camera.position;
    let disk = AccretionDisk::default();
    let integrator = make_integrator();

    // Equatorial-ish ray skimming past the disk: a slight off-center pixel so
    // it isn't captured but does cross the disk band.
    let mut ray = Ray::from_camera(&metric, &camera, 5, 4, 16, 8);
    let outcome = trace_ray_with_disk(&metric, &mut ray, &disk, &observer, &integrator, 20_000);

    match outcome {
        RayOutcome::Disk { intensity, .. } => assert!(intensity > 0.0),
        other => panic!("expected Disk, got {other:?}"),
    }
}
