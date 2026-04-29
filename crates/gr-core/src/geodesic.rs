//! Geodesic equation integration
//!
//! Provides numerical integration of the geodesic equation:
//! d²x^μ/dλ² + Γ^μ_αβ (dx^α/dλ)(dx^β/dλ) = 0
//!
//! This module implements Runge-Kutta methods for evolving both
//! position and velocity along geodesics.

use crate::metric::{ChristoffelSymbols, FourVelocity, Metric, SpacetimePoint};

/// State of a particle/photon on a geodesic
#[derive(Debug, Clone, Copy)]
pub struct GeodesicState {
    /// Position in spacetime (t, r, θ, φ)
    pub position: SpacetimePoint,
    /// 4-velocity (dt/dλ, dr/dλ, dθ/dλ, dφ/dλ)
    pub velocity: FourVelocity,
    /// Affine parameter
    pub lambda: f64,
}

impl GeodesicState {
    pub fn new(position: SpacetimePoint, velocity: FourVelocity) -> Self {
        Self {
            position,
            velocity,
            lambda: 0.0,
        }
    }
}

/// Result of a geodesic integration step
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    /// Normal step completed
    Continue,
    /// Crossed event horizon
    Horizon,
    /// Escaped to infinity
    Escaped,
    /// Hit a coordinate singularity or numerical issue
    Singular,
}

/// Compute the geodesic acceleration d²x^μ/dλ²
///
/// From the geodesic equation:
/// d²x^μ/dλ² = -Γ^μ_αβ (dx^α/dλ)(dx^β/dλ)
fn geodesic_acceleration(gamma: &ChristoffelSymbols, vel: &FourVelocity) -> FourVelocity {
    let mut acc = FourVelocity::zeros();

    for mu in 0..4 {
        for alpha in 0..4 {
            for beta in 0..4 {
                acc[mu] -= gamma[mu][alpha][beta] * vel[alpha] * vel[beta];
            }
        }
    }

    acc
}

/// Fourth-order Runge-Kutta integrator for geodesics
pub struct RK4Integrator {
    /// Step size in affine parameter
    pub step_size: f64,
    /// Maximum radius (escape condition)
    pub max_radius: f64,
    /// Minimum radius buffer above horizon
    pub horizon_buffer: f64,
}

impl Default for RK4Integrator {
    fn default() -> Self {
        Self {
            step_size: 0.1,
            max_radius: 100.0,
            horizon_buffer: 0.01,
        }
    }
}

impl RK4Integrator {
    pub fn new(step_size: f64) -> Self {
        Self {
            step_size,
            ..Default::default()
        }
    }

    /// Perform one RK4 step
    pub fn step<M: Metric>(&self, metric: &M, state: &mut GeodesicState) -> StepResult {
        let h = self.step_size;
        let pos = state.position;
        let vel = state.velocity;

        // Check bounds before step
        if pos[1] > self.max_radius {
            return StepResult::Escaped;
        }

        if let Some(r_h) = metric.event_horizon() {
            if pos[1] < r_h + self.horizon_buffer {
                return StepResult::Horizon;
            }
        }

        if metric.is_inside_horizon(&pos) {
            return StepResult::Horizon;
        }

        // k1
        let gamma1 = metric.christoffel(&pos);
        let k1_pos = vel;
        let k1_vel = geodesic_acceleration(&gamma1, &vel);

        // k2
        let pos2 = pos + 0.5 * h * k1_pos;
        let vel2 = vel + 0.5 * h * k1_vel;
        if !is_valid_position(&pos2) {
            return StepResult::Singular;
        }
        let gamma2 = metric.christoffel(&pos2);
        let k2_pos = vel2;
        let k2_vel = geodesic_acceleration(&gamma2, &vel2);

        // k3
        let pos3 = pos + 0.5 * h * k2_pos;
        let vel3 = vel + 0.5 * h * k2_vel;
        if !is_valid_position(&pos3) {
            return StepResult::Singular;
        }
        let gamma3 = metric.christoffel(&pos3);
        let k3_pos = vel3;
        let k3_vel = geodesic_acceleration(&gamma3, &vel3);

        // k4
        let pos4 = pos + h * k3_pos;
        let vel4 = vel + h * k3_vel;
        if !is_valid_position(&pos4) {
            return StepResult::Singular;
        }
        let gamma4 = metric.christoffel(&pos4);
        let k4_pos = vel4;
        let k4_vel = geodesic_acceleration(&gamma4, &vel4);

        // Combine
        state.position = pos + (h / 6.0) * (k1_pos + 2.0 * k2_pos + 2.0 * k3_pos + k4_pos);
        state.velocity = vel + (h / 6.0) * (k1_vel + 2.0 * k2_vel + 2.0 * k3_vel + k4_vel);
        state.lambda += h;

        // Normalize theta to [0, π]
        normalize_spherical_coords(&mut state.position);

        StepResult::Continue
    }

    /// Integrate geodesic for a maximum number of steps
    pub fn integrate<M: Metric>(
        &self,
        metric: &M,
        state: &mut GeodesicState,
        max_steps: usize,
    ) -> StepResult {
        for _ in 0..max_steps {
            let result = self.step(metric, state);
            if result != StepResult::Continue {
                return result;
            }
        }
        StepResult::Continue
    }
}

/// Adaptive step size RK45 integrator (Dormand-Prince)
pub struct RK45Integrator {
    /// Initial step size
    pub initial_step: f64,
    /// Minimum step size
    pub min_step: f64,
    /// Maximum step size
    pub max_step: f64,
    /// Error tolerance
    pub tolerance: f64,
    /// Maximum radius
    pub max_radius: f64,
    /// Horizon buffer
    pub horizon_buffer: f64,
}

impl Default for RK45Integrator {
    fn default() -> Self {
        Self {
            initial_step: 0.1,
            min_step: 1e-8,
            max_step: 1.0,
            tolerance: 1e-6,
            max_radius: 100.0,
            horizon_buffer: 0.01,
        }
    }
}

impl RK45Integrator {
    /// Perform one adaptive RK45 step
    pub fn step<M: Metric>(
        &self,
        metric: &M,
        state: &mut GeodesicState,
        h: &mut f64,
    ) -> StepResult {
        let pos = state.position;
        let vel = state.velocity;

        // Check bounds
        if pos[1] > self.max_radius {
            return StepResult::Escaped;
        }

        if let Some(r_h) = metric.event_horizon() {
            if pos[1] < r_h + self.horizon_buffer {
                return StepResult::Horizon;
            }
        }

        if metric.is_inside_horizon(&pos) {
            return StepResult::Horizon;
        }

        // Dormand-Prince coefficients
        let a2 = 1.0 / 5.0;
        let a3 = 3.0 / 10.0;
        let a4 = 4.0 / 5.0;
        let a5 = 8.0 / 9.0;

        let b21 = 1.0 / 5.0;
        let b31 = 3.0 / 40.0;
        let b32 = 9.0 / 40.0;
        let b41 = 44.0 / 45.0;
        let b42 = -56.0 / 15.0;
        let b43 = 32.0 / 9.0;
        let b51 = 19372.0 / 6561.0;
        let b52 = -25360.0 / 2187.0;
        let b53 = 64448.0 / 6561.0;
        let b54 = -212.0 / 729.0;
        let b61 = 9017.0 / 3168.0;
        let b62 = -355.0 / 33.0;
        let b63 = 46732.0 / 5247.0;
        let b64 = 49.0 / 176.0;
        let b65 = -5103.0 / 18656.0;

        let c1 = 35.0 / 384.0;
        let c3 = 500.0 / 1113.0;
        let c4 = 125.0 / 192.0;
        let c5 = -2187.0 / 6784.0;
        let c6 = 11.0 / 84.0;

        let d1 = 5179.0 / 57600.0;
        let d3 = 7571.0 / 16695.0;
        let d4 = 393.0 / 640.0;
        let d5 = -92097.0 / 339200.0;
        let d6 = 187.0 / 2100.0;
        let d7 = 1.0 / 40.0;

        // k1
        let gamma1 = metric.christoffel(&pos);
        let k1_vel = geodesic_acceleration(&gamma1, &vel);

        // k2
        let pos2 = pos + *h * a2 * vel;
        let vel2 = vel + *h * b21 * k1_vel;
        if !is_valid_position(&pos2) {
            *h *= 0.5;
            return StepResult::Continue;
        }
        let gamma2 = metric.christoffel(&pos2);
        let k2_vel = geodesic_acceleration(&gamma2, &vel2);

        // k3
        let pos3 = pos + *h * a3 * vel;
        let vel3 = vel + *h * (b31 * k1_vel + b32 * k2_vel);
        if !is_valid_position(&pos3) {
            *h *= 0.5;
            return StepResult::Continue;
        }
        let gamma3 = metric.christoffel(&pos3);
        let k3_vel = geodesic_acceleration(&gamma3, &vel3);

        // k4
        let pos4 = pos + *h * a4 * vel;
        let vel4 = vel + *h * (b41 * k1_vel + b42 * k2_vel + b43 * k3_vel);
        if !is_valid_position(&pos4) {
            *h *= 0.5;
            return StepResult::Continue;
        }
        let gamma4 = metric.christoffel(&pos4);
        let k4_vel = geodesic_acceleration(&gamma4, &vel4);

        // k5
        let pos5 = pos + *h * a5 * vel;
        let vel5 = vel + *h * (b51 * k1_vel + b52 * k2_vel + b53 * k3_vel + b54 * k4_vel);
        if !is_valid_position(&pos5) {
            *h *= 0.5;
            return StepResult::Continue;
        }
        let gamma5 = metric.christoffel(&pos5);
        let k5_vel = geodesic_acceleration(&gamma5, &vel5);

        // k6
        let pos6 = pos + *h * vel;
        let vel6 =
            vel + *h * (b61 * k1_vel + b62 * k2_vel + b63 * k3_vel + b64 * k4_vel + b65 * k5_vel);
        if !is_valid_position(&pos6) {
            *h *= 0.5;
            return StepResult::Continue;
        }
        let gamma6 = metric.christoffel(&pos6);
        let k6_vel = geodesic_acceleration(&gamma6, &vel6);

        // 5th order solution
        let new_vel =
            vel + *h * (c1 * k1_vel + c3 * k3_vel + c4 * k4_vel + c5 * k5_vel + c6 * k6_vel);

        // 4th order solution for error estimate
        let k7_vel = geodesic_acceleration(&gamma6, &new_vel);
        let vel_err = vel
            + *h * (d1 * k1_vel
                + d3 * k3_vel
                + d4 * k4_vel
                + d5 * k5_vel
                + d6 * k6_vel
                + d7 * k7_vel);

        // Error estimate
        let err = (new_vel - vel_err).norm();

        if err < self.tolerance || *h <= self.min_step {
            // Accept step
            state.position = pos + *h * vel;
            state.velocity = new_vel;
            state.lambda += *h;
            normalize_spherical_coords(&mut state.position);

            // Adjust step size
            if err > 0.0 {
                *h = (*h * 0.9 * (self.tolerance / err).powf(0.2))
                    .clamp(self.min_step, self.max_step);
            }
        } else {
            // Reject step, reduce h
            *h = (*h * 0.9 * (self.tolerance / err).powf(0.25)).max(self.min_step);
        }

        StepResult::Continue
    }
}

/// Check if position is valid (no NaN, reasonable theta)
fn is_valid_position(pos: &SpacetimePoint) -> bool {
    pos.iter().all(|x| x.is_finite()) && pos[1] > 0.0
}

/// Normalize spherical coordinates
fn normalize_spherical_coords(pos: &mut SpacetimePoint) {
    use std::f64::consts::PI;

    // Keep theta in [0, π]
    if pos[2] < 0.0 {
        pos[2] = -pos[2];
        pos[3] += PI;
    }
    if pos[2] > PI {
        pos[2] = 2.0 * PI - pos[2];
        pos[3] += PI;
    }

    // Keep phi in [0, 2π)
    pos[3] = pos[3].rem_euclid(2.0 * PI);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schwarzschild::Schwarzschild;
    use std::f64::consts::PI;

    #[test]
    fn test_circular_orbit() {
        // Test a circular orbit at r = 6M (stable circular orbit for Schwarzschild)
        let metric = Schwarzschild::new(1.0);
        let r = 6.0;

        // For circular orbit: dφ/dt = sqrt(M/r³), so with dt/dλ = 1:
        let omega = ((1.0 / (r * r * r)) as f64).sqrt();
        let pos = SpacetimePoint::new(0.0, r, PI / 2.0, 0.0);
        let vel = FourVelocity::new(1.0, 0.0, 0.0, omega);

        let mut state = GeodesicState::new(pos, vel);
        let integrator = RK4Integrator::new(0.01);

        // Integrate for one period
        let period = 2.0 * PI / omega;
        let steps = (period / integrator.step_size) as usize;

        for _ in 0..steps {
            integrator.step(&metric, &mut state);
        }

        // Radius should remain approximately constant
        assert!(
            (state.position[1] - r).abs() < 0.1,
            "Radius drifted: {} -> {}",
            r,
            state.position[1]
        );
    }
}
