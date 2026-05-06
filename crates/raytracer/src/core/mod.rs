mod aabb;
mod color;
mod hit;
mod interval;
mod onb;
mod ray;
mod scatter;

pub use aabb::Aabb;
pub use color::Color;
pub use hit::{HitRecord, Hittable, SurfaceSample};
pub use interval::Interval;
pub use onb::Onb;
pub use ray::Ray;
pub use scatter::{ScatterPdf, ScatterRecord};

// Re-export nalgebra types with convenient aliases
pub type Vec3 = nalgebra::Vector3<f64>;
pub type Point3 = nalgebra::Point3<f64>;
pub type Mat4 = nalgebra::Matrix4<f64>;

/// Convenience constructors
#[inline]
pub fn vec3(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new(x, y, z)
}

#[inline]
pub fn point3(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}
