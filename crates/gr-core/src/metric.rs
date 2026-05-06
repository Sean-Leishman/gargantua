use nalgebra::{Matrix4, Vector4};

pub type FourVelocity = Vector4<f64>;
pub type SpacetimePoint = Vector4<f64>;
pub type MetricTensor = Matrix4<f64>;

/// Christoffel symbols Γ^μ_αβ
/// Indexed as christoffel[μ][α][β]
pub type ChristoffelSymbols = [[[f64; 4]; 4]; 4];

pub trait Metric: Send + Sync {
    fn metric_tensor(&self, pos: &SpacetimePoint) -> MetricTensor;

    /// Compute the inverse metric tensor g^μν at a given spacetime point
    fn inverse_metric(&self, pos: &SpacetimePoint) -> MetricTensor {
        self.metric_tensor(pos)
            .try_inverse()
            .expect("Metric tensor should be invertible")
    }

    /// Compute Christoffel symbols Γ^μ_αβ at a given point
    ///
    /// Default implementation uses numerical differentiation.
    /// Override for analytical expressions when available.
    fn christoffel(&self, pos: &SpacetimePoint) -> ChristoffelSymbols {
        numerical_christoffel(self, pos)
    }

    /// Event horizon radius (if applicable)
    fn event_horizon(&self) -> Option<f64> {
        None
    }

    /// Check if a point is inside the event horizon
    fn is_inside_horizon(&self, pos: &SpacetimePoint) -> bool {
        if let Some(r_h) = self.event_horizon() {
            pos[1] < r_h
        } else {
            false
        }
    }

    /// 4-velocity of a prograde circular Keplerian orbit in the equatorial plane.
    ///
    /// Returned in coordinate basis (u^t, 0, 0, u^φ); used by the disk renderer
    /// to compute Doppler boost on emitted light. Returns `None` if the metric
    /// has no sensible circular-orbit definition or the orbit is unphysical at
    /// `pos[1]` (e.g. inside the photon sphere).
    fn orbital_four_velocity(&self, _pos: &SpacetimePoint) -> Option<FourVelocity> {
        None
    }
}

/// Build a circular-orbit 4-velocity from an angular velocity Ω, normalising
/// against the metric so that u·u = -1. Returns None if the orbit is spacelike
/// (denominator non-positive — i.e. inside the photon sphere or extreme spin).
pub fn circular_orbit_velocity<M: Metric + ?Sized>(
    metric: &M,
    pos: &SpacetimePoint,
    omega: f64,
) -> Option<FourVelocity> {
    let g = metric.metric_tensor(pos);
    let denom = -(g[(0, 0)] + 2.0 * g[(0, 3)] * omega + g[(3, 3)] * omega * omega);
    if denom <= 0.0 || !denom.is_finite() {
        return None;
    }
    let u_t = 1.0 / denom.sqrt();
    Some(FourVelocity::new(u_t, 0.0, 0.0, omega * u_t))
}

/// Compute Christoffel symbols numerically via finite differences
fn numerical_christoffel<M: Metric + ?Sized>(
    metric: &M,
    pos: &SpacetimePoint,
) -> ChristoffelSymbols {
    let h = 1e-6;
    let mut gamma = [[[0.0; 4]; 4]; 4];

    // Compute metric derivatives d_sigma g_uν
    let mut dg = [[[0.0; 4]; 4]; 4]; // dg[sigma][mu][ν] = d_sigma g_uν

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

    // Get inverse metric
    let g_inv = metric.inverse_metric(pos);

    // Γ^μ_αβ = (1/2) g^μσ (∂_α g_σβ + ∂_β g_σα - ∂_σ g_αβ)
    for mu in 0..4 {
        for alpha in 0..4 {
            for beta in 0..4 {
                let mut sum = 0.0;
                for sigma in 0..4 {
                    sum += g_inv[(mu, sigma)]
                        * (dg[alpha][sigma][beta] + dg[beta][sigma][alpha]
                            - dg[sigma][alpha][beta]);
                }
                gamma[mu][alpha][beta] = 0.5 * sum;
            }
        }
    }

    gamma
}
