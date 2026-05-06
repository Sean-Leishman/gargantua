mod cosine;
mod hittable_pdf;
mod mixture_pdf;
mod sphere_pdf;
mod uniform_hemisphere;

pub use cosine::CosinePdf;
pub use hittable_pdf::HittablePdf;
pub use mixture_pdf::MixturePdf;
pub use sphere_pdf::SpherePdf;
pub use uniform_hemisphere::UniformHemispherePdf;

use crate::core::Vec3;

/// Probability Distribution Function for importance sampling
pub trait Pdf: Send + Sync {
    /// Evaluate the PDF for a given direction
    fn value(&self, direction: Vec3) -> f64;

    /// Generate a random direction according to this PDF
    fn generate(&self) -> Vec3;
}
