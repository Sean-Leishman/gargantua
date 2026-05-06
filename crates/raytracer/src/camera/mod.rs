mod perspective;
mod thin_lens;

pub use perspective::PerspectiveCamera;
pub use thin_lens::ThinLensCamera;

use crate::core::Ray;

/// Trait for cameras that generate rays for each pixel
pub trait Camera: Send + Sync {
    /// Generate a ray for normalized screen coordinates (u, v) in [0, 1]
    /// u=0 is left, u=1 is right
    /// v=0 is bottom, v=1 is top
    fn get_ray(&self, u: f64, v: f64) -> Ray;
}
