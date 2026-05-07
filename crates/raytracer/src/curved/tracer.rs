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

/// Trace a null geodesic against an arbitrary `Hittable` scene living in
/// the cartesian spatial slice.
///
/// Between consecutive integrator steps the photon's path is approximated
/// by a straight chord in cartesian space, and we test that chord against
/// the scene's BVH. This is exact in flat space and a good local
/// approximation when the integrator step is small relative to the scene
/// curvature scale — the user can tighten `max_steps` or the integrator's
/// tolerance if they need finer chords near a horizon.
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
    integrator: &RK45Integrator,
    max_steps: usize,
) -> RayOutcome {
    let mut h = integrator.initial_step;

    for _ in 0..max_steps {
        let pos_before = ray.state.position;
        let result = integrator.step(metric, &mut ray.state, &mut h);
        let pos_after = ray.state.position;

        // Test the chord [cart_before, cart_after] against the scene.
        let cart_before = spherical_to_cartesian(&pos_before);
        let cart_after = spherical_to_cartesian(&pos_after);
        let segment = cart_after - cart_before;
        let length = segment.norm();
        if length > 0.0 {
            // Direction is non-unit; t in `Hittable::hit` is in segment-length
            // units, so the valid range is [0, 1] for "anywhere on the chord".
            let chord = Ray::new(cart_before, segment);
            if let Some(hit) = scene.hit(&chord, Interval::new(0.0, 1.0)) {
                return RayOutcome::Scene { color: shade_hit(&chord, &hit) };
            }
        }

        match result {
            StepResult::Horizon | StepResult::Singular => return RayOutcome::Horizon,
            StepResult::Escaped => {
                return RayOutcome::Escaped { final_direction: spatial_dir(&ray.state.velocity) };
            }
            StepResult::Continue => {}
        }
    }
    RayOutcome::MaxSteps
}

/// Compose a curved-spacetime path through *both* a volumetric disk and
/// a `Hittable` scene.
///
/// Per integrator step we (1) test the chord against the scene first; on
/// hit, return `Scene { color }` with the disk emission accumulated *up
/// to* the hit baked in (`color = scene_shaded · transmission +
/// disk_color(intensity)`). Then (2) accumulate the segment's disk
/// contribution as in `trace_ray_with_disk`.
///
/// The "scene first" ordering means we slightly under-count disk
/// emission within the same step as a scene hit (by ≤ one step's
/// dlambda × emission). For the integrator's step sizes that's well
/// below the noise floor.
pub fn trace_ray_with_disk_and_scene<M: Metric, S: Hittable>(
    metric: &M,
    ray: &mut GeodesicRay,
    disk: &AccretionDisk,
    scene: &S,
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

        // 1. Scene chord test — if we hit, compose the scene color with
        //    the disk emission already accumulated *in front of* it.
        let cart_before = spherical_to_cartesian(&pos_before);
        let cart_after = spherical_to_cartesian(&ray.state.position);
        let segment = cart_after - cart_before;
        if segment.norm() > 0.0 {
            let chord = Ray::new(cart_before, segment);
            if let Some(hit) = scene.hit(&chord, Interval::new(0.0, 1.0)) {
                let scene_lin = shade_hit(&chord, &hit);
                let disk_lin = crate::curved::renderer::disk_color(intensity);
                return RayOutcome::Scene {
                    color: [
                        scene_lin[0] * transmission + disk_lin[0],
                        scene_lin[1] * transmission + disk_lin[1],
                        scene_lin[2] * transmission + disk_lin[2],
                    ],
                };
            }
        }

        // 2. Disk volume contribution along the segment (mirrors
        //    `trace_ray_with_disk`).
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

