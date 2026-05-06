//! Kerr metric for a rotating black hole in Boyer-Lindquist coordinates.
//!
//! Coordinates: (t, r, θ, φ), parameters: mass M, spin a = J/M.
//! Geometric units G = c = 1.

use crate::metric::{
    ChristoffelSymbols, FourVelocity, Metric, MetricTensor, SpacetimePoint,
    circular_orbit_velocity,
};
use nalgebra::Matrix4;

/// Kerr spacetime (rotating black hole).
///
/// Boyer-Lindquist coordinates (t, r, θ, φ), geometric units G = c = 1.
/// The spin parameter `a` satisfies `0 ≤ |a| ≤ M`; inputs with `|a| > M`
/// are clamped to `|a| = M` (extremal black hole).
pub struct Kerr {
    /// Black hole mass.
    pub mass: f64,
    /// Spin parameter a = J/M (satisfies |a| ≤ M after clamping).
    pub spin: f64,
}

impl Kerr {
    /// Construct a Kerr black hole with given mass and spin.
    ///
    /// Panics if `mass ≤ 0`. If `|spin| > mass` the spin is clamped to
    /// `±mass * 0.9999` so the horizon remains well-defined.
    pub fn new(mass: f64, spin: f64) -> Self {
        assert!(mass > 0.0, "Kerr: mass must be positive");
        let max_spin = mass * 0.9999;
        let spin = spin.clamp(-max_spin, max_spin);
        Self { mass, spin }
    }

    /// Σ = r² + a² cos²θ
    #[inline]
    fn sigma(&self, r: f64, theta: f64) -> f64 {
        let a = self.spin;
        let cos_theta = theta.cos();
        r * r + a * a * cos_theta * cos_theta
    }

    /// Δ = r² − 2Mr + a²
    #[inline]
    fn delta(&self, r: f64) -> f64 {
        let m = self.mass;
        let a = self.spin;
        r * r - 2.0 * m * r + a * a
    }
}

impl Default for Kerr {
    /// Default: M = 1, a = 0.9 (high-spin Kerr).
    fn default() -> Self {
        Self::new(1.0, 0.9)
    }
}

impl Metric for Kerr {
    fn metric_tensor(&self, pos: &SpacetimePoint) -> MetricTensor {
        let r = pos[1].max(1e-10);
        let theta = pos[2];

        let m = self.mass;
        let a = self.spin;
        let a2 = a * a;

        let sigma = self.sigma(r, theta);
        let delta = self.delta(r);
        let sin_theta = theta.sin();
        let sin2 = sin_theta * sin_theta;

        let mut g = Matrix4::zeros();

        // g_tt = -(1 - 2Mr/Σ)
        g[(0, 0)] = -(1.0 - 2.0 * m * r / sigma);

        // g_rr = Σ/Δ
        g[(1, 1)] = sigma / delta;

        // g_θθ = Σ
        g[(2, 2)] = sigma;

        // g_φφ = (r² + a² + 2Ma²r sin²θ / Σ) sin²θ
        g[(3, 3)] = (r * r + a2 + 2.0 * m * a2 * r * sin2 / sigma) * sin2;

        // g_tφ = g_φt = -2Mar sin²θ / Σ
        let g_tphi = -2.0 * m * a * r * sin2 / sigma;
        g[(0, 3)] = g_tphi;
        g[(3, 0)] = g_tphi;

        g
    }

    /// Analytical Boyer-Lindquist Christoffel symbols.
    ///
    /// Source: Misner, Thorne & Wheeler §33.5; Chandrasekhar, "The Mathematical
    /// Theory of Black Holes", Ch. 6. Indices: (0=t, 1=r, 2=θ, 3=φ).
    fn christoffel(&self, pos: &SpacetimePoint) -> ChristoffelSymbols {
        let r = pos[1].max(1e-10);
        let theta = pos[2];

        let m = self.mass;
        let a = self.spin;
        let a2 = a * a;
        let m_r = m * r; // Mr shorthand

        let sigma = self.sigma(r, theta);
        let delta = self.delta(r);
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        let sin2 = sin_theta * sin_theta;
        let cos2 = cos_theta * cos_theta;

        // Frequently used composites
        let sigma2 = sigma * sigma;
        let r2 = r * r;
        let a2_cos2 = a2 * cos2; // a² cos²θ  (sigma = r² + a²cos²θ)

        // rho² is a common shorthand (same as sigma but using MTW notation)
        // For clarity we keep using `sigma` throughout.

        // Intermediate quantities from differentiating Σ and Δ
        // ∂Σ/∂r  = 2r
        let dsigma_dr = 2.0 * r;
        // ∂Σ/∂θ  = -2a² sinθ cosθ
        let dsigma_dtheta = -2.0 * a2 * sin_theta * cos_theta;
        // ∂Δ/∂r  = 2r - 2M
        let ddelta_dr = 2.0 * r - 2.0 * m;

        // A = (r² + a²)² − a²Δ sin²θ  (appears in g_φφ)
        let r2_plus_a2 = r2 + a2;
        let big_a = r2_plus_a2 * r2_plus_a2 - a2 * delta * sin2;

        // Inverse metric g^μν (Kerr, closed form):
        //   g^tt  = -A / (Σ Δ)
        //   g^tφ  =  g^φt = -2Mar / (Σ Δ)          [off-diagonal]
        //   g^rr  =  Δ / Σ
        //   g^θθ  =  1 / Σ
        //   g^φφ  =  (Δ - a² sin²θ) / (Σ Δ sin²θ)
        //
        // Note: g^tt and g^tφ can also be written via the 2×2 block inverse.
        let g_inv_tt = -big_a / (sigma * delta);
        let g_inv_tphi = -2.0 * m * a * r / (sigma * delta);
        let g_inv_rr = delta / sigma;
        let g_inv_thth = 1.0 / sigma;
        let g_inv_phph = (delta - a2 * sin2) / (sigma * delta * sin2);

        // ── metric component derivatives ─────────────────────────────────────
        // We differentiate the five independent components w.r.t. r and θ only
        // (the metric is independent of t and φ in Boyer-Lindquist).

        // g_tt = -(1 - 2Mr/Σ) = -1 + 2Mr/Σ
        // ∂g_tt/∂r
        let dg_tt_dr = 2.0 * m / sigma - 2.0 * m * r * dsigma_dr / sigma2;
        // ∂g_tt/∂θ
        let dg_tt_dth = -2.0 * m * r * dsigma_dtheta / sigma2;

        // g_rr = Σ/Δ
        // ∂g_rr/∂r
        let dg_rr_dr = (dsigma_dr * delta - sigma * ddelta_dr) / (delta * delta);
        // ∂g_rr/∂θ
        let dg_rr_dth = dsigma_dtheta / delta;

        // g_θθ = Σ
        // ∂g_θθ/∂r
        let dg_thth_dr = dsigma_dr;
        // ∂g_θθ/∂θ
        let dg_thth_dth = dsigma_dtheta;

        // g_φφ = (r² + a² + 2Ma²r sin²θ/Σ) sin²θ
        //      = (r² + a²) sin²θ + 2Ma²r sin⁴θ / Σ
        //
        // Let P = 2Ma²r, Q = P sin²θ / Σ
        let big_p = 2.0 * m * a2 * r;
        let big_q = big_p * sin2 / sigma;

        // ∂g_φφ/∂r
        let dg_phph_dr = 2.0 * r * sin2
            + (2.0 * m * a2 * sin2 * sigma - big_p * sin2 * dsigma_dr) / sigma2 * sin2;
        // Break apart: g_φφ = (r²+a²)sin²θ + P sin⁴θ / Σ
        //  d/dr[(r²+a²)sin²θ] = 2r sin²θ
        //  d/dr[P sin⁴θ / Σ]  = (2Ma² sin⁴θ Σ - P sin⁴θ · 2r) / Σ²
        //                      = sin⁴θ (2Ma² Σ - P · 2r) / Σ²
        //                      = sin⁴θ · 2Ma²(Σ - 2r²) / Σ²     [since P=2Ma²r]
        let dg_phph_dr_clean =
            2.0 * r * sin2 + sin2 * sin2 * 2.0 * m * a2 * (sigma - 2.0 * r2) / sigma2;

        // ∂g_φφ/∂θ
        // d/dθ[(r²+a²)sin²θ] = 2(r²+a²) sinθ cosθ
        // d/dθ[P sin⁴θ / Σ]  = P(4 sin³θ cosθ Σ - sin⁴θ · dsigma_dtheta) / Σ²
        let dg_phph_dth = 2.0 * r2_plus_a2 * sin_theta * cos_theta
            + big_p * (4.0 * sin_theta * sin_theta * sin_theta * cos_theta * sigma
                - sin2 * sin2 * dsigma_dtheta)
                / sigma2;

        // g_tφ = -2Mar sin²θ / Σ
        // ∂g_tφ/∂r
        let dg_tphi_dr = -2.0 * m * a * sin2 * (sigma - r * dsigma_dr) / sigma2;
        // ∂g_tφ/∂θ
        let dg_tphi_dth =
            -2.0 * m * a * r * (2.0 * sin_theta * cos_theta * sigma - sin2 * dsigma_dtheta)
                / sigma2;

        // Replace the messy intermediate with the cleaned version
        let _ = dg_phph_dr; // silence unused warning
        let dg_phph_dr = dg_phph_dr_clean;

        // ── Christoffel computation ──────────────────────────────────────────
        // Γ^μ_αβ = ½ g^μν (∂_α g_νβ + ∂_β g_να − ∂_ν g_αβ)
        //
        // Because the metric depends only on r (index 1) and θ (index 2),
        // derivatives w.r.t. t and φ vanish.  We label the non-zero partial
        // derivatives as:
        //   dg[i][j][1] = ∂_r g_{ij}
        //   dg[i][j][2] = ∂_θ g_{ij}
        //
        // Rather than storing a full derivative array, we build the Christoffel
        // symbols directly from the closed-form expressions.

        let mut gamma = [[[0.0f64; 4]; 4]; 4];

        // Helper: ½ g^μν (∂_α g_νβ + ∂_β g_να − ∂_σ g_αβ) summed over ν.
        // We inline the computation for each non-zero (μ, α, β) using the
        // fact that g^μν is block-diagonal: {t,φ} × {t,φ} and {r}, {θ}.

        // ── Γ^t_αβ  (μ = 0) ─────────────────────────────────────────────────
        // g^tν non-zero for ν = t (0) and ν = φ (3).
        //
        // Γ^t_tr = Γ^t_rt  (α=0,β=1 and sym)
        {
            // ½ [g^tt(∂_t g_tr + ∂_r g_tt − ∂_t g_tr)  + g^tφ(∂_t g_φr + ∂_r g_φt − ∂_t g_φr)]
            // = ½ [g^tt ∂_r g_tt  + g^tφ ∂_r g_φt]
            let val = 0.5 * (g_inv_tt * dg_tt_dr + g_inv_tphi * dg_tphi_dr);
            gamma[0][0][1] = val;
            gamma[0][1][0] = val;
        }

        // Γ^t_tθ = Γ^t_θt  (α=0,β=2)
        {
            let val = 0.5 * (g_inv_tt * dg_tt_dth + g_inv_tphi * dg_tphi_dth);
            gamma[0][0][2] = val;
            gamma[0][2][0] = val;
        }

        // Γ^t_rφ = Γ^t_φr  (α=1,β=3)
        {
            // ½ g^tν (∂_r g_νφ + ∂_φ g_νr − ∂_ν g_rφ)
            // ∂_φ and ∂_t of metric = 0 → simplifies to ½ g^tν ∂_r g_νφ  for ν=t,φ
            // − ∂_ν g_rφ: only non-zero for ν=r,θ but g^tr = g^tθ = 0 → 0
            // = ½ (g^tt ∂_r g_tφ + g^tφ ∂_r g_φφ)
            let val = 0.5 * (g_inv_tt * dg_tphi_dr + g_inv_tphi * dg_phph_dr);
            gamma[0][1][3] = val;
            gamma[0][3][1] = val;
        }

        // Γ^t_θφ = Γ^t_φθ  (α=2,β=3)
        {
            let val = 0.5 * (g_inv_tt * dg_tphi_dth + g_inv_tphi * dg_phph_dth);
            gamma[0][2][3] = val;
            gamma[0][3][2] = val;
        }

        // ── Γ^r_αβ  (μ = 1) ─────────────────────────────────────────────────
        // g^rν non-zero only for ν = r (1).

        // Γ^r_tt  (α=0,β=0)
        {
            // ½ g^rr (∂_t g_rt + ∂_t g_rt − ∂_r g_tt)
            // = ½ g^rr (−∂_r g_tt)
            gamma[1][0][0] = -0.5 * g_inv_rr * dg_tt_dr;
        }

        // Γ^r_rr  (α=1,β=1)
        {
            // ½ g^rr ∂_r g_rr
            gamma[1][1][1] = 0.5 * g_inv_rr * dg_rr_dr;
        }

        // Γ^r_θθ  (α=2,β=2)
        {
            // ½ g^rr (−∂_r g_θθ)
            gamma[1][2][2] = -0.5 * g_inv_rr * dg_thth_dr;
        }

        // Γ^r_φφ  (α=3,β=3)
        {
            // ½ g^rr (−∂_r g_φφ) + g^rr · ½ (0 + 0 − ∂_r g_φφ)
            // Wait — also need the g_tφ contribution:
            // Γ^r_φφ = ½ g^rr (∂_φ g_rφ + ∂_φ g_rφ − ∂_r g_φφ)
            //         = ½ g^rr (−∂_r g_φφ)    [∂_φ = 0]
            gamma[1][3][3] = -0.5 * g_inv_rr * dg_phph_dr;
        }

        // Γ^r_rθ = Γ^r_θr  (α=1,β=2)
        {
            // ½ g^rr ∂_θ g_rr   [the ∂_r g_rθ term vanishes since g_rθ=0]
            let val = 0.5 * g_inv_rr * dg_rr_dth;
            gamma[1][1][2] = val;
            gamma[1][2][1] = val;
        }

        // Γ^r_tφ = Γ^r_φt  (α=0,β=3)
        {
            // ½ g^rr (∂_t g_rφ + ∂_φ g_rt − ∂_r g_tφ)
            // = ½ g^rr (−∂_r g_tφ)    [g_rφ = g_rt = 0, ∂_t = ∂_φ = 0]
            let val = -0.5 * g_inv_rr * dg_tphi_dr;
            gamma[1][0][3] = val;
            gamma[1][3][0] = val;
        }

        // ── Γ^θ_αβ  (μ = 2) ─────────────────────────────────────────────────
        // g^θν non-zero only for ν = θ (2).

        // Γ^θ_tt  (α=0,β=0)
        {
            gamma[2][0][0] = -0.5 * g_inv_thth * dg_tt_dth;
        }

        // Γ^θ_rr  (α=1,β=1)
        {
            gamma[2][1][1] = -0.5 * g_inv_thth * dg_rr_dth;
        }

        // Γ^θ_θθ  (α=2,β=2)
        {
            gamma[2][2][2] = 0.5 * g_inv_thth * dg_thth_dth;
        }

        // Γ^θ_φφ  (α=3,β=3)
        {
            gamma[2][3][3] = -0.5 * g_inv_thth * dg_phph_dth;
        }

        // Γ^θ_rθ = Γ^θ_θr  (α=1,β=2)
        {
            // ½ g^θθ ∂_r g_θθ
            let val = 0.5 * g_inv_thth * dg_thth_dr;
            gamma[2][1][2] = val;
            gamma[2][2][1] = val;
        }

        // Γ^θ_tφ = Γ^θ_φt  (α=0,β=3)
        {
            // ½ g^θθ (−∂_θ g_tφ)
            let val = -0.5 * g_inv_thth * dg_tphi_dth;
            gamma[2][0][3] = val;
            gamma[2][3][0] = val;
        }

        // ── Γ^φ_αβ  (μ = 3) ─────────────────────────────────────────────────
        // g^φν non-zero for ν = t (0) and ν = φ (3).

        // Γ^φ_tr = Γ^φ_rt  (α=0,β=1)
        {
            // ½ (g^φt ∂_r g_tt + g^φφ ∂_r g_φt)
            // = ½ (g^tφ ∂_r g_tt + g^φφ ∂_r g_tφ)   [g^φt = g^tφ by symmetry]
            let val = 0.5 * (g_inv_tphi * dg_tt_dr + g_inv_phph * dg_tphi_dr);
            gamma[3][0][1] = val;
            gamma[3][1][0] = val;
        }

        // Γ^φ_tθ = Γ^φ_θt  (α=0,β=2)
        {
            let val = 0.5 * (g_inv_tphi * dg_tt_dth + g_inv_phph * dg_tphi_dth);
            gamma[3][0][2] = val;
            gamma[3][2][0] = val;
        }

        // Γ^φ_rφ = Γ^φ_φr  (α=1,β=3)
        {
            // ½ [g^φt(∂_r g_tφ + ∂_φ g_tr − ∂_t g_rφ) + g^φφ(∂_r g_φφ + ∂_φ g_φr − ∂_φ g_rφ)]
            // = ½ (g^tφ ∂_r g_tφ + g^φφ ∂_r g_φφ)
            let val = 0.5 * (g_inv_tphi * dg_tphi_dr + g_inv_phph * dg_phph_dr);
            gamma[3][1][3] = val;
            gamma[3][3][1] = val;
        }

        // Γ^φ_θφ = Γ^φ_φθ  (α=2,β=3)
        {
            let val = 0.5 * (g_inv_tphi * dg_tphi_dth + g_inv_phph * dg_phph_dth);
            gamma[3][2][3] = val;
            gamma[3][3][2] = val;
        }

        // Γ^φ_tt  (α=0,β=0)
        {
            // ½ [g^φt ∂_t g_tt + g^φt ∂_t g_tt − g^φν ∂_ν g_tt]
            // = ½ [−g^φt ∂_t g_tt] ... wait, let's be careful.
            // Γ^μ_αβ = ½ g^μν (∂_α g_νβ + ∂_β g_να − ∂_ν g_αβ)
            // Γ^φ_tt = ½ g^φν (∂_t g_νt + ∂_t g_νt − ∂_ν g_tt)
            //        = ½ g^φν (2·0 − ∂_ν g_tt)   [∂_t=0]
            //        = −½ (g^φt ∂_t g_tt + g^φφ ∂_φ g_tt)  -- those vanish too
            //        = −½ g^φr ∂_r g_tt − ½ g^φθ ∂_θ g_tt
            // But g^φr = g^φθ = 0.  Hmm, we're missing contributions from ν=t and ν=φ
            // which have zero partial derivatives.
            // Actually: sum over all ν: g^φν = {g^φt (ν=0), g^φφ (ν=3)} only.
            // ∂_ν g_tt for ν=t: 0, ν=φ: 0 → zero.
            // So Γ^φ_tt = 0.  But that's wrong for Kerr; let me re-examine.
            //
            // Γ^φ_tt = ½ g^φν (∂_t g_νt + ∂_t g_νt − ∂_ν g_tt)
            //        = ½ Σ_ν g^φν (−∂_ν g_tt)
            //        = −½ [g^φt ∂_t g_tt + g^φr ∂_r g_tt + g^φθ ∂_θ g_tt + g^φφ ∂_φ g_tt]
            //
            // g^φt = g_inv_tphi, g^φr = 0, g^φθ = 0, g^φφ = g_inv_phph
            // ∂_t = 0, ∂_φ = 0 → contributions vanish.
            // So Γ^φ_tt = 0.  This is actually correct for Boyer-Lindquist!
            // (The metric is stationary and axisymmetric, no ∂_t or ∂_φ terms.)
            gamma[3][0][0] = 0.0; // explicitly zero
        }

        // Γ^φ_rr  (α=1,β=1) — also zero (g^φr=0)
        // Γ^φ_θθ  (α=2,β=2) — also zero
        // These remain 0.0 from initialization.

        // Suppress the no-longer-needed intermediate
        let _ = big_q;
        let _ = dg_phph_dr_clean;
        let _ = a2_cos2;
        let _ = g_inv_tt; // used above

        gamma
    }

    /// Outer event horizon: r₊ = M + √(M² − a²).
    fn event_horizon(&self) -> Option<f64> {
        let m = self.mass;
        let a = self.spin;
        let discriminant = m * m - a * a;
        if discriminant < 0.0 {
            None
        } else {
            Some(m + discriminant.sqrt())
        }
    }

    /// Prograde Keplerian Ω = √M / (r^{3/2} + a√M) in the equatorial plane.
    fn orbital_four_velocity(&self, pos: &SpacetimePoint) -> Option<FourVelocity> {
        let r = pos[1];
        let m = self.mass;
        let a = self.spin;
        if r <= 0.0 {
            return None;
        }
        let sqrt_m = m.sqrt();
        let omega = sqrt_m / (r.powf(1.5) + a * sqrt_m);
        circular_orbit_velocity(self, pos, omega)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metric::Metric;
    use crate::schwarzschild::Schwarzschild;
    use crate::{GeodesicState, RK4Integrator};
    use std::f64::consts::PI;

    /// Tolerance for exact-formula comparisons.
    const EPS: f64 = 1e-12;

    // ── helper: build a SpacetimePoint ───────────────────────────────────────
    fn pt(t: f64, r: f64, theta: f64, phi: f64) -> SpacetimePoint {
        SpacetimePoint::new(t, r, theta, phi)
    }

    // ── helper: numerical Christoffel (duplicate of trait default, for testing)
    fn numerical_christoffel_ref(metric: &Kerr, pos: &SpacetimePoint) -> ChristoffelSymbols {
        let h = 1e-5;
        let mut gamma = [[[0.0f64; 4]; 4]; 4];
        let mut dg = [[[0.0f64; 4]; 4]; 4];

        for sigma in 0..4 {
            let mut pos_plus = *pos;
            let mut pos_minus = *pos;
            pos_plus[sigma] += h;
            pos_minus[sigma] -= h;
            let g_plus = metric.metric_tensor(&pos_plus);
            let g_minus = metric.metric_tensor(&pos_minus);
            for mu in 0..4 {
                for nu in 0..4 {
                    dg[sigma][mu][nu] = (g_plus[(mu, nu)] - g_minus[(mu, nu)]) / (2.0 * h);
                }
            }
        }

        let g_inv = metric.inverse_metric(pos);
        for mu in 0..4 {
            for alpha in 0..4 {
                for beta in 0..4 {
                    let mut sum = 0.0;
                    for s in 0..4 {
                        sum += g_inv[(mu, s)]
                            * (dg[alpha][s][beta] + dg[beta][s][alpha] - dg[s][alpha][beta]);
                    }
                    gamma[mu][alpha][beta] = 0.5 * sum;
                }
            }
        }
        gamma
    }

    // ── test_kerr_reduces_to_schwarzschild_when_a_is_zero ────────────────────
    #[test]
    fn test_kerr_reduces_to_schwarzschild_when_a_is_zero() {
        let kerr = Kerr::new(1.0, 0.0);
        let schw = Schwarzschild::new(1.0);

        let positions = [
            pt(0.0, 10.0, PI / 3.0, 0.0),
            pt(0.0, 6.0, PI / 2.0, 1.0),
            pt(1.0, 20.0, PI / 4.0, 2.5),
            pt(0.0, 5.0, 2.0, 0.5),
        ];

        for pos in &positions {
            let gk = kerr.metric_tensor(pos);
            let gs = schw.metric_tensor(pos);

            for i in 0..4 {
                for j in 0..4 {
                    assert!(
                        (gk[(i, j)] - gs[(i, j)]).abs() < EPS,
                        "Kerr(a=0) vs Schwarzschild at pos={:?}: g[{}][{}] = {} vs {}",
                        pos,
                        i,
                        j,
                        gk[(i, j)],
                        gs[(i, j)]
                    );
                }
            }
        }
    }

    // ── test_kerr_horizon ─────────────────────────────────────────────────────
    #[test]
    fn test_kerr_horizon() {
        // a = 0.5M: r₊ = 1 + √(1 − 0.25) = 1 + √0.75
        let kerr = Kerr::new(1.0, 0.5);
        let expected = 1.0 + 0.75_f64.sqrt();
        let got = kerr.event_horizon().expect("horizon must exist");
        assert!(
            (got - expected).abs() < EPS,
            "horizon: got {got}, expected {expected}"
        );
    }

    // ── test_kerr_christoffel_matches_numerical ───────────────────────────────
    #[test]
    fn test_kerr_christoffel_matches_numerical() {
        let kerr = Kerr::new(1.0, 0.7);
        let pos = pt(0.0, 10.0, PI / 3.0, 0.7);

        let analytical = kerr.christoffel(&pos);
        let numerical = numerical_christoffel_ref(&kerr, &pos);

        for mu in 0..4 {
            for alpha in 0..4 {
                for beta in 0..4 {
                    let diff = (analytical[mu][alpha][beta] - numerical[mu][alpha][beta]).abs();
                    assert!(
                        diff < 1e-4,
                        "Γ^{mu}_{alpha}{beta}: analytical={} numerical={} diff={}",
                        analytical[mu][alpha][beta],
                        numerical[mu][alpha][beta],
                        diff
                    );
                }
            }
        }
    }

    // ── test_kerr_circular_orbit_equatorial ───────────────────────────────────
    #[test]
    fn test_kerr_circular_orbit_equatorial() {
        let kerr = Kerr::new(1.0, 0.5);
        let r0: f64 = 10.0;
        let m = kerr.mass;
        let a = kerr.spin;

        // Prograde circular orbit angular frequency in Kerr:
        // Ω = M^(1/2) / (r^(3/2) + a M^(1/2))
        let omega = m.sqrt() / (r0.powf(1.5) + a * m.sqrt());

        let pos = SpacetimePoint::new(0.0, r0, PI / 2.0, 0.0);
        // 4-velocity: dt/dλ = 1, dr/dλ = 0, dθ/dλ = 0, dφ/dλ = Ω
        let vel = SpacetimePoint::new(1.0, 0.0, 0.0, omega);

        let mut state = GeodesicState::new(pos, vel);
        let integrator = RK4Integrator::new(0.05);

        // One orbital period: T = 2π / Ω
        let period = 2.0 * PI / omega;
        let steps = (period / integrator.step_size) as usize;

        for _ in 0..steps {
            integrator.step(&kerr, &mut state);
        }

        assert!(
            (state.position[1] - r0).abs() < 0.5,
            "Circular orbit radius drifted: {} -> {}",
            r0,
            state.position[1]
        );
    }

    // ── sanity: metric signature and symmetry ─────────────────────────────────
    #[test]
    fn test_kerr_metric_signature_and_symmetry() {
        let kerr = Kerr::new(1.0, 0.9);
        let pos = pt(0.0, 10.0, PI / 2.0, 0.0);
        let g = kerr.metric_tensor(&pos);

        assert!(g[(0, 0)] < 0.0, "g_tt must be negative");
        assert!(g[(1, 1)] > 0.0, "g_rr must be positive");
        assert!(g[(2, 2)] > 0.0, "g_θθ must be positive");
        assert!(g[(3, 3)] > 0.0, "g_φφ must be positive");

        // Off-diagonal symmetry
        assert!((g[(0, 3)] - g[(3, 0)]).abs() < EPS, "g_tφ must be symmetric");

        // Non-zero off-diagonal for spinning BH
        assert!(g[(0, 3)].abs() > 1e-10, "g_tφ should be non-zero");
    }
}
