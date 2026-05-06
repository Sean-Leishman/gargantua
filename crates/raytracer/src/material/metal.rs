use super::Material;
use crate::core::{Color, Point3, Ray, ScatterRecord, Vec3};
use rand::Rng;

/// Reflective metal material
#[derive(Clone, Debug)]
pub struct Metal {
    pub albedo: Color,
    pub fuzz: f64,
}

impl Metal {
    pub fn new(albedo: Color, fuzz: f64) -> Self {
        Self {
            albedo,
            fuzz: fuzz.clamp(0.0, 1.0),
        }
    }
}

impl Material for Metal {
    fn scatter_pdf(
        &self,
        ray: &Ray,
        hit: &crate::core::HitRecord,
    ) -> Option<crate::core::ScatterRecord> {
        let reflected = reflect(ray.direction.normalize(), hit.normal);
        let fuzzed = reflected + random_in_unit_sphere() * self.fuzz;

        if fuzzed.dot(&hit.normal) > 0.0 {
            Some(ScatterRecord::specular(
                self.albedo,
                Ray::new(hit.point, fuzzed.normalize()),
            ))
        } else {
            None
        }
    }

    fn scatter(
        &self,
        ray: &Ray,
        hit_point: Point3,
        normal: Vec3,
        _front_face: bool,
    ) -> Option<(Color, Ray)> {
        let reflected = reflect(ray.direction.normalize(), normal);
        let scattered_dir = reflected + self.fuzz * random_in_unit_sphere();

        // Only scatter if reflection is in the same hemisphere as normal
        if scattered_dir.dot(&normal) > 0.0 {
            let scattered = Ray::new(hit_point, scattered_dir.normalize());
            Some((self.albedo, scattered))
        } else {
            None
        }
    }

    fn bsdf(&self, _point: Point3, normal: Vec3, wi: Vec3, wo: Vec3) -> Color {
        // For perfect specular (fuzz=0), BSDF is a delta function
        // For fuzzy metals, we use a rough approximation
        if self.fuzz < 0.001 {
            // Perfect specular - return zero for non-specular directions
            return Color::BLACK;
        }

        // Fuzzy metal - approximate with a rough lobe around reflection direction
        let reflected = reflect(wi.normalize(), normal);
        let cos_angle = reflected.dot(&wo.normalize());

        if cos_angle > 0.0 {
            // Approximate GGX-like lobe
            let roughness = self.fuzz;
            let cos_angle_sq = cos_angle * cos_angle;
            let roughness_sq = roughness * roughness;
            let denom = cos_angle_sq * (roughness_sq - 1.0) + 1.0;
            let d = roughness_sq / (std::f64::consts::PI * denom * denom);

            self.albedo * d
        } else {
            Color::BLACK
        }
    }

    fn pdf(&self, _point: Point3, normal: Vec3, wi: Vec3, wo: Vec3) -> f64 {
        if self.fuzz < 0.001 {
            return 0.0; // Delta distribution
        }

        let reflected = reflect(wi.normalize(), normal);
        let cos_angle = reflected.dot(&wo.normalize());

        if cos_angle > 0.0 {
            // Approximate PDF for fuzzy reflection
            let roughness = self.fuzz;
            (1.0 - roughness).powi(2) * cos_angle / std::f64::consts::PI
        } else {
            0.0
        }
    }

    fn is_delta(&self) -> bool {
        self.fuzz < 0.001
    }
}

/// Reflect vector v about normal n
#[inline]
fn reflect(v: Vec3, n: Vec3) -> Vec3 {
    v - 2.0 * v.dot(&n) * n
}

/// Random point in unit sphere (for fuzzy reflections)
fn random_in_unit_sphere() -> Vec3 {
    let mut rng = rand::thread_rng();
    loop {
        let v = Vec3::new(
            rng.r#gen_range(-1.0..1.0),
            rng.r#gen_range(-1.0..1.0),
            rng.r#gen_range(-1.0..1.0),
        );
        if v.magnitude_squared() < 1.0 {
            return v;
        }
    }
}
