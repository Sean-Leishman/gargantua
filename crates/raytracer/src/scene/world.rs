use crate::accel::BvhNode;
use crate::core::{Aabb, HitRecord, Hittable, Interval, Ray};
use std::sync::Arc;

/// A collection of hittable objects
pub struct World {
    objects: Vec<Arc<dyn Hittable>>,
    bbox: Aabb,
}

impl World {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            bbox: Aabb::EMPTY,
        }
    }

    /// Add an object to the world
    pub fn add<H: Hittable + 'static>(mut self, object: H) -> Self {
        let obj_box = object.bounding_box();
        self.bbox = self.bbox.union(&obj_box);
        self.objects.push(Arc::new(object));
        self
    }

    /// Add a pre-wrapped Arc object
    pub fn add_arc(mut self, object: Arc<dyn Hittable>) -> Self {
        let obj_box = object.bounding_box();
        self.bbox = self.bbox.union(&obj_box);
        self.objects.push(object);
        self
    }

    /// Number of objects in the world
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Check if world is empty
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Clear all objects
    pub fn clear(&mut self) {
        self.objects.clear();
        self.bbox = Aabb::EMPTY;
    }

    /// Build a BVH from the objects in this world for accelerated rendering
    pub fn build_bvh(self) -> BvhNode {
        BvhNode::build(self.objects)
    }

    /// Get the objects in this world (for building light lists, etc.)
    pub fn objects(&self) -> &[Arc<dyn Hittable>] {
        &self.objects
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl Hittable for World {
    fn hit<'a>(&'a self, ray: &Ray, t_range: Interval) -> Option<HitRecord<'a>> {
        let mut closest_hit: Option<HitRecord<'a>> = None;
        let mut closest_t = t_range.max;

        for object in &self.objects {
            if let Some(hit) = object.hit(ray, Interval::new(t_range.min, closest_t)) {
                closest_t = hit.t;
                closest_hit = Some(hit);
            }
        }

        closest_hit
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }
}
