use super::{Aabb, Interval, Point3, Ray, Vec3};
use crate::material::Material;

/// Record of a ray-surface intersection.
///
/// Holds a borrowed reference to the material so a hit doesn't have to
/// `Arc::clone(material)` per primitive intersection. The lifetime `'a` is
/// tied to the scene the hit was found in.
pub struct HitRecord<'a> {
    /// Intersection point
    pub point: Point3,
    /// Surface normal (always points against incident ray)
    pub normal: Vec3,
    /// Ray parameter at intersection
    pub t: f64,
    /// UV texture coordinates
    pub uv: (f64, f64),
    /// True if ray hit from outside the surface
    pub front_face: bool,
    /// Material at hit point — borrowed from the owning primitive.
    pub material: &'a dyn Material,
}

impl<'a> HitRecord<'a> {
    /// Create a hit record, ensuring normal faces against the ray
    pub fn new(
        ray: &Ray,
        point: Point3,
        outward_normal: Vec3,
        t: f64,
        uv: (f64, f64),
        material: &'a dyn Material,
    ) -> Self {
        let front_face = ray.direction.dot(&outward_normal) < 0.0;
        let normal = if front_face {
            outward_normal
        } else {
            -outward_normal
        };

        Self {
            point,
            normal,
            t,
            uv,
            front_face,
            material,
        }
    }
}

/// Result of sampling a point on a surface
#[derive(Clone, Debug)]
pub struct SurfaceSample {
    /// Sampled point on the surface
    pub point: Point3,
    /// Surface normal at the sampled point
    pub normal: Vec3,
    /// Probability density (per unit area) of sampling this point
    pub pdf: f64,
}

/// Trait for objects that can be hit by rays
pub trait Hittable: Send + Sync {
    /// Test ray intersection within t_range, return closest hit.
    ///
    /// The returned `HitRecord` borrows from `self`, so the implementor must
    /// store its material directly (e.g. `Arc<dyn Material>` or `Box<dyn Material>`)
    /// rather than constructing one on the fly.
    fn hit<'a>(&'a self, ray: &Ray, t_range: Interval) -> Option<HitRecord<'a>>;

    /// Axis-aligned bounding box for acceleration
    fn bounding_box(&self) -> Aabb;

    /// PDF value for sampling this object from a given origin in a given direction
    /// Used for importance sampling lights
    fn pdf_value(&self, _origin: Point3, _direction: Vec3) -> f64 {
        0.0
    }

    /// Generate a random direction toward this object from a given origin
    /// Used for importance sampling lights
    fn random_direction(&self, _origin: Point3) -> Vec3 {
        Vec3::new(1.0, 0.0, 0.0)
    }

    /// Sample a point uniformly on the surface of this object
    ///
    /// Returns the sampled point, surface normal, and area PDF.
    /// Used for BDPT light path generation.
    fn sample_surface(&self) -> Option<SurfaceSample> {
        None
    }

    /// Get the surface area of this object
    ///
    /// Used for uniform surface sampling in BDPT.
    fn area(&self) -> f64 {
        0.0
    }
}
