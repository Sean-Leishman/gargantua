//! Geometry utilities for BDPT

use crate::core::{Hittable, Interval, Point3, Ray, Vec3};

/// Compute the geometry term G(x, y) between two points
///
/// G(x,y) = |cos(theta_x)| * |cos(theta_y)| / |x - y|^2
///
/// where theta_x is the angle between the connection direction and normal at x,
/// and theta_y is the angle between the connection direction and normal at y.
pub fn geometry_term(p1: Point3, n1: Vec3, p2: Point3, n2: Vec3) -> f64 {
    let d = p2 - p1;
    let dist_sq = d.magnitude_squared();

    if dist_sq < 1e-10 {
        return 0.0;
    }

    let d_norm = d / dist_sq.sqrt();

    let cos1 = n1.dot(&d_norm).abs();
    let cos2 = n2.dot(&(-d_norm)).abs();

    (cos1 * cos2) / dist_sq
}

/// Compute the geometry term with only the cosine at one point
/// Useful when one endpoint doesn't have a geometric normal (e.g., camera)
pub fn geometry_term_single(p1: Point3, n1: Vec3, p2: Point3) -> f64 {
    let d = p2 - p1;
    let dist_sq = d.magnitude_squared();

    if dist_sq < 1e-10 {
        return 0.0;
    }

    let d_norm = d / dist_sq.sqrt();
    let cos1 = n1.dot(&d_norm).abs();

    cos1 / dist_sq
}

/// Test visibility between two points in the scene
pub fn visible<S: Hittable>(scene: &S, p1: Point3, p2: Point3) -> bool {
    let d = p2 - p1;
    let dist = d.magnitude();

    if dist < 1e-8 {
        return true;
    }

    let ray = Ray::new(p1, d / dist);

    // Check if anything blocks the path between p1 and p2
    // We use a slightly reduced distance to avoid self-intersection at p2
    let t_max = dist - 1e-4;
    let t_min = 1e-4;

    if t_max <= t_min {
        return true;
    }

    scene.hit(&ray, Interval::new(t_min, t_max)).is_none()
}

/// Compute the solid angle PDF for a point on a surface viewed from another point
pub fn solid_angle_pdf(point_from: Point3, point_to: Point3, normal_to: Vec3, area: f64) -> f64 {
    let d = point_to - point_from;
    let dist_sq = d.magnitude_squared();

    if dist_sq < 1e-10 || area < 1e-10 {
        return 0.0;
    }

    let d_norm = d / dist_sq.sqrt();
    let cos_theta = normal_to.dot(&(-d_norm)).abs();

    if cos_theta < 1e-8 {
        return 0.0;
    }

    // Convert area PDF to solid angle PDF
    dist_sq / (cos_theta * area)
}

/// Convert area PDF to solid angle PDF
pub fn area_to_solid_angle_pdf(area_pdf: f64, dist_sq: f64, cos_theta: f64) -> f64 {
    if cos_theta < 1e-8 {
        return 0.0;
    }
    area_pdf * dist_sq / cos_theta
}

/// Convert solid angle PDF to area PDF
pub fn solid_angle_to_area_pdf(solid_angle_pdf: f64, dist_sq: f64, cos_theta: f64) -> f64 {
    solid_angle_pdf * cos_theta / dist_sq.max(1e-10)
}
