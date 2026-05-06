//! Light subpath generation for BDPT

use super::vertex::PathVertex;
use super::Path;
use crate::core::{Color, Hittable, Interval, Onb, Ray, Vec3};
use crate::scene::LightList;
use rand::Rng;
use std::f64::consts::PI;

/// Generate a light subpath starting from a light source
///
/// 1. Sample a light uniformly from the light list
/// 2. Sample a point on the light surface
/// 3. Sample an emission direction (cosine-weighted)
/// 4. Trace through the scene storing vertices
pub fn generate_light_path<'a, S, R>(
    scene: &'a S,
    lights: &LightList,
    max_depth: usize,
    rng: &mut R,
) -> Path<'a>
where
    S: Hittable,
    R: Rng,
{
    let mut path = Path::with_capacity(max_depth + 1);

    if lights.is_empty() {
        return path;
    }

    // Sample a light and a point on its surface
    let Some((light, light_pdf)) = lights.sample_light(rng) else {
        return path;
    };

    let Some(surface_sample) = light.sample_surface() else {
        return path;
    };

    // Sample emission direction (cosine-weighted hemisphere)
    let onb = Onb::from_w(surface_sample.normal);
    let (dir, dir_pdf) = sample_cosine_hemisphere(&onb, rng);

    // Get emission from the light
    // We'll estimate it from a test hit
    let test_ray = Ray::new(surface_sample.point + surface_sample.normal * 0.001, -surface_sample.normal);
    let emission = if let Some(hit) = light.hit(&test_ray, Interval::new(0.0001, 10.0)) {
        hit.material.emitted(hit.uv.0, hit.uv.1, hit.point)
    } else {
        Color::WHITE * 10.0 // Fallback
    };

    // Create light vertex
    let light_vertex = PathVertex::light(
        surface_sample.point,
        surface_sample.normal,
        emission,
        surface_sample.pdf * light_pdf,
        dir_pdf,
    );
    path.push(light_vertex);

    // Trace from the light into the scene
    let mut current_ray = Ray::new(surface_sample.point, dir);
    let mut throughput = emission / (surface_sample.pdf * light_pdf * dir_pdf);

    // Account for cosine at the light
    let cos_light = surface_sample.normal.dot(&dir).abs();
    throughput = throughput * cos_light;

    let mut depth = 0;

    while depth < max_depth {
        let t_min = 0.001;

        if let Some(hit) = scene.hit(&current_ray, Interval::new(t_min, f64::INFINITY)) {
            let is_delta = hit.material.is_delta();
            let wi = -current_ray.direction.normalize();

            // Forward PDF for this vertex
            let pdf_fwd = if depth == 0 {
                // First bounce - use area * direction PDF converted to solid angle
                let cos_theta = hit.normal.dot(&wi).abs();
                let dist_sq = (hit.point - current_ray.origin).magnitude_squared();
                dir_pdf * dist_sq / cos_theta.max(1e-8)
            } else {
                1.0 // Placeholder
            };

            // Create surface vertex
            let vertex = PathVertex::surface(
                hit.point,
                hit.normal,
                hit.material,
                hit.uv,
                hit.front_face,
                wi,
                throughput,
                pdf_fwd,
                is_delta,
            );
            path.push(vertex);

            // Try to scatter
            if let Some(scatter) = hit.material.scatter_pdf(&current_ray, &hit) {
                // Russian Roulette after depth 3
                if depth >= 3 {
                    let q = scatter.attenuation.luminance().clamp(0.05, 0.95);
                    if rng.r#gen::<f64>() > q {
                        break;
                    }
                    throughput = throughput * scatter.attenuation / q;
                } else {
                    throughput = throughput * scatter.attenuation;
                }

                // Get scattered ray
                let scattered_ray = if scatter.is_specular {
                    scatter.specular_ray
                } else if let Some(ref pdf) = scatter.pdf {
                    let dir = pdf.generate();
                    Some(Ray::new(hit.point, dir))
                } else {
                    None
                };

                if let Some(scattered) = scattered_ray {
                    // Update outgoing direction
                    if let Some(v) = path.last_mut() {
                        v.wo = scattered.direction.normalize();

                        if !is_delta {
                            v.pdf_rev = hit.material.pdf(hit.point, hit.normal, v.wo, wi);
                        }
                    }

                    current_ray = scattered;
                    depth += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    path
}

/// Sample a direction from a cosine-weighted hemisphere
fn sample_cosine_hemisphere<R: Rng>(onb: &Onb, rng: &mut R) -> (Vec3, f64) {
    let r1: f64 = rng.r#gen();
    let r2: f64 = rng.r#gen();

    let cos_theta = (1.0 - r2).sqrt();
    let sin_theta = r2.sqrt();
    let phi = 2.0 * PI * r1;

    let x = phi.cos() * sin_theta;
    let y = phi.sin() * sin_theta;
    let z = cos_theta;

    let dir = onb.local(x, y, z).normalize();
    let pdf = cos_theta / PI;

    (dir, pdf)
}
