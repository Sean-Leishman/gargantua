use crate::core::{Aabb, Color, HitRecord, Hittable, Interval, Point3, Ray, Vec3};
use crate::material::Material;
use rand::Rng;
use std::sync::Arc;

/// Isotropic scattering material for volumetric effects
#[derive(Clone)]
pub struct Isotropic {
    albedo: Color,
}

impl Isotropic {
    pub fn new(color: Color) -> Self {
        Self { albedo: color }
    }

    pub fn white() -> Self {
        Self::new(Color::WHITE)
    }
}

impl Material for Isotropic {
    fn scatter(
        &self,
        _ray: &Ray,
        hit_point: Point3,
        _normal: Vec3,
        _front_face: bool,
    ) -> Option<(Color, Ray)> {
        // Scatter in a random direction (isotropic)
        let scattered = Ray::new(hit_point, random_unit_vector());
        Some((self.albedo, scattered))
    }
}

/// Constant density medium for fog, smoke, and other volumetric effects
pub struct ConstantMedium {
    boundary: Arc<dyn Hittable>,
    neg_inv_density: f64,
    phase_function: Arc<dyn Material>,
}

impl ConstantMedium {
    /// Create a constant medium with given density and color
    ///
    /// # Arguments
    /// * `boundary` - The shape that bounds the medium
    /// * `density` - Density of the medium (higher = more opaque)
    /// * `color` - Color/albedo of the medium
    pub fn new<H: Hittable + 'static>(boundary: H, density: f64, color: Color) -> Self {
        Self {
            boundary: Arc::new(boundary),
            neg_inv_density: -1.0 / density,
            phase_function: Arc::new(Isotropic::new(color)),
        }
    }

    /// Create fog (white, low density)
    pub fn fog<H: Hittable + 'static>(boundary: H, density: f64) -> Self {
        Self::new(boundary, density, Color::WHITE)
    }

    /// Create smoke (dark gray)
    pub fn smoke<H: Hittable + 'static>(boundary: H, density: f64) -> Self {
        Self::new(boundary, density, Color::new(0.2, 0.2, 0.2))
    }
}

impl Hittable for ConstantMedium {
    fn hit<'a>(&'a self, ray: &Ray, t_range: Interval) -> Option<HitRecord<'a>> {
        let mut rng = rand::thread_rng();

        // Find where ray enters the boundary
        let mut hit1 = self
            .boundary
            .hit(ray, Interval::new(f64::NEG_INFINITY, f64::INFINITY))?;

        // Find where ray exits the boundary
        let mut hit2 = self
            .boundary
            .hit(ray, Interval::new(hit1.t + 0.0001, f64::INFINITY))?;

        // Clamp to valid range
        if hit1.t < t_range.min {
            hit1.t = t_range.min;
        }
        if hit2.t > t_range.max {
            hit2.t = t_range.max;
        }

        if hit1.t >= hit2.t {
            return None;
        }

        if hit1.t < 0.0 {
            hit1.t = 0.0;
        }

        let ray_length = ray.direction.magnitude();
        let distance_inside_boundary = (hit2.t - hit1.t) * ray_length;

        // Random distance at which scattering occurs
        let hit_distance = self.neg_inv_density * rng.r#gen::<f64>().ln();

        if hit_distance > distance_inside_boundary {
            return None;
        }

        let t = hit1.t + hit_distance / ray_length;
        let point = ray.at(t);

        // For volumetric media, normal and front_face are arbitrary
        Some(HitRecord::new(
            ray,
            point,
            Vec3::new(1.0, 0.0, 0.0), // arbitrary normal
            t,
            (0.0, 0.0), // no UV
            &*self.phase_function,
        ))
    }

    fn bounding_box(&self) -> Aabb {
        self.boundary.bounding_box()
    }
}

/// Random unit vector for isotropic scattering
fn random_unit_vector() -> Vec3 {
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
