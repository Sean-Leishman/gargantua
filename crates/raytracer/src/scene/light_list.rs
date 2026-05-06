use crate::core::{Aabb, HitRecord, Hittable, Interval, Point3, Ray, SurfaceSample, Vec3};
use rand::Rng;
use std::sync::Arc;

/// A list of light sources for importance sampling
#[derive(Clone)]
pub struct LightList {
    lights: Vec<Arc<dyn Hittable>>,
    bbox: Aabb,
}

impl LightList {
    pub fn new() -> Self {
        Self {
            lights: Vec::new(),
            bbox: Aabb::EMPTY,
        }
    }

    /// Add a light source
    pub fn add<H: Hittable + 'static>(mut self, light: H) -> Self {
        let light_box = light.bounding_box();
        self.bbox = self.bbox.union(&light_box);
        self.lights.push(Arc::new(light));
        self
    }

    /// Add a pre-wrapped Arc light source
    pub fn add_arc(mut self, light: Arc<dyn Hittable>) -> Self {
        let light_box = light.bounding_box();
        self.bbox = self.bbox.union(&light_box);
        self.lights.push(light);
        self
    }

    /// Number of lights
    pub fn len(&self) -> usize {
        self.lights.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.lights.is_empty()
    }

    /// Sample a light uniformly from the list
    ///
    /// Returns a reference to the selected light and the probability of selecting it.
    pub fn sample_light<R: Rng>(&self, rng: &mut R) -> Option<(&Arc<dyn Hittable>, f64)> {
        if self.lights.is_empty() {
            return None;
        }

        let idx = rng.r#gen_range(0..self.lights.len());
        let prob = 1.0 / self.lights.len() as f64;
        Some((&self.lights[idx], prob))
    }

    /// Get iterator over lights
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Hittable>> {
        self.lights.iter()
    }
}

impl Default for LightList {
    fn default() -> Self {
        Self::new()
    }
}

impl Hittable for LightList {
    fn hit<'a>(&'a self, ray: &Ray, t_range: Interval) -> Option<HitRecord<'a>> {
        let mut closest_hit: Option<HitRecord<'a>> = None;
        let mut closest_t = t_range.max;

        for light in &self.lights {
            if let Some(hit) = light.hit(ray, Interval::new(t_range.min, closest_t)) {
                closest_t = hit.t;
                closest_hit = Some(hit);
            }
        }

        closest_hit
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }

    fn pdf_value(&self, origin: Point3, direction: Vec3) -> f64 {
        if self.lights.is_empty() {
            return 0.0;
        }

        // Average PDF over all lights (uniform selection)
        let weight = 1.0 / self.lights.len() as f64;
        self.lights
            .iter()
            .map(|light| weight * light.pdf_value(origin, direction))
            .sum()
    }

    fn random_direction(&self, origin: Point3) -> Vec3 {
        if self.lights.is_empty() {
            return Vec3::new(1.0, 0.0, 0.0);
        }

        // Randomly select a light and sample from it
        let idx = rand::thread_rng().r#gen_range(0..self.lights.len());
        self.lights[idx].random_direction(origin)
    }

    fn sample_surface(&self) -> Option<SurfaceSample> {
        if self.lights.is_empty() {
            return None;
        }

        // Randomly select a light
        let idx = rand::thread_rng().r#gen_range(0..self.lights.len());
        let light = &self.lights[idx];

        // Sample its surface
        let sample = light.sample_surface()?;

        // Adjust PDF for light selection probability
        let light_select_prob = 1.0 / self.lights.len() as f64;

        Some(SurfaceSample {
            point: sample.point,
            normal: sample.normal,
            pdf: sample.pdf * light_select_prob,
        })
    }

    fn area(&self) -> f64 {
        self.lights.iter().map(|l| l.area()).sum()
    }
}
