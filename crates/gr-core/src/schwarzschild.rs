use crate::metric::{
    ChristoffelSymbols, FourVelocity, Metric, MetricTensor, SpacetimePoint,
    circular_orbit_velocity,
};
use nalgebra::Vector4;

pub struct Schwarzschild {
    pub mass: f64,
    pub rs: f64,
}

impl Schwarzschild {
    pub fn new(mass: f64) -> Self {
        Self {
            mass,
            rs: 2.0 * mass,
        }
    }

    pub fn with_radius(&mut self, rs: f64) -> Self {
        Self { mass: rs / 2.0, rs }
    }
}

impl Metric for Schwarzschild {
    fn metric_tensor(&self, pos: &SpacetimePoint) -> MetricTensor {
        let r = pos[1];
        let theta = pos[2];

        // Avoid singularity at r = 0
        let r = r.max(1e-10);

        let f = 1.0 - self.rs / r;
        let sin_theta = theta.sin();

        // g_μν in (t, r, θ, φ) coordinates
        MetricTensor::from_diagonal(&Vector4::new(
            -f,                            // g_tt
            1.0 / f,                       // g_rr
            r * r,                         // g_θθ
            r * r * sin_theta * sin_theta, // g_φφ
        ))
    }

    fn christoffel(&self, pos: &SpacetimePoint) -> ChristoffelSymbols {
        let r = pos[1].max(1e-10);
        let theta = pos[2];
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        let rs = self.rs;
        let f = 1.0 - rs / r;

        let mut gamma = [[[0.0; 4]; 4]; 4];

        // Non-zero Christoffel symbols for Schwarzschild metric
        // Γ^t_tr = Γ^t_rt = rs / (2r(r - rs))
        gamma[0][0][1] = rs / (2.0 * r * (r - rs));
        gamma[0][1][0] = gamma[0][0][1];

        // Γ^r_tt = rs(r - rs) / (2r³)
        gamma[1][0][0] = rs * (r - rs) / (2.0 * r * r * r);

        // Γ^r_rr = -rs / (2r(r - rs))
        gamma[1][1][1] = -rs / (2.0 * r * (r - rs));

        // Γ^r_θθ = -(r - rs)
        gamma[1][2][2] = -(r - rs);

        // Γ^r_φφ = -(r - rs)sin²θ
        gamma[1][3][3] = -(r - rs) * sin_theta * sin_theta;

        // Γ^θ_rθ = Γ^θ_θr = 1/r
        gamma[2][1][2] = 1.0 / r;
        gamma[2][2][1] = 1.0 / r;

        // Γ^θ_φφ = -sinθ cosθ
        gamma[2][3][3] = -sin_theta * cos_theta;

        // Γ^φ_rφ = Γ^φ_φr = 1/r
        gamma[3][1][3] = 1.0 / r;
        gamma[3][3][1] = 1.0 / r;

        // Γ^φ_θφ = Γ^φ_φθ = cotθ
        gamma[3][2][3] = cos_theta / sin_theta;
        gamma[3][3][2] = gamma[3][2][3];

        gamma
    }

    fn event_horizon(&self) -> Option<f64> {
        Some(self.rs)
    }

    fn orbital_four_velocity(&self, pos: &SpacetimePoint) -> Option<FourVelocity> {
        let r = pos[1];
        if r <= 3.0 * self.mass {
            return None; // inside photon sphere — no timelike circular orbit
        }
        let omega = (self.mass / (r * r * r)).sqrt();
        circular_orbit_velocity(self, pos, omega)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_keplerian_orbit_is_normalized_timelike() {
        let metric = Schwarzschild::new(1.0);
        let pos = SpacetimePoint::new(0.0, 10.0, PI / 2.0, 0.0);
        let u = metric.orbital_four_velocity(&pos).expect("r=10M is outside photon sphere");
        let g = metric.metric_tensor(&pos);
        let norm = (g * u).dot(&u);
        assert!((norm + 1.0).abs() < 1e-9, "u·u = {norm}, expected -1");
        assert!(u[3] > 0.0, "prograde orbit should have u^φ > 0");
    }

    #[test]
    fn test_no_circular_orbit_inside_photon_sphere() {
        let metric = Schwarzschild::new(1.0);
        let pos = SpacetimePoint::new(0.0, 2.5, PI / 2.0, 0.0);
        assert!(metric.orbital_four_velocity(&pos).is_none());
    }

    #[test]
    fn test_schwarzschild_flat_at_infinity() {
        let metric = Schwarzschild::new(1.0);
        let pos = SpacetimePoint::new(0.0, 1000.0, PI / 2.0, 0.0);
        let g = metric.metric_tensor(&pos);

        // At large r, should approach flat Minkowski metric
        assert!((g[(0, 0)] + 1.0).abs() < 0.01);
        assert!((g[(1, 1)] - 1.0).abs() < 0.01);
    }
}
