use rand::Rng;

use crate::core::{Onb, Vec3};
use crate::pdf::Pdf;

pub struct CosinePdf {
    onb: Onb,
}

impl CosinePdf {
    pub fn new(normal: Vec3) -> Self {
        Self {
            onb: Onb::from_w(normal),
        }
    }
}

impl Pdf for CosinePdf {
    fn value(&self, direction: Vec3) -> f64 {
        let cosine = direction.normalize().dot(&self.onb.w);
        if cosine > 0.0 {
            cosine / std::f64::consts::PI
        } else {
            0.0
        }
    }

    fn generate(&self) -> Vec3 {
        let mut rng = rand::thread_rng();
        let r1: f64 = rng.r#gen();
        let r2: f64 = rng.r#gen();

        let phi = 2.0 * std::f64::consts::PI * r1;
        let sqrt_r2 = r2.sqrt();

        let x = phi.cos() * sqrt_r2;
        let y = phi.sin() * sqrt_r2;
        let z = (1.0 - r2).sqrt();

        self.onb.local(x, y, z).normalize()
    }
}
