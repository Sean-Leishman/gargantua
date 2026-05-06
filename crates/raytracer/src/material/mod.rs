mod dielectric;
mod diffuse_light;
mod glossy;
mod lambertian;
mod metal;

pub use dielectric::Dielectric;
pub use diffuse_light::DiffuseLight;
pub use glossy::Glossy;
pub use lambertian::Lambertian;
pub use metal::Metal;

use crate::core::{Color, HitRecord, Point3, Ray, ScatterRecord, Vec3};

/// Trait for materials that determine how light scatters
pub trait Material: Send + Sync {
    fn scatter_pdf(&self, ray: &Ray, hit: &HitRecord) -> Option<ScatterRecord> {
        self.scatter(ray, hit.point, hit.normal, hit.front_face)
            .map(|(attenuation, scattered)| ScatterRecord {
                attenuation,
                pdf: None,
                specular_ray: Some(scattered),
                is_specular: true,
            })
    }

    fn scattering_pdf(&self, _ray: &Ray, _hit: &HitRecord, _scattered: &Ray) -> f64 {
        0.0
    }

    /// Scatter an incoming ray, returning attenuation color and scattered ray
    fn scatter(
        &self,
        ray: &Ray,
        hit_point: Point3,
        normal: Vec3,
        front_face: bool,
    ) -> Option<(Color, Ray)>;

    /// Emitted light (for emissive materials)
    fn emitted(&self, _u: f64, _v: f64, _point: Point3) -> Color {
        Color::BLACK
    }

    /// Evaluate the BSDF (Bidirectional Scattering Distribution Function)
    ///
    /// Returns the BSDF value f(wi, wo) for incoming direction wi and outgoing direction wo.
    /// For BDPT, this is used to evaluate the contribution along connection paths.
    ///
    /// Default implementation uses albedo for diffuse materials.
    fn bsdf(&self, _point: Point3, normal: Vec3, wi: Vec3, wo: Vec3) -> Color {
        // Default: assume diffuse (Lambertian) BSDF
        // f = albedo / pi, but we also need to check hemisphere
        let cos_i = wi.dot(&normal);
        let cos_o = wo.dot(&normal);

        // Both directions should be in the same hemisphere as normal
        if cos_i * cos_o > 0.0 {
            // Return placeholder - materials should override this
            Color::new(0.5, 0.5, 0.5) / std::f64::consts::PI
        } else {
            Color::BLACK
        }
    }

    /// Evaluate the PDF for sampling direction wo given incoming wi
    ///
    /// For diffuse materials, this is typically cosine-weighted hemisphere sampling.
    fn pdf(&self, _point: Point3, normal: Vec3, _wi: Vec3, wo: Vec3) -> f64 {
        // Default: cosine-weighted hemisphere PDF
        let cos_theta = wo.normalize().dot(&normal);
        if cos_theta > 0.0 {
            cos_theta / std::f64::consts::PI
        } else {
            0.0
        }
    }

    /// Whether this material has a delta distribution (perfect specular)
    ///
    /// Delta materials cannot be connected in BDPT since their PDF is zero
    /// for any specific direction.
    fn is_delta(&self) -> bool {
        false
    }
}
