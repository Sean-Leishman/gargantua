use crate::curved::disk::AccretionDisk;
use crate::curved::outcome::RayOutcome;
use crate::curved::ray::GeodesicRay;
use gr_core::{Metric, RK45Integrator, SpacetimePoint, StepResult};
use nalgebra::Vector3;

pub fn trace_ray<M: Metric>(
    metric: &M,
    ray: &mut GeodesicRay,
    integrator: &RK45Integrator,
    max_steps: usize,
) -> RayOutcome {
    let mut h = integrator.initial_step;
    for _ in 0..max_steps {
        match integrator.step(metric, &mut ray.state, &mut h) {
            StepResult::Horizon => return RayOutcome::Horizon,
            StepResult::Escaped => return RayOutcome::Escaped { final_direction: spatial_dir(&ray.state.velocity) },
            StepResult::Singular => return RayOutcome::Horizon,
            StepResult::Continue => {}
        }
    }
    RayOutcome::MaxSteps
}

/// Trace a photon through a volumetric accretion disk, accumulating emission
/// with gravitational redshift and orbital Doppler along the path.
///
/// Energy ratio g = E_obs / E_emit is computed from p·u at each end:
/// the observer is taken as a static (Killing-aligned) observer at `observer`,
/// and the emitter as a prograde Keplerian circular orbiter when the metric
/// supplies one (otherwise also static — gravitational shift only). Specific
/// intensity transforms as I_obs = I_emit · g⁴ (Liouville's theorem).
pub fn trace_ray_with_disk<M: Metric>(
    metric: &M,
    ray: &mut GeodesicRay,
    disk: &AccretionDisk,
    observer: &SpacetimePoint,
    integrator: &RK45Integrator,
    max_steps: usize,
) -> RayOutcome {
    let mut h = integrator.initial_step;
    let mut intensity = 0.0_f64;
    let mut transmission = 1.0_f64;
    let absorption_coeff = 0.1_f64;

    let g_tt_obs = -metric.metric_tensor(observer)[(0, 0)];
    let sqrt_neg_gtt_obs = g_tt_obs.max(0.0).sqrt();

    for _ in 0..max_steps {
        let pos_before = ray.state.position;
        let vel_before = ray.state.velocity;
        let lambda_before = ray.state.lambda;
        let result = integrator.step(metric, &mut ray.state, &mut h);
        let dlambda = (ray.state.lambda - lambda_before).max(0.0);

        if dlambda > 0.0 {
            let mid = SpacetimePoint::new(
                0.5 * (pos_before[0] + ray.state.position[0]),
                0.5 * (pos_before[1] + ray.state.position[1]),
                0.5 * (pos_before[2] + ray.state.position[2]),
                0.5 * (pos_before[3] + ray.state.position[3]),
            );
            let rho = disk.density(&mid);
            if rho > 0.0 {
                let j = disk.emission(&mid);
                let g_factor = redshift_factor(
                    metric,
                    &mid,
                    &(0.5 * (vel_before + ray.state.velocity)),
                    sqrt_neg_gtt_obs,
                );
                let redshifted = j * g_factor.powi(4);
                intensity += redshifted * dlambda * transmission;
                transmission *= (-rho * dlambda * absorption_coeff).exp();
                if transmission < 1e-3 {
                    return RayOutcome::Disk { intensity, color_temp: disk.temperature_0 };
                }
            }
        }

        match result {
            StepResult::Horizon | StepResult::Singular => {
                return if intensity > 0.0 {
                    RayOutcome::Disk { intensity, color_temp: disk.temperature_0 }
                } else {
                    RayOutcome::Horizon
                };
            }
            StepResult::Escaped => {
                return if intensity > 0.0 {
                    RayOutcome::Disk { intensity, color_temp: disk.temperature_0 }
                } else {
                    RayOutcome::Escaped { final_direction: spatial_dir(&ray.state.velocity) }
                };
            }
            StepResult::Continue => {}
        }
    }

    if intensity > 0.0 {
        RayOutcome::Disk { intensity, color_temp: disk.temperature_0 }
    } else {
        RayOutcome::MaxSteps
    }
}

/// E_obs / E_emit, where E = -p·u for the photon 4-momentum p and observer
/// 4-velocity u. Observer is static (Killing-aligned) at the camera; emitter
/// uses the metric's prograde circular orbit if available, otherwise also
/// static (gravitational redshift only).
fn redshift_factor<M: Metric>(
    metric: &M,
    pos: &SpacetimePoint,
    p: &nalgebra::Vector4<f64>,
    sqrt_neg_gtt_obs: f64,
) -> f64 {
    let g = metric.metric_tensor(pos);
    // p_t = g_tμ p^μ — Killing-conserved along the geodesic.
    let p_t = g[(0, 0)] * p[0] + g[(0, 3)] * p[3];
    let e_obs = (-p_t / sqrt_neg_gtt_obs).max(0.0);

    let e_emit = match metric.orbital_four_velocity(pos) {
        Some(u) => {
            // -p·u = -(g_tμ p^μ u^t + g_φμ p^μ u^φ) = -(p_t u^t + p_φ u^φ)
            let p_phi = g[(3, 0)] * p[0] + g[(3, 3)] * p[3];
            (-(p_t * u[0] + p_phi * u[3])).max(0.0)
        }
        None => {
            // Static observer at emit point.
            let g_tt_emit = -g[(0, 0)];
            if g_tt_emit > 0.0 { -p_t / g_tt_emit.sqrt() } else { 0.0 }.max(0.0)
        }
    };

    if e_emit > 0.0 { e_obs / e_emit } else { 0.0 }
}

fn spatial_dir(v: &nalgebra::Vector4<f64>) -> Vector3<f64> {
    let s = Vector3::new(v[1], v[2], v[3]);
    let n = s.norm();
    if n > 0.0 { s / n } else { Vector3::new(1.0, 0.0, 0.0) }
}
