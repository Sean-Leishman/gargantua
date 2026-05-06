use super::Material;
use crate::{
    core::{Color, Point3, Ray, ScatterPdf, ScatterRecord, Vec3},
    pdf::CosinePdf,
};
use rand::Rng;

/// Diffuse (Lambertian) material
#[derive(Clone, Debug)]
pub struct Lambertian {
    pub albedo: Color,
}

impl Lambertian {
    pub fn new(albedo: Color) -> Self {
        Self { albedo }
    }
}

impl Material for Lambertian {
    fn scatter_pdf(
        &self,
        _ray: &Ray,
        hit: &crate::core::HitRecord,
    ) -> Option<crate::core::ScatterRecord> {
        Some(ScatterRecord::diffuse(
            self.albedo,
            ScatterPdf::Cosine(CosinePdf::new(hit.normal)),
        ))
    }

    fn scattering_pdf(&self, _ray: &Ray, hit: &crate::core::HitRecord, scattered: &Ray) -> f64 {
        let cosine = hit.normal.dot(&scattered.direction.normalize());
        if cosine > 0.0 {
            cosine / std::f64::consts::PI
        } else {
            0.0
        }
    }

    fn scatter(
        &self,
        _ray: &Ray,
        hit_point: Point3,
        normal: Vec3,
        _front_face: bool,
    ) -> Option<(Color, Ray)> {
        let mut scatter_dir = normal + random_unit_vector();

        // Catch degenerate scatter direction
        if scatter_dir.magnitude_squared() < 1e-8 {
            scatter_dir = normal;
        }

        let scattered = Ray::new(hit_point, scatter_dir.normalize());
        Some((self.albedo, scattered))
    }

    fn bsdf(&self, _point: Point3, normal: Vec3, wi: Vec3, wo: Vec3) -> Color {
        // Lambertian BSDF: f = albedo / pi
        let cos_i = wi.normalize().dot(&normal);
        let cos_o = wo.normalize().dot(&normal);

        // Both directions should be in the upper hemisphere
        if cos_i * cos_o > 0.0 {
            self.albedo / std::f64::consts::PI
        } else {
            Color::BLACK
        }
    }

    fn pdf(&self, _point: Point3, normal: Vec3, _wi: Vec3, wo: Vec3) -> f64 {
        let cos_theta = wo.normalize().dot(&normal);
        if cos_theta > 0.0 {
            cos_theta / std::f64::consts::PI
        } else {
            0.0
        }
    }

    fn is_delta(&self) -> bool {
        false
    }
}

/// Random unit vector for diffuse scattering
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
