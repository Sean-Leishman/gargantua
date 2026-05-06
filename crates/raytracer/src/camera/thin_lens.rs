use super::Camera;
use crate::core::{Point3, Ray, Vec3};
use rand::Rng;

/// Camera with depth of field (thin lens model)
/// Creates bokeh blur for out-of-focus objects
#[derive(Clone, Debug)]
pub struct ThinLensCamera {
    origin: Point3,
    lower_left: Point3,
    horizontal: Vec3,
    vertical: Vec3,
    u: Vec3,
    v: Vec3,
    lens_radius: f64,
}

impl ThinLensCamera {
    /// Create a thin lens camera with depth of field
    ///
    /// # Arguments
    /// * `look_from` - Camera position
    /// * `look_at` - Point camera is looking at
    /// * `vup` - World up vector (usually (0, 1, 0))
    /// * `vfov_degrees` - Vertical field of view in degrees
    /// * `aspect_ratio` - Width / height
    /// * `aperture` - Lens aperture diameter (0 = pinhole, no blur)
    /// * `focus_dist` - Distance to the focus plane
    pub fn new(
        look_from: Point3,
        look_at: Point3,
        vup: Vec3,
        vfov_degrees: f64,
        aspect_ratio: f64,
        aperture: f64,
        focus_dist: f64,
    ) -> Self {
        let theta = vfov_degrees.to_radians();
        let h = (theta / 2.0).tan();
        let viewport_height = 2.0 * h;
        let viewport_width = aspect_ratio * viewport_height;

        // Camera coordinate frame
        let w = (look_from - look_at).normalize(); // Back
        let u = vup.cross(&w).normalize(); // Right
        let v = w.cross(&u); // Up

        let horizontal = focus_dist * viewport_width * u;
        let vertical = focus_dist * viewport_height * v;
        let lower_left = look_from - horizontal / 2.0 - vertical / 2.0 - focus_dist * w;

        Self {
            origin: look_from,
            lower_left,
            horizontal,
            vertical,
            u,
            v,
            lens_radius: aperture / 2.0,
        }
    }

    /// Create camera with automatic focus on look_at point
    pub fn auto_focus(
        look_from: Point3,
        look_at: Point3,
        vup: Vec3,
        vfov_degrees: f64,
        aspect_ratio: f64,
        aperture: f64,
    ) -> Self {
        let focus_dist = (look_at - look_from).magnitude();
        Self::new(
            look_from,
            look_at,
            vup,
            vfov_degrees,
            aspect_ratio,
            aperture,
            focus_dist,
        )
    }
}

impl Camera for ThinLensCamera {
    fn get_ray(&self, s: f64, t: f64) -> Ray {
        // Random point on lens disk
        let rd = self.lens_radius * random_in_unit_disk();
        let offset = self.u * rd.x + self.v * rd.y;

        let ray_origin = self.origin + offset;
        let target = self.lower_left + s * self.horizontal + t * self.vertical;
        let direction = (target - ray_origin).normalize();

        Ray::new(ray_origin, direction)
    }
}

/// Random point in unit disk (for lens sampling)
fn random_in_unit_disk() -> Vec3 {
    let mut rng = rand::thread_rng();
    loop {
        let p = Vec3::new(rng.r#gen_range(-1.0..1.0), rng.r#gen_range(-1.0..1.0), 0.0);
        if p.magnitude_squared() < 1.0 {
            return p;
        }
    }
}
