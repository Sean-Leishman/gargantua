use crate::curved::camera::Camera;
use gr_core::{GeodesicState, Metric, SpacetimePoint};
use nalgebra::{Vector3, Vector4};

/// A null geodesic in flight through curved spacetime.
///
/// Distinct from `raytracer::core::Ray`, which is a straight Euclidean ray.
pub struct GeodesicRay {
    pub state: GeodesicState,
    pub pixel: (u32, u32),
}

impl GeodesicRay {
    pub fn from_camera<M: Metric>(
        metric: &M,
        camera: &Camera,
        px: u32,
        py: u32,
        width: u32,
        height: u32,
    ) -> Self {
        let u = (px as f64 + 0.5) / width as f64 * 2.0 - 1.0;
        let v = 1.0 - (py as f64 + 0.5) / height as f64 * 2.0;

        let n_cart: Vector3<f64> = camera.pixel_direction(u, v);
        let pos: SpacetimePoint = camera.position;

        // Local orthonormal spherical basis (cartesian components) at camera position.
        let theta = pos[2];
        let phi = pos[3];
        let (st, ct) = (theta.sin(), theta.cos());
        let (sp, cp) = (phi.sin(), phi.cos());
        let r_hat = Vector3::new(st * cp, st * sp, ct);
        let theta_hat = Vector3::new(ct * cp, ct * sp, -st);
        let phi_hat = Vector3::new(-sp, cp, 0.0);

        // Project cartesian camera direction onto the local spherical orthonormal frame.
        let n_r = n_cart.dot(&r_hat);
        let n_t = n_cart.dot(&theta_hat);
        let n_p = n_cart.dot(&phi_hat);

        // Convert local-frame unit components to coordinate basis: k^i = n_î / sqrt(g_ii).
        // This is the Minkowski-tetrad approximation valid for diagonal metrics; off-diagonal
        // (Kerr g_tφ) will need a proper ZAMO tetrad.
        let g = metric.metric_tensor(&pos);
        let kr = n_r / g[(1, 1)].sqrt();
        let kth = n_t / g[(2, 2)].sqrt();
        let kph = n_p / g[(3, 3)].sqrt();

        // Null condition: g_tt (k^t)^2 + g_ij k^i k^j = 0  (assumes g_tφ ≈ 0).
        let spatial_norm_sq =
            g[(1, 1)] * kr * kr + g[(2, 2)] * kth * kth + g[(3, 3)] * kph * kph;
        let kt = (-spatial_norm_sq / g[(0, 0)]).sqrt();

        let velocity = Vector4::new(kt, kr, kth, kph);

        Self {
            state: GeodesicState::new(pos, velocity),
            pixel: (px, py),
        }
    }
}
