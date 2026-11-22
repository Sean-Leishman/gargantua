use crate::core::{Color, Hittable, Interval, Point3, Ray};
use crate::curved::disk::AccretionDisk;
use crate::curved::outcome::RayOutcome;
use crate::curved::ray::GeodesicRay;
use gr_core::{Metric, RK45Integrator, SpacetimePoint, StepResult};
use nalgebra::Vector3;

/// Spherical `(t, r, θ, φ)` (the GR coordinate convention) → cartesian
/// `Point3`. Drops the time component — the scene lives in a single
/// spatial slice.
#[inline]
fn spherical_to_cartesian(p: &SpacetimePoint) -> Point3 {
    let r = p[1];
    let theta = p[2];
    let phi = p[3];
    let st = theta.sin();
    Point3::new(r * st * phi.cos(), r * st * phi.sin(), r * theta.cos())
}

/// Cartesian `Point3` → spherical `SpacetimePoint` `(0, r, θ, φ)`.
/// `t` is set to 0 — only the spatial coords feed into stationary metrics.
#[inline]
fn cartesian_to_spherical(p: &Point3) -> SpacetimePoint {
    let r = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt().max(1e-10);
    let theta = (p.z / r).clamp(-1.0, 1.0).acos();
    let phi = p.y.atan2(p.x);
    SpacetimePoint::new(0.0, r, theta, phi)
}

/// Gravitational-redshift surface-brightness factor `g⁴` for a static
/// emitter at `pos_emit` observed by a static observer at `pos_obs`.
/// `g = √(g_tt_emit / g_tt_obs)`, then specific intensity transforms as
/// `I_obs = I_emit · g⁴` (Liouville's theorem on phase-space density).
///
/// For a scene point at smaller r than the camera this is < 1 (light
/// climbing out of the well loses energy → fainter, redder); at larger
/// r the photon falls toward the camera and gains energy → slight
/// blueshift / brightening.
#[inline]
fn redshift_g4_static<M: Metric>(
    metric: &M,
    pos_emit: &SpacetimePoint,
    g_tt_obs: f64,
) -> f64 {
    let g_tt_emit = -metric.metric_tensor(pos_emit)[(0, 0)];
    if g_tt_emit <= 0.0 || g_tt_obs <= 0.0 {
        return 0.0;
    }
    let g = (g_tt_emit / g_tt_obs).sqrt();
    let g2 = g * g;
    g2 * g2
}

/// The single geodesic march behind every public `trace_ray*` variant.
///
/// Integrates the null geodesic with RK45. Per step, optionally:
/// - tests the straight cartesian chord between consecutive positions
///   against a `Hittable` scene (exact in flat space, a good local
///   approximation when the step is small relative to the curvature
///   scale);
/// - accumulates volumetric disk emission along the segment, with
///   redshift/Doppler (`g⁴`) and absorption.
///
/// On a scene hit the shaded surface color is redshifted by `g⁴`
/// (static emitter), attenuated by the disk transmission accumulated in
/// front of it, and composed with the disk emission so far. The "scene
/// first" ordering slightly under-counts disk emission within the hit
/// step (≤ one step's dλ × emission) — well below the noise floor.
pub(crate) fn trace_core<M: Metric>(
    metric: &M,
    ray: &mut GeodesicRay,
    disk: Option<&AccretionDisk>,
    scene: Option<&dyn Hittable>,
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

        // 1. Scene chord test.
        if let Some(scene) = scene {
            let cart_before = spherical_to_cartesian(&pos_before);
            let cart_after = spherical_to_cartesian(&ray.state.position);
            let segment = cart_after - cart_before;
            if segment.norm() > 0.0 {
                let chord = Ray::new(cart_before, segment);
                if let Some(hit) = scene.hit(&chord, Interval::new(0.0, 1.0)) {
                    let scene_lin = shade_hit(&chord, &hit);
                    // Use the cartesian hit point (exact) to compute the
                    // emitter's stationary 4-velocity, then apply g⁴ to
                    // the shaded surface brightness.
                    let pos_emit = cartesian_to_spherical(&hit.point);
                    let g4 = redshift_g4_static(metric, &pos_emit, g_tt_obs);
                    // disk_color(0.0) is black, so this composes to the
                    // plain shaded color when no disk is present.
                    let disk_lin = crate::curved::renderer::disk_color(intensity);
                    return RayOutcome::Scene {
                        color: [
                            scene_lin[0] * g4 * transmission + disk_lin[0],
                            scene_lin[1] * g4 * transmission + disk_lin[1],
                            scene_lin[2] * g4 * transmission + disk_lin[2],
                        ],
                    };
                }
            }
        }

        // 2. Disk volume contribution along the segment.
        if let Some(disk) = disk {
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
        }

        match result {
            StepResult::Horizon | StepResult::Singular => {
                return disk_or(disk, intensity, RayOutcome::Horizon);
            }
            StepResult::Escaped => {
                return disk_or(disk, intensity, RayOutcome::Escaped {
                    final_direction: escape_direction(metric, &ray.state.position, &ray.state.velocity),
                });
            }
            StepResult::Continue => {}
        }
    }
    disk_or(disk, intensity, RayOutcome::MaxSteps)
}

/// Terminal outcome: accumulated disk emission wins over the bare outcome.
fn disk_or(disk: Option<&AccretionDisk>, intensity: f64, otherwise: RayOutcome) -> RayOutcome {
    match disk {
        Some(d) if intensity > 0.0 => {
            RayOutcome::Disk { intensity, color_temp: d.temperature_0 }
        }
        _ => otherwise,
    }
}

pub fn trace_ray<M: Metric>(
    metric: &M,
    ray: &mut GeodesicRay,
    integrator: &RK45Integrator,
    max_steps: usize,
) -> RayOutcome {
    let observer = ray.state.position;
    trace_core(metric, ray, None, None, &observer, integrator, max_steps)
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
    trace_core(metric, ray, Some(disk), None, observer, integrator, max_steps)
}

/// Trace a null geodesic against an arbitrary `Hittable` scene living in
/// the cartesian spatial slice.
///
/// Shading is intentionally cheap: the material's `scatter` is sampled
/// once for an albedo, the `emitted` term is added, and the result is
/// modulated by a `0.5 + 0.5·(N·-D)` facing-ratio so flat-shaded objects
/// read clearly. This is *not* a recursive path trace — caustics, GI,
/// and shadows aren't computed. The point is geometry through a curved
/// metric.
pub fn trace_ray_with_scene<M: Metric, S: Hittable>(
    metric: &M,
    ray: &mut GeodesicRay,
    scene: &S,
    observer: &SpacetimePoint,
    integrator: &RK45Integrator,
    max_steps: usize,
) -> RayOutcome {
    trace_core(metric, ray, None, Some(scene as &dyn Hittable), observer, integrator, max_steps)
}

/// Compose a curved-spacetime path through *both* a volumetric disk and
/// a `Hittable` scene. See `trace_core` for the per-step composition.
pub fn trace_ray_with_disk_and_scene<M: Metric, S: Hittable>(
    metric: &M,
    ray: &mut GeodesicRay,
    disk: &AccretionDisk,
    scene: &S,
    observer: &SpacetimePoint,
    integrator: &RK45Integrator,
    max_steps: usize,
) -> RayOutcome {
    trace_core(metric, ray, Some(disk), Some(scene as &dyn Hittable), observer, integrator, max_steps)
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

/// Coordinate-basis photon velocity → cartesian unit direction, via the
/// local orthonormal frame at `pos` (physical components `√g_ii·k^i` on
/// `r̂/θ̂/φ̂`). The inverse of the projection in `GeodesicRay::from_camera`;
/// valid for diagonal metrics.
fn escape_direction<M: Metric>(
    metric: &M,
    pos: &SpacetimePoint,
    v: &nalgebra::Vector4<f64>,
) -> Vector3<f64> {
    let g = metric.metric_tensor(pos);
    let n_r = g[(1, 1)].max(0.0).sqrt() * v[1];
    let n_t = g[(2, 2)].max(0.0).sqrt() * v[2];
    let n_p = g[(3, 3)].max(0.0).sqrt() * v[3];

    let (st, ct) = (pos[2].sin(), pos[2].cos());
    let (sp, cp) = (pos[3].sin(), pos[3].cos());
    let r_hat = Vector3::new(st * cp, st * sp, ct);
    let theta_hat = Vector3::new(ct * cp, ct * sp, -st);
    let phi_hat = Vector3::new(-sp, cp, 0.0);

    let d = n_r * r_hat + n_t * theta_hat + n_p * phi_hat;
    let n = d.norm();
    if n > 0.0 { d / n } else { r_hat }
}

/// One-bounce-free shading: pull the material's albedo via `scatter` (or
/// fall back to its emission), then multiply by a facing ratio so curved
/// surfaces read.
fn shade_hit(ray: &Ray, hit: &crate::core::HitRecord) -> [f64; 3] {
    let albedo = match hit
        .material
        .scatter(ray, hit.point, hit.normal, hit.front_face)
    {
        Some((c, _)) => c,
        None => Color::BLACK,
    };
    let emission = hit.material.emitted(hit.uv.0, hit.uv.1, hit.point);

    // -ray.dir·normal, clamped — the standard "facing the camera" term.
    let nd = (-ray.direction.normalize()).dot(&hit.normal).max(0.0);
    let facing = 0.5 + 0.5 * nd;
    let lit = albedo * facing + emission;
    [lit.r, lit.g, lit.b]
}

#[cfg(test)]
mod tests {
    use super::*;
    use gr_core::Schwarzschild;

    #[test]
    fn escape_direction_inverts_camera_projection() {
        // Build a coordinate-basis velocity from a known cartesian direction
        // exactly the way GeodesicRay::from_camera does, then check
        // escape_direction recovers that direction.
        let metric = Schwarzschild::new(1.0);
        let pos = SpacetimePoint::new(0.0, 150.0, 1.1, 2.3);
        let dir = Vector3::new(0.3, -0.5, 0.8).normalize();

        let (st, ct) = (pos[2].sin(), pos[2].cos());
        let (sp, cp) = (pos[3].sin(), pos[3].cos());
        let r_hat = Vector3::new(st * cp, st * sp, ct);
        let theta_hat = Vector3::new(ct * cp, ct * sp, -st);
        let phi_hat = Vector3::new(-sp, cp, 0.0);

        let g = metric.metric_tensor(&pos);
        let v = nalgebra::Vector4::new(
            1.0,
            dir.dot(&r_hat) / g[(1, 1)].sqrt(),
            dir.dot(&theta_hat) / g[(2, 2)].sqrt(),
            dir.dot(&phi_hat) / g[(3, 3)].sqrt(),
        );

        let out = escape_direction(&metric, &pos, &v);
        assert!((out - dir).norm() < 1e-12, "got {out:?}, want {dir:?}");
    }
}
