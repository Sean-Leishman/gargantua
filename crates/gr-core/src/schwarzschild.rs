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

    /// Direct geodesic RHS bypassing the dense Christoffel tensor.
    ///
    /// Schwarzschild has 13 non-zero Γ^μ_αβ entries; the generic path
    /// (a) zeros a 512-byte `[[[f64;4];4];4]` array and (b) multiplies
    /// through all 64 components, ~51 of which are multiply-by-zero.
    /// Profiling showed those costs at ~6.7% (memset) + a chunk of
    /// `geodesic_acceleration`'s 29%. Inlining the formula here cuts
    /// both. Equivalent to `-Γ^μ_αβ v^α v^β` for the symbols listed in
    /// `christoffel` above (factors of 2 absorbed where α ≠ β).
    fn geodesic_acceleration(
        &self,
        pos: &SpacetimePoint,
        vel: &FourVelocity,
    ) -> FourVelocity {
        let r = pos[1].max(1e-10);
        let theta = pos[2];
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        let rs = self.rs;
        let f_denom = r * (r - rs); // 2r(r-rs) / 2 — keep as one product
        let rmrs = r - rs;
        let inv_r = 1.0 / r;

        // Γ values (no array storage; only the non-zero ones).
        let g_t_tr = rs / (2.0 * f_denom);            // Γ^t_tr = Γ^t_rt
        let g_r_tt = rs * rmrs / (2.0 * r * r * r);   // Γ^r_tt
        let g_r_rr = -rs / (2.0 * f_denom);           // Γ^r_rr
        let g_r_thth = -rmrs;                         // Γ^r_θθ
        let g_r_phph = -rmrs * sin_theta * sin_theta; // Γ^r_φφ
        // Γ^θ_rθ = Γ^θ_θr = 1/r
        let g_th_phph = -sin_theta * cos_theta;       // Γ^θ_φφ
        // Γ^φ_rφ = Γ^φ_φr = 1/r
        let g_ph_thph = cos_theta / sin_theta;        // Γ^φ_θφ = Γ^φ_φθ

        let vt = vel[0];
        let vr = vel[1];
        let vth = vel[2];
        let vph = vel[3];

        // a^μ = -Σ Γ^μ_αβ v^α v^β. Symmetric pairs (α≠β) get a factor of 2.
        let acc_t = -2.0 * g_t_tr * vt * vr;
        let acc_r = -g_r_tt * vt * vt
            - g_r_rr * vr * vr
            - g_r_thth * vth * vth
            - g_r_phph * vph * vph;
        let acc_th = -2.0 * inv_r * vr * vth - g_th_phph * vph * vph;
        let acc_ph = -2.0 * inv_r * vr * vph - 2.0 * g_ph_thph * vth * vph;

        Vector4::new(acc_t, acc_r, acc_th, acc_ph)
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
