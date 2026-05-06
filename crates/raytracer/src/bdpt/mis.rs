//! Multiple Importance Sampling (MIS) weight computation for BDPT

use super::Path;

/// Compute the MIS weight for a path contribution with s light vertices and t camera vertices
///
/// Uses the power heuristic with exponent 2:
/// w_{s,t} = p_{s,t}^2 / sum(p_k^2)
///
/// This is computed using relative probability ratios to avoid numerical issues.
pub fn compute_mis_weight(
    light_path: &Path,
    camera_path: &Path,
    s: usize,
    t: usize,
) -> f64 {
    if light_path.is_empty() && camera_path.is_empty() {
        return 0.0;
    }

    // For a single technique, weight is 1.0
    let n_light = light_path.len();
    let n_camera = camera_path.len();

    // Total path length is s + t
    let path_length = s + t;
    if path_length < 2 {
        return 1.0;
    }

    // Calculate relative probabilities using the balance heuristic first
    // Sum of all technique PDFs (we'll use power heuristic later)
    let mut sum_ri_sq = 0.0_f64;

    // We compute relative importance using the stored pdf_fwd and pdf_rev values
    // ri = p_i / p_{s,t} for each alternative technique i

    // Current technique contribution
    sum_ri_sq += 1.0; // ri = 1 for the current technique

    // Compute relative importance for shifting vertex from light to camera
    // and vice versa using stored PDFs
    let mut ri = 1.0_f64;

    // Walk from current connection point towards camera
    for i in (1..=s).rev() {
        if i > n_light || t + (s - i) > n_camera {
            continue;
        }

        let light_vertex = light_path.get(i - 1);
        let camera_vertex = if t + (s - i) > 0 {
            camera_path.get(t + (s - i) - 1)
        } else {
            None
        };

        if let (Some(lv), Some(cv)) = (light_vertex, camera_vertex) {
            if lv.pdf_fwd > 0.0 && cv.pdf_rev > 0.0 {
                ri *= cv.pdf_rev / lv.pdf_fwd;
                if !ri.is_finite() || ri < 1e-10 {
                    break;
                }
                sum_ri_sq += ri * ri;
            }
        }
    }

    // Reset and walk towards light
    ri = 1.0;
    for i in 1..=t {
        if i > n_camera || s + (t - i) > n_light {
            continue;
        }

        let camera_vertex = camera_path.get(i - 1);
        let light_vertex = if s + (t - i) > 0 {
            light_path.get(s + (t - i) - 1)
        } else {
            None
        };

        if let (Some(cv), Some(lv)) = (camera_vertex, light_vertex) {
            if cv.pdf_fwd > 0.0 && lv.pdf_rev > 0.0 {
                ri *= lv.pdf_rev / cv.pdf_fwd;
                if !ri.is_finite() || ri < 1e-10 {
                    break;
                }
                sum_ri_sq += ri * ri;
            }
        }
    }

    // Power heuristic weight
    if sum_ri_sq < 1e-10 {
        return 0.0;
    }

    1.0 / sum_ri_sq
}

/// Simple balance heuristic MIS weight (beta=1)
pub fn balance_heuristic(pdf1: f64, pdf2: f64) -> f64 {
    if pdf1 + pdf2 < 1e-10 {
        return 0.0;
    }
    pdf1 / (pdf1 + pdf2)
}

/// Power heuristic MIS weight (beta=2)
pub fn power_heuristic(pdf1: f64, pdf2: f64) -> f64 {
    let p1_sq = pdf1 * pdf1;
    let p2_sq = pdf2 * pdf2;
    if p1_sq + p2_sq < 1e-10 {
        return 0.0;
    }
    p1_sq / (p1_sq + p2_sq)
}
