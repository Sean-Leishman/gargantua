//! Curved-spacetime renderer built on `gr-core`.
//!
//! Gated behind the `curved` cargo feature.

pub mod camera;
pub mod disk;
pub mod outcome;
pub mod ray;
pub mod renderer;
pub mod tracer;

pub use camera::Camera;
pub use disk::AccretionDisk;
pub use outcome::RayOutcome;
pub use ray::GeodesicRay;
pub use renderer::{
    RenderOptions, disk_color, encode_srgb, render, render_with_disk, render_with_disk_and_scene,
    render_with_scene, shade_outcome, shade_outcome_linear, sky_color,
};
pub use tracer::{
    trace_ray, trace_ray_with_disk, trace_ray_with_disk_and_scene, trace_ray_with_scene,
};
