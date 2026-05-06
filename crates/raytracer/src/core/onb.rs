use crate::core::Vec3;

/// Orthonormal basis for coordinate transformations
#[derive(Clone, Copy, Debug)]
pub struct Onb {
    pub u: Vec3,
    pub v: Vec3,
    pub w: Vec3,
}

impl Onb {
    /// Build ONB from a single vector (typically the normal)
    pub fn from_w(n: Vec3) -> Self {
        let w = n.normalize();
        // Choose axis least parallel to w
        let a = if w.x.abs() > 0.9 {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };
        let v = w.cross(&a).normalize();
        let u = w.cross(&v);
        Self { u, v, w }
    }

    /// Transform from local coordinates to world coordinates
    pub fn local(&self, a: f64, b: f64, c: f64) -> Vec3 {
        self.u * a + self.v * b + self.w * c
    }

    /// Transform a local-space vector to world space
    pub fn local_vec(&self, v: Vec3) -> Vec3 {
        self.local(v.x, v.y, v.z)
    }
}
