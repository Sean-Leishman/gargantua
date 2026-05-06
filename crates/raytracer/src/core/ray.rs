use super::{Point3, Vec3};

/// A ray with origin and direction
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Point3,
    pub direction: Vec3,
}

impl Ray {
    #[inline]
    pub fn new(origin: Point3, direction: Vec3) -> Self {
        Self { origin, direction }
    }

    /// Point along the ray at parameter t: origin + t * direction
    #[inline]
    pub fn at(&self, t: f64) -> Point3 {
        self.origin + self.direction * t
    }

    /// Create ray with normalized direction
    #[inline]
    pub fn normalized(origin: Point3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }
}
