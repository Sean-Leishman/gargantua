use super::Material;
use crate::core::{Color, Point3, Ray, ScatterRecord, Vec3};
use rand::Rng;

/// Dielectric (glass/water) material with refraction
#[derive(Clone, Debug)]
pub struct Dielectric {
    /// Index of refraction (1.0 = air, 1.5 = glass, 2.4 = diamond)
    pub ior: f64,
}

impl Dielectric {
    pub fn new(ior: f64) -> Self {
        Self { ior }
    }

    /// Glass with IOR 1.5
    pub fn glass() -> Self {
        Self::new(1.5)
    }

    /// Water with IOR 1.33
    pub fn water() -> Self {
        Self::new(1.33)
    }

    /// Diamond with IOR 2.4
    pub fn diamond() -> Self {
        Self::new(2.4)
    }
}

impl Material for Dielectric {
    fn scatter_pdf(
        &self,
        ray: &Ray,
        hit: &crate::core::HitRecord,
    ) -> Option<crate::core::ScatterRecord> {
        let attenuation = Color::WHITE;
        let ri = if hit.front_face {
            1.0 / self.ior
        } else {
            self.ior
        };

        let unit_dir = ray.direction.normalize();
        let cos_theta = (-unit_dir).dot(&hit.normal).min(1.0);
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        let cannot_refract = ri * sin_theta > 1.0;
        let mut rng = rand::thread_rng();

        let direction = if cannot_refract || reflectance(cos_theta, ri) > rng.r#gen() {
            reflect(unit_dir, hit.normal)
        } else {
            refract(unit_dir, hit.normal, ri)
        };

        Some(ScatterRecord::specular(
            attenuation,
            Ray::new(hit.point, direction),
        ))
    }

    fn scatter(
        &self,
        ray: &Ray,
        hit_point: Point3,
        normal: Vec3,
        front_face: bool,
    ) -> Option<(Color, Ray)> {
        let attenuation = Color::WHITE;
        let refraction_ratio = if front_face { 1.0 / self.ior } else { self.ior };

        let unit_dir = ray.direction.normalize();
        let cos_theta = (-unit_dir).dot(&normal).min(1.0);
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        let cannot_refract = refraction_ratio * sin_theta > 1.0;
        let mut rng = rand::thread_rng();

        let direction = if cannot_refract || reflectance(cos_theta, refraction_ratio) > rng.r#gen() {
            reflect(unit_dir, normal)
        } else {
            refract(unit_dir, normal, refraction_ratio)
        };

        let scattered = Ray::new(hit_point, direction);
        Some((attenuation, scattered))
    }

    fn bsdf(&self, _point: Point3, _normal: Vec3, _wi: Vec3, _wo: Vec3) -> Color {
        // Dielectric is a delta distribution - BSDF is zero for arbitrary directions
        Color::BLACK
    }

    fn pdf(&self, _point: Point3, _normal: Vec3, _wi: Vec3, _wo: Vec3) -> f64 {
        // Delta distribution - PDF is zero for any specific direction
        0.0
    }

    fn is_delta(&self) -> bool {
        true
    }
}

/// Reflect vector v about normal n
#[inline]
fn reflect(v: Vec3, n: Vec3) -> Vec3 {
    v - 2.0 * v.dot(&n) * n
}

/// Refract vector using Snell's law
#[inline]
fn refract(uv: Vec3, n: Vec3, etai_over_etat: f64) -> Vec3 {
    let cos_theta = (-uv).dot(&n).min(1.0);
    let r_out_perp = etai_over_etat * (uv + cos_theta * n);
    let r_out_parallel = -(1.0 - r_out_perp.magnitude_squared()).abs().sqrt() * n;
    r_out_perp + r_out_parallel
}

/// Schlick's approximation for reflectance
#[inline]
fn reflectance(cosine: f64, ref_idx: f64) -> f64 {
    let r0 = ((1.0 - ref_idx) / (1.0 + ref_idx)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
}
