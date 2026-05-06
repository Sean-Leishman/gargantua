use super::Camera;
use crate::core::{vec3, Point3, Ray, Vec3};

/// Standard pinhole perspective camera
#[derive(Clone, Debug)]
pub struct PerspectiveCamera {
    origin: Point3,
    lower_left: Point3,
    horizontal: Vec3,
    vertical: Vec3,
}

impl PerspectiveCamera {
    /// Create a perspective camera
    ///
    /// # Arguments
    /// * `look_from` - Camera position
    /// * `look_at` - Point camera is looking at
    /// * `vup` - World up vector (usually (0, 1, 0))
    /// * `vfov_degrees` - Vertical field of view in degrees
    /// * `aspect_ratio` - Width / height
    pub fn new(
        look_from: Point3,
        look_at: Point3,
        vup: Vec3,
        vfov_degrees: f64,
        aspect_ratio: f64,
    ) -> Self {
        let theta = vfov_degrees.to_radians();
        let h = (theta / 2.0).tan();
        let viewport_height = 2.0 * h;
        let viewport_width = aspect_ratio * viewport_height;

        // Camera coordinate frame
        let w = (look_from - look_at).normalize(); // Back
        let u = vup.cross(&w).normalize(); // Right
        let v = w.cross(&u); // Up

        let horizontal = viewport_width * u;
        let vertical = viewport_height * v;
        let lower_left = look_from - horizontal / 2.0 - vertical / 2.0 - w;

        Self {
            origin: look_from,
            lower_left,
            horizontal,
            vertical,
        }
    }

    /// Create camera looking down -Z axis (default orientation)
    pub fn default_at(position: Point3, vfov_degrees: f64, aspect_ratio: f64) -> Self {
        Self::new(
            position,
            position + vec3(0.0, 0.0, -1.0),
            vec3(0.0, 1.0, 0.0),
            vfov_degrees,
            aspect_ratio,
        )
    }
}

impl Camera for PerspectiveCamera {
    fn get_ray(&self, u: f64, v: f64) -> Ray {
        let direction =
            self.lower_left.coords + u * self.horizontal + v * self.vertical - self.origin.coords;

        Ray::new(self.origin, direction.normalize())
    }
}
