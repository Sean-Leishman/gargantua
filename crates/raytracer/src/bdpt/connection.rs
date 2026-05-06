//! Path connection strategies for BDPT

use super::geometry::{geometry_term, visible};
use super::mis::{compute_mis_weight, power_heuristic};
use super::Path;
use crate::core::{Color, Hittable, Point3, Vec3};
use crate::scene::LightList;

/// Connect light and camera subpaths and compute the combined contribution
///
/// Evaluates all valid (s, t) combinations where:
/// - s = number of vertices used from light path
/// - t = number of vertices used from camera path
///
/// Strategies:
/// - s=0: Camera path hits light directly (unidirectional path tracing contribution)
/// - s=1: NEE - sample point on light, connect to camera vertex
/// - s>=2, t>=1: Connect interior vertices from both paths
pub fn connect_paths<S: Hittable>(
    scene: &S,
    light_path: &Path,
    camera_path: &Path,
    _lights: &LightList,
) -> Color {
    let mut total = Color::BLACK;

    let n_light = light_path.len();
    let n_camera = camera_path.len();

    // Strategy s=0: Camera path directly hits a light source
    // This is the unidirectional PT contribution
    for t in 1..=n_camera {
        let camera_vertex = &camera_path[t - 1];

        // Check if this camera vertex hit a light
        let emitted = camera_vertex.emitted();
        if emitted.luminance() > 0.0 && !camera_vertex.is_camera() {
            // Weight for direct hits
            let mis_weight = compute_mis_weight(light_path, camera_path, 0, t);
            total += camera_vertex.throughput * emitted * mis_weight;
        }
    }

    // Strategy s>=1, t>=1: Connect subpaths
    for s in 1..=n_light {
        for t in 1..=n_camera {
            let light_vertex = &light_path[s - 1];
            let camera_vertex = &camera_path[t - 1];

            // Skip if camera vertex is the actual camera (t=1 is camera itself)
            if camera_vertex.is_camera() {
                continue;
            }

            // Skip if either vertex is delta (cannot connect through specular)
            if !light_vertex.is_connectible() || !camera_vertex.is_connectible() {
                continue;
            }

            // Visibility check
            if !visible(scene, light_vertex.point, camera_vertex.point) {
                continue;
            }

            // Compute connection contribution
            let g = geometry_term(
                light_vertex.point,
                light_vertex.normal,
                camera_vertex.point,
                camera_vertex.normal,
            );

            if g < 1e-10 {
                continue;
            }

            // Connection direction
            let d = camera_vertex.point - light_vertex.point;
            let dist = d.magnitude();
            if dist < 1e-8 {
                continue;
            }
            let connection_dir = d / dist;

            // Evaluate BSDF at light vertex (outgoing towards camera)
            let f_light = evaluate_bsdf_at_vertex(light_vertex, connection_dir);

            // Evaluate BSDF at camera vertex (incoming from light)
            let f_camera = evaluate_bsdf_at_vertex(camera_vertex, -connection_dir);

            // Combined throughput
            let contribution = light_vertex.throughput * f_light * g * f_camera * camera_vertex.throughput;

            if contribution.luminance() > 0.0 {
                // MIS weight
                let mis_weight = compute_connection_mis_weight(light_path, camera_path, s, t, g);
                total += contribution * mis_weight;
            }
        }
    }

    total
}

/// Evaluate BSDF at a vertex for a given connection direction
fn evaluate_bsdf_at_vertex(vertex: &super::PathVertex, connection_dir: Vec3) -> Color {
    if vertex.is_light() {
        // Light source - just emit in that direction
        // Weight by cosine
        let cos_theta = vertex.normal.dot(&connection_dir).abs();
        if cos_theta > 0.0 {
            Color::WHITE * cos_theta
        } else {
            Color::BLACK
        }
    } else if let Some(ref mat) = vertex.material {
        // Surface vertex - evaluate BSDF
        mat.bsdf(vertex.point, vertex.normal, vertex.wi, connection_dir)
    } else {
        // Camera vertex - no BSDF
        Color::WHITE
    }
}

/// Compute MIS weight for a specific connection
fn compute_connection_mis_weight(
    light_path: &Path,
    camera_path: &Path,
    s: usize,
    t: usize,
    _g: f64,
) -> f64 {
    // For now, use a simplified MIS based on path lengths
    // This could be improved with proper PDF computation

    if s == 0 {
        // Direct light hit - full weight
        return 1.0;
    }

    if light_path.is_empty() || camera_path.is_empty() {
        return 1.0;
    }

    // Get PDFs from vertices
    let light_vertex = light_path.get(s - 1);
    let camera_vertex = camera_path.get(t - 1);

    if let (Some(lv), Some(cv)) = (light_vertex, camera_vertex) {
        // Simple heuristic based on PDF products
        let pdf_light = lv.pdf_fwd.max(1e-10);
        let pdf_camera = cv.pdf_fwd.max(1e-10);

        // Balance heuristic
        let total_pdf = pdf_light + pdf_camera;
        if total_pdf > 0.0 {
            return power_heuristic(pdf_light, pdf_camera);
        }
    }

    // Fallback to uniform weighting
    let n_techniques = (s + 1).min(light_path.len() + 1) as f64;
    1.0 / n_techniques
}

/// Connect a light vertex directly to the camera (for light tracing)
pub fn connect_to_camera<S: Hittable>(
    scene: &S,
    light_vertex: &super::PathVertex,
    camera_pos: Point3,
) -> Option<(Color, f64, f64)> {
    // Check visibility
    if !visible(scene, light_vertex.point, camera_pos) {
        return None;
    }

    // Direction from light to camera
    let d = camera_pos - light_vertex.point;
    let dist = d.magnitude();
    if dist < 1e-8 {
        return None;
    }
    let dir = d / dist;

    // Evaluate BSDF at light vertex
    let f = evaluate_bsdf_at_vertex(light_vertex, dir);

    // Cosine at light vertex
    let cos_light = light_vertex.normal.dot(&dir).abs();

    // Contribution
    let contribution = light_vertex.throughput * f * cos_light / (dist * dist);

    // Return contribution and pixel coordinates (u, v) would need camera info
    // For now, return the contribution
    Some((contribution, 0.0, 0.0))
}
