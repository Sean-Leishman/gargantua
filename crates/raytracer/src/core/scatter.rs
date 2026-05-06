use crate::core::{Color, Ray, Vec3};
use crate::pdf::{CosinePdf, Pdf};

/// PDF stored inline in a ScatterRecord. Avoids boxing `dyn Pdf` on every
/// non-specular hit (which showed up as ~3% libc allocator time in profiling).
/// Add variants here when a new BSDF needs a different sampling distribution.
pub enum ScatterPdf {
    Cosine(CosinePdf),
}

impl ScatterPdf {
    #[inline]
    pub fn value(&self, direction: Vec3) -> f64 {
        match self {
            ScatterPdf::Cosine(p) => p.value(direction),
        }
    }

    #[inline]
    pub fn generate(&self) -> Vec3 {
        match self {
            ScatterPdf::Cosine(p) => p.generate(),
        }
    }
}

pub struct ScatterRecord {
    pub attenuation: Color,
    pub pdf: Option<ScatterPdf>,
    pub specular_ray: Option<Ray>,
    pub is_specular: bool,
}

impl ScatterRecord {
    pub fn diffuse(attenuation: Color, pdf: ScatterPdf) -> Self {
        Self {
            attenuation,
            pdf: Some(pdf),
            specular_ray: None,
            is_specular: false,
        }
    }

    pub fn specular(attenuation: Color, ray: Ray) -> Self {
        Self {
            attenuation,
            pdf: None,
            specular_ray: Some(ray),
            is_specular: true,
        }
    }
}
