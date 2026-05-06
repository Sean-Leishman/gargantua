use crate::core::{Aabb, HitRecord, Hittable, Interval, Point3, Ray, Vec3};
use std::sync::Arc;

/// Bounding Volume Hierarchy node for O(log n) ray intersection.
///
/// Internal nodes store the axis along which their children were split
/// (0=x, 1=y, 2=z). Traversal uses the sign of the ray direction on that
/// axis to descend into the *near* child first, so its hits narrow `t_max`
/// for the *far* child — often letting the far AABB test reject the whole
/// subtree.
pub enum BvhNode {
    Internal {
        left: Box<BvhNode>,
        right: Box<BvhNode>,
        bbox: Aabb,
        split_axis: u8,
    },
    Leaf {
        object: Arc<dyn Hittable>,
        bbox: Aabb,
    },
}

impl BvhNode {
    /// Build a BVH from a list of objects
    pub fn build(objects: Vec<Arc<dyn Hittable>>) -> Self {
        Self::build_recursive(objects)
    }

    fn build_recursive(mut objects: Vec<Arc<dyn Hittable>>) -> Self {
        let len = objects.len();

        // Base case: single object becomes a leaf
        if len == 1 {
            let obj = objects.pop().unwrap();
            let bbox = obj.bounding_box();
            return BvhNode::Leaf { object: obj, bbox };
        }

        // Base case: two objects - create internal node directly
        if len == 2 {
            let right_obj = objects.pop().unwrap();
            let left_obj = objects.pop().unwrap();
            let left_bbox = left_obj.bounding_box();
            let right_bbox = right_obj.bounding_box();
            let bbox = left_bbox.union(&right_bbox);
            let axis = bbox.longest_axis() as u8;

            return BvhNode::Internal {
                left: Box::new(BvhNode::Leaf {
                    object: left_obj,
                    bbox: left_bbox,
                }),
                right: Box::new(BvhNode::Leaf {
                    object: right_obj,
                    bbox: right_bbox,
                }),
                bbox,
                split_axis: axis,
            };
        }

        // Compute combined bounding box
        let combined_bbox = objects
            .iter()
            .fold(Aabb::EMPTY, |acc, obj| acc.union(&obj.bounding_box()));

        // Choose split axis (longest dimension)
        let axis = combined_bbox.longest_axis();

        // Sort objects by their bounding box center along the split axis
        objects.sort_by(|a, b| {
            let a_center = a.bounding_box().center()[axis];
            let b_center = b.bounding_box().center()[axis];
            a_center.partial_cmp(&b_center).unwrap()
        });

        // Try SAH-based split
        let (split_index, _best_cost) = Self::find_sah_split(&objects, axis, &combined_bbox);

        // Split objects
        let right_objects: Vec<_> = objects.drain(split_index..).collect();
        let left_objects = objects;

        // Recursively build children
        let left = Box::new(Self::build_recursive(left_objects));
        let right = Box::new(Self::build_recursive(right_objects));

        let bbox = left.bounding_box().union(&right.bounding_box());

        BvhNode::Internal { left, right, bbox, split_axis: axis as u8 }
    }

    /// Find the best split using Surface Area Heuristic (SAH)
    fn find_sah_split(
        objects: &[Arc<dyn Hittable>],
        _axis: usize,
        parent_bbox: &Aabb,
    ) -> (usize, f64) {
        let n = objects.len();
        if n <= 2 {
            return (1, f64::INFINITY);
        }

        // Cost of traversal vs intersection
        const T_TRAVERSAL: f64 = 1.0;
        const T_INTERSECT: f64 = 1.0;

        let parent_area = parent_bbox.surface_area();
        let mut best_cost = f64::INFINITY;
        let mut best_split = n / 2;

        // Compute prefix bounding boxes (left to right)
        let mut left_boxes: Vec<Aabb> = Vec::with_capacity(n);
        let mut running_box = Aabb::EMPTY;
        for obj in objects.iter() {
            running_box = running_box.union(&obj.bounding_box());
            left_boxes.push(running_box);
        }

        // Compute suffix bounding boxes (right to left) and evaluate SAH
        let mut right_box = Aabb::EMPTY;
        for i in (1..n).rev() {
            right_box = right_box.union(&objects[i].bounding_box());

            let left_count = i;
            let right_count = n - i;
            let left_area = left_boxes[i - 1].surface_area();
            let right_area = right_box.surface_area();

            // SAH cost function
            let cost = T_TRAVERSAL
                + T_INTERSECT * (left_area * left_count as f64 + right_area * right_count as f64)
                    / parent_area;

            if cost < best_cost {
                best_cost = cost;
                best_split = i;
            }
        }

        (best_split, best_cost)
    }
}

impl Hittable for BvhNode {
    fn hit<'a>(&'a self, ray: &Ray, t_range: Interval) -> Option<HitRecord<'a>> {
        // Precompute inverse direction once for the whole traversal.
        let inv_dir = Vec3::new(
            1.0 / ray.direction.x,
            1.0 / ray.direction.y,
            1.0 / ray.direction.z,
        );
        // Test the root bbox once here, then descend into a path that
        // tests children's bboxes from inside the parent — saving one
        // AABB test (and one function call on misses) per level.
        if !self.bounding_box().hit_precomputed(&ray.origin, &inv_dir, t_range) {
            return None;
        }
        self.descend(ray, &ray.origin, &inv_dir, t_range)
    }

    fn bounding_box(&self) -> Aabb {
        match self {
            BvhNode::Internal { bbox, .. } => *bbox,
            BvhNode::Leaf { bbox, .. } => *bbox,
        }
    }
}

impl BvhNode {
    /// Descend assuming the caller has already tested *this* node's bbox.
    ///
    /// At each internal node we test each child's bbox inline (rather than
    /// having the child's `descend` test it) — this saves one AABB test per
    /// level on rays that miss a subtree, plus a function call on misses.
    /// Children are visited in near-first order (chosen by the sign of the
    /// ray direction on the split axis) so a near hit narrows `t_max` and
    /// often makes the far subtree's AABB test reject outright.
    #[inline]
    fn descend<'a>(
        &'a self,
        ray: &Ray,
        origin: &Point3,
        inv_dir: &Vec3,
        t_range: Interval,
    ) -> Option<HitRecord<'a>> {
        match self {
            BvhNode::Internal { left, right, split_axis, .. } => {
                let dir_axis = match *split_axis {
                    0 => ray.direction.x,
                    1 => ray.direction.y,
                    _ => ray.direction.z,
                };
                let (near, far) = if dir_axis >= 0.0 {
                    (left.as_ref(), right.as_ref())
                } else {
                    (right.as_ref(), left.as_ref())
                };

                let near_hit = if near
                    .bounding_box()
                    .hit_precomputed(origin, inv_dir, t_range)
                {
                    near.descend(ray, origin, inv_dir, t_range)
                } else {
                    None
                };

                let far_range = Interval::new(
                    t_range.min,
                    near_hit.as_ref().map(|h| h.t).unwrap_or(t_range.max),
                );
                let far_hit = if far_range.min < far_range.max
                    && far
                        .bounding_box()
                        .hit_precomputed(origin, inv_dir, far_range)
                {
                    far.descend(ray, origin, inv_dir, far_range)
                } else {
                    None
                };

                far_hit.or(near_hit)
            }
            BvhNode::Leaf { object, .. } => object.hit(ray, t_range),
        }
    }
}
