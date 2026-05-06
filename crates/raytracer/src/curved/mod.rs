//! Curved-spacetime renderer built on `gr-core`.
//!
//! Gated behind the `curved` cargo feature.

pub mod camera;
pub mod disk;
pub mod outcome;
pub mod ray;
pub mod tracer;

pub use camera::Camera;
pub use disk::AccretionDisk;
pub use outcome::RayOutcome;
pub use ray::GeodesicRay;
pub use tracer::{trace_ray, trace_ray_with_disk};
