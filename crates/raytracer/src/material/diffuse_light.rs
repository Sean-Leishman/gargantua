use super::Material;
use crate::core::{Color, Point3, Ray, Vec3};

/// A material that emits light
#[derive(Clone, Debug)]
pub struct DiffuseLight {
    pub emit: Color,
}

impl DiffuseLight {
    pub fn new(color: Color) -> Self {
        Self { emit: color }
    }

    /// Create a white light with given intensity
    pub fn white(intensity: f64) -> Self {
        Self {
            emit: Color::new(intensity, intensity, intensity),
        }
    }
}

impl Material for DiffuseLight {
    fn scatter(
        &self,
        _ray: &Ray,
        _hit_point: Point3,
        _normal: Vec3,
        _front_face: bool,
    ) -> Option<(Color, Ray)> {
        // Lights don't scatter, they just emit
        None
    }

    fn emitted(&self, _u: f64, _v: f64, _point: Point3) -> Color {
        self.emit
    }

    fn bsdf(&self, _point: Point3, _normal: Vec3, _wi: Vec3, _wo: Vec3) -> Color {
        // Light sources don't scatter
        Color::BLACK
    }

    fn pdf(&self, _point: Point3, _normal: Vec3, _wi: Vec3, _wo: Vec3) -> f64 {
        0.0
    }

    fn is_delta(&self) -> bool {
        true // Lights are treated as endpoints, not scattering surfaces
    }
}
