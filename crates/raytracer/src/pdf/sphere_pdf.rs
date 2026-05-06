use crate::core::{Onb, Point3, Vec3};
use crate::pdf::Pdf;
use rand::Rng;
use std::f64::consts::PI;

/// PDF for sampling directions toward a sphere using solid angle
/// This is the mathematically correct way to importance sample spherical lights
pub struct SpherePdf {
    center: Point3,
    radius: f64,
    origin: Point3,
}

impl SpherePdf {
    pub fn new(origin: Point3, center: Point3, radius: f64) -> Self {
        Self {
            center,
            radius,
            origin,
        }
    }

    /// Compute the solid angle subtended by the sphere from the origin
    fn solid_angle(&self) -> f64 {
        let dist_squared = (self.center - self.origin).magnitude_squared();
        let cos_theta_max = (1.0 - self.radius * self.radius / dist_squared).sqrt();
        2.0 * PI * (1.0 - cos_theta_max)
    }
}

impl Pdf for SpherePdf {
    fn value(&self, direction: Vec3) -> f64 {
        // Check if direction actually points toward the sphere
        let to_center = self.center - self.origin;
        let dist_squared = to_center.magnitude_squared();

        // If we're inside the sphere, uniform PDF over full sphere
        if dist_squared <= self.radius * self.radius {
            return 1.0 / (4.0 * PI);
        }

        let cos_theta_max = (1.0 - self.radius * self.radius / dist_squared).sqrt();
        let cos_theta = direction.normalize().dot(&to_center.normalize());

        // Check if direction is within the cone toward the sphere
        if cos_theta < cos_theta_max {
            return 0.0;
        }

        // PDF = 1 / solid_angle
        let solid_angle = 2.0 * PI * (1.0 - cos_theta_max);
        1.0 / solid_angle
    }

    fn generate(&self) -> Vec3 {
        let mut rng = rand::thread_rng();
        let r1: f64 = rng.r#gen();
        let r2: f64 = rng.r#gen();

        let to_center = self.center - self.origin;
        let dist_squared = to_center.magnitude_squared();

        // If inside sphere, sample uniformly
        if dist_squared <= self.radius * self.radius {
            return random_unit_sphere();
        }

        let cos_theta_max = (1.0 - self.radius * self.radius / dist_squared).sqrt();

        // Sample within cone
        let z = 1.0 + r2 * (cos_theta_max - 1.0);
        let phi = 2.0 * PI * r1;
        let x = phi.cos() * (1.0 - z * z).sqrt();
        let y = phi.sin() * (1.0 - z * z).sqrt();

        // Transform to world space
        let onb = Onb::from_w(to_center);
        onb.local(x, y, z).normalize()
    }
}

/// Random point on unit sphere surface
fn random_unit_sphere() -> Vec3 {
    let mut rng = rand::thread_rng();
    loop {
        let v = Vec3::new(
            rng.r#gen_range(-1.0..1.0),
            rng.r#gen_range(-1.0..1.0),
            rng.r#gen_range(-1.0..1.0),
        );
        let len_sq = v.magnitude_squared();
        if len_sq > 1e-6 && len_sq <= 1.0 {
            return v / len_sq.sqrt();
        }
    }
}
