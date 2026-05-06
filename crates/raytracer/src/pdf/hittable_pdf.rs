use crate::core::{Hittable, Point3, Vec3};
use crate::pdf::Pdf;
use std::sync::Arc;

/// PDF for sampling toward a hittable object (e.g., a light)
pub struct HittablePdf {
    origin: Point3,
    object: Arc<dyn Hittable>,
}

impl HittablePdf {
    pub fn new(origin: Point3, object: Arc<dyn Hittable>) -> Self {
        Self { origin, object }
    }
}

impl Pdf for HittablePdf {
    fn value(&self, direction: Vec3) -> f64 {
        self.object.pdf_value(self.origin, direction)
    }

    fn generate(&self) -> Vec3 {
        self.object.random_direction(self.origin)
    }
}
