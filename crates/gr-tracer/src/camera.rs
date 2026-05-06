use gr_core::SpacetimePoint;
use nalgebra::Vector3;

pub struct Camera {
    pub position: SpacetimePoint,
    pub look_at: Vector3<f64>,
    pub up: Vector3<f64>,
    pub fov_y_radians: f64,
    pub aspect: f64,
}

impl Camera {
    /// Return the unit spatial direction in the camera's local orthonormal frame for
    /// normalized pixel coords (u, v) in [-1, 1] x [-1, 1].
    pub fn pixel_direction(&self, u: f64, v: f64) -> Vector3<f64> {
        let forward = self.look_at.normalize();
        let right = forward.cross(&self.up).normalize();
        let up = right.cross(&forward).normalize();

        let half_h = (self.fov_y_radians / 2.0).tan();
        let half_w = half_h * self.aspect;

        (forward + u * half_w * right + v * half_h * up).normalize()
    }
}
