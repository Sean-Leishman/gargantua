use crate::core::Vec3;
use crate::pdf::Pdf;
use rand::Rng;

/// Mixture of two PDFs for Multiple Importance Sampling (MIS)
pub struct MixturePdf<'a> {
    pdf1: &'a dyn Pdf,
    pdf2: &'a dyn Pdf,
    weight: f64,
}

impl<'a> MixturePdf<'a> {
    /// Create a mixture PDF with equal weighting (0.5)
    pub fn new(pdf1: &'a dyn Pdf, pdf2: &'a dyn Pdf) -> Self {
        Self {
            pdf1,
            pdf2,
            weight: 0.5,
        }
    }

    /// Create a mixture PDF with custom weighting
    /// weight is the probability of sampling from pdf1
    pub fn with_weight(pdf1: &'a dyn Pdf, pdf2: &'a dyn Pdf, weight: f64) -> Self {
        Self { pdf1, pdf2, weight }
    }
}

impl Pdf for MixturePdf<'_> {
    fn value(&self, direction: Vec3) -> f64 {
        self.weight * self.pdf1.value(direction) + (1.0 - self.weight) * self.pdf2.value(direction)
    }

    fn generate(&self) -> Vec3 {
        if rand::thread_rng().r#gen::<f64>() < self.weight {
            self.pdf1.generate()
        } else {
            self.pdf2.generate()
        }
    }
}
