//! Camera subpath generation for BDPT

use super::vertex::PathVertex;
use super::Path;
use crate::camera::Camera;
use crate::core::{Color, Hittable, Interval, Ray};
use rand::Rng;

/// Generate a camera subpath starting from the camera
///
/// Traces a path from the camera into the scene, storing vertices along the way.
/// Uses Russian Roulette termination after depth 3.
pub fn generate_camera_path<'a, S, C, R>(
    scene: &'a S,
    camera: &C,
    u: f64,
    v: f64,
    max_depth: usize,
    rng: &mut R,
) -> Path<'a>
where
    S: Hittable,
    C: Camera,
    R: Rng,
{
    let mut path = Path::with_capacity(max_depth + 1);

    // Get initial ray from camera
    let ray = camera.get_ray(u, v);

    // Add camera vertex
    let camera_vertex = PathVertex::camera(ray.origin, ray.direction);
    path.push(camera_vertex);

    // Trace through scene
    let mut current_ray = ray;
    let mut throughput = Color::WHITE;
    let mut depth = 0;

    while depth < max_depth {
        let t_min = 0.001;

        if let Some(hit) = scene.hit(&current_ray, Interval::new(t_min, f64::INFINITY)) {
            // Get material properties
            let is_delta = hit.material.is_delta();
            let wi = -current_ray.direction.normalize();

            // Compute forward PDF for reaching this vertex
            // For camera paths, we use the material's scattering PDF
            let pdf_fwd = if depth == 0 {
                1.0 // First vertex after camera
            } else {
                // Use solid angle PDF
                1.0 // Will be refined below
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

                // Get the scattered ray
                let scattered_ray = if scatter.is_specular {
                    scatter.specular_ray
                } else if let Some(ref pdf) = scatter.pdf {
                    let dir = pdf.generate();
                    Some(Ray::new(hit.point, dir))
                } else {
                    None
                };

                if let Some(scattered) = scattered_ray {
                    // Update outgoing direction on current vertex
                    if let Some(v) = path.last_mut() {
                        v.wo = scattered.direction.normalize();

                        // Compute reverse PDF
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
                // No scattering - this could be a light source
                break;
            }
        } else {
            // Ray escaped to background
            break;
        }
    }

    path
}
