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
        // Cosine-weighted hemisphere via Malley's method: project a
        // uniform-disk sample onto the hemisphere with z = sqrt(1 - r²).
        // We sample the disk by rejection on the unit square (acceptance
        // π/4 ≈ 78.5%, ~1.27 RNG draws on average) — strictly faster
        // than the previous `phi.cos()/phi.sin()` form, which compiled
        // to a libm `sincos` call (~3.4% of total instructions).
        let mut rng = rand::thread_rng();
        let (x, y, s) = loop {
            let a = 2.0 * rng.r#gen::<f64>() - 1.0;
            let b = 2.0 * rng.r#gen::<f64>() - 1.0;
            let s = a * a + b * b;
            if s < 1.0 {
                break (a, b, s);
            }
        };
        let z = (1.0 - s).sqrt();

        self.onb.local(x, y, z).normalize()
    }
}
