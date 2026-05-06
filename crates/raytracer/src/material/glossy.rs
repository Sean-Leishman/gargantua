use super::Material;
use crate::core::{Color, HitRecord, Point3, Ray, ScatterPdf, ScatterRecord, Vec3};
use crate::pdf::CosinePdf;
use rand::Rng;

/// Glossy material with adjustable roughness
/// Blends between perfect mirror (roughness=0) and diffuse (roughness=1)
/// Uses GGX-like microfacet distribution
#[derive(Clone, Debug)]
pub struct Glossy {
    /// Base color/albedo
    pub albedo: Color,
    /// Roughness: 0 = perfect mirror, 1 = fully diffuse
    pub roughness: f64,
    /// Fresnel reflectance at normal incidence (F0)
    /// For dielectrics: ~0.04, for metals: use albedo
    pub f0: f64,
}

impl Glossy {
    /// Create a glossy material
    pub fn new(albedo: Color, roughness: f64) -> Self {
        Self {
            albedo,
            roughness: roughness.clamp(0.0, 1.0),
            f0: 0.04, // Default for dielectrics
        }
    }

    /// Create a glossy metal (uses albedo for Fresnel)
    pub fn metal(albedo: Color, roughness: f64) -> Self {
        Self {
            albedo,
            roughness: roughness.clamp(0.0, 1.0),
            f0: 0.9, // Metals have high F0
        }
    }

    /// Create plastic-like material
    pub fn plastic(albedo: Color, roughness: f64) -> Self {
        Self {
            albedo,
            roughness: roughness.clamp(0.0, 1.0),
            f0: 0.04,
        }
    }

    /// Schlick's approximation for Fresnel reflectance
    fn fresnel_schlick(&self, cos_theta: f64) -> f64 {
        self.f0 + (1.0 - self.f0) * (1.0 - cos_theta).powi(5)
    }
}

impl Material for Glossy {
    fn scatter_pdf(&self, ray: &Ray, hit: &HitRecord) -> Option<ScatterRecord> {
        let mut rng = rand::thread_rng();

        // Compute Fresnel term
        let cos_theta = (-ray.direction.normalize()).dot(&hit.normal).max(0.0);
        let fresnel = self.fresnel_schlick(cos_theta);

        // Decide between specular and diffuse based on Fresnel and roughness
        let specular_prob = fresnel * (1.0 - self.roughness);

        if rng.r#gen::<f64>() < specular_prob {
            // Specular reflection with roughness
            let reflected = reflect(ray.direction.normalize(), hit.normal);
            let alpha = self.roughness * self.roughness; // Roughness squared for perceptual linearity

            // GGX-like importance sampling
            let scattered_dir = if alpha < 0.001 {
                reflected // Perfect mirror
            } else {
                // Sample microfacet normal
                let half = sample_ggx_half(hit.normal, alpha, &mut rng);
                let scattered = reflect(ray.direction.normalize(), half);
                if scattered.dot(&hit.normal) > 0.0 {
                    scattered
                } else {
                    reflected
                }
            };

            if scattered_dir.dot(&hit.normal) > 0.0 {
                Some(ScatterRecord::specular(
                    self.albedo,
                    Ray::new(hit.point, scattered_dir.normalize()),
                ))
            } else {
                None
            }
        } else {
            // Diffuse reflection
            Some(ScatterRecord::diffuse(
                self.albedo * (1.0 - fresnel),
                ScatterPdf::Cosine(CosinePdf::new(hit.normal)),
            ))
        }
    }

    fn scattering_pdf(&self, _ray: &Ray, hit: &HitRecord, scattered: &Ray) -> f64 {
        let cosine = hit.normal.dot(&scattered.direction.normalize());
        if cosine > 0.0 {
            cosine / std::f64::consts::PI
        } else {
            0.0
        }
    }

    fn bsdf(&self, _point: Point3, normal: Vec3, wi: Vec3, wo: Vec3) -> Color {
        let cos_i = wi.normalize().dot(&normal);
        let cos_o = wo.normalize().dot(&normal);

        if cos_i <= 0.0 || cos_o <= 0.0 {
            return Color::BLACK;
        }

        // Fresnel term
        let fresnel = self.fresnel_schlick(cos_i);
        let alpha = self.roughness * self.roughness;

        // Diffuse component
        let diffuse = self.albedo * (1.0 - fresnel) / std::f64::consts::PI;

        // Specular component (simplified GGX)
        let half = (wi.normalize() + wo.normalize()).normalize();
        let n_dot_h = normal.dot(&half).max(0.0);
        let alpha_sq = alpha * alpha;
        let denom = n_dot_h * n_dot_h * (alpha_sq - 1.0) + 1.0;
        let d = alpha_sq / (std::f64::consts::PI * denom * denom);

        let specular = self.albedo * fresnel * d;

        diffuse + specular
    }

    fn pdf(&self, _point: Point3, normal: Vec3, wi: Vec3, wo: Vec3) -> f64 {
        let cos_i = wi.normalize().dot(&normal);
        let cos_o = wo.normalize().dot(&normal);

        if cos_o <= 0.0 {
            return 0.0;
        }

        // Mix of diffuse and specular PDFs
        let fresnel = self.fresnel_schlick(cos_i.max(0.0));
        let specular_prob = fresnel * (1.0 - self.roughness);

        // Diffuse PDF
        let diffuse_pdf = cos_o / std::f64::consts::PI;

        // Specular PDF (simplified)
        let half = (wi.normalize() + wo.normalize()).normalize();
        let n_dot_h = normal.dot(&half).max(0.0);
        let alpha = self.roughness * self.roughness;
        let alpha_sq = alpha * alpha;
        let denom = n_dot_h * n_dot_h * (alpha_sq - 1.0) + 1.0;
        let d = alpha_sq / (std::f64::consts::PI * denom * denom);
        let specular_pdf = d * n_dot_h / (4.0 * wo.normalize().dot(&half).abs().max(0.001));

        (1.0 - specular_prob) * diffuse_pdf + specular_prob * specular_pdf
    }

    fn is_delta(&self) -> bool {
        self.roughness < 0.001
    }

    fn scatter(
        &self,
        ray: &Ray,
        hit_point: Point3,
        normal: Vec3,
        _front_face: bool,
    ) -> Option<(Color, Ray)> {
        let mut rng = rand::thread_rng();

        let cos_theta = (-ray.direction.normalize()).dot(&normal).max(0.0);
        let fresnel = self.fresnel_schlick(cos_theta);
        let specular_prob = fresnel * (1.0 - self.roughness);

        if rng.r#gen::<f64>() < specular_prob {
            // Specular
            let reflected = reflect(ray.direction.normalize(), normal);
            let alpha = self.roughness * self.roughness;
            let scattered_dir = if alpha < 0.001 {
                reflected
            } else {
                let half = sample_ggx_half(normal, alpha, &mut rng);
                let s = reflect(ray.direction.normalize(), half);
                if s.dot(&normal) > 0.0 { s } else { reflected }
            };

            if scattered_dir.dot(&normal) > 0.0 {
                Some((self.albedo, Ray::new(hit_point, scattered_dir.normalize())))
            } else {
                None
            }
        } else {
            // Diffuse
            let scatter_dir = normal + random_unit_vector(&mut rng);
            let scatter_dir = if scatter_dir.magnitude_squared() < 1e-8 {
                normal
            } else {
                scatter_dir.normalize()
            };
            Some((self.albedo * (1.0 - fresnel), Ray::new(hit_point, scatter_dir)))
        }
    }
}

/// Reflect vector v about normal n
#[inline]
fn reflect(v: Vec3, n: Vec3) -> Vec3 {
    v - 2.0 * v.dot(&n) * n
}

/// Sample GGX microfacet distribution half-vector
fn sample_ggx_half<R: Rng>(normal: Vec3, alpha: f64, rng: &mut R) -> Vec3 {
    let r1: f64 = rng.r#gen();
    let r2: f64 = rng.r#gen();

    // GGX importance sampling
    let phi = 2.0 * std::f64::consts::PI * r1;
    let cos_theta = ((1.0 - r2) / (1.0 + (alpha * alpha - 1.0) * r2)).sqrt();
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

    // Local coordinates
    let x = phi.cos() * sin_theta;
    let y = phi.sin() * sin_theta;
    let z = cos_theta;

    // Build orthonormal basis from normal
    let onb = crate::core::Onb::from_w(normal);
    onb.local(x, y, z).normalize()
}

/// Random unit vector
fn random_unit_vector<R: Rng>(rng: &mut R) -> Vec3 {
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
