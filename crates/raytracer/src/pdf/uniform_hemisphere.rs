use crate::core::{Onb, Vec3};
use crate::pdf::Pdf;
use rand::Rng;
use std::f64::consts::PI;

/// Uniform hemisphere sampling PDF
/// Samples directions uniformly over the hemisphere (not cosine-weighted)
/// Useful for comparison and certain BRDFs
pub struct UniformHemispherePdf {
    onb: Onb,
}

impl UniformHemispherePdf {
    pub fn new(normal: Vec3) -> Self {
        Self {
            onb: Onb::from_w(normal),
        }
    }
}

impl Pdf for UniformHemispherePdf {
    fn value(&self, direction: Vec3) -> f64 {
        let cosine = direction.normalize().dot(&self.onb.w);
        if cosine > 0.0 {
            // Uniform hemisphere: PDF = 1 / (2π)
            1.0 / (2.0 * PI)
        } else {
            0.0
        }
    }

    fn generate(&self) -> Vec3 {
        let mut rng = rand::thread_rng();
        let r1: f64 = rng.r#gen();
        let r2: f64 = rng.r#gen();

        // Uniform hemisphere sampling
        let phi = 2.0 * PI * r1;
        let cos_theta = r2;
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        let x = phi.cos() * sin_theta;
        let y = phi.sin() * sin_theta;
        let z = cos_theta;

        self.onb.local(x, y, z).normalize()
    }
}
