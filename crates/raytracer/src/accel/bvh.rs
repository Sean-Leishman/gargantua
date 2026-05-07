use crate::core::{Aabb, HitRecord, Hittable, Interval, Ray, Vec3};
use std::mem::MaybeUninit;
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
        /// AABBs of the children, stored *inline* in the parent so the
        /// traversal loop can test them without dereferencing the child
        /// `Box`. The original layout had to follow each Box pointer
        /// just to read the child's bbox — a near-certain L1 miss when
        /// the child hasn't been visited yet. With the bbox inline we
        /// only pay the cache miss on actual descent.
        left_bbox: Aabb,
        right_bbox: Aabb,
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
                left_bbox,
                right_bbox,
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

        let left_bbox = left.bounding_box();
        let right_bbox = right.bounding_box();
        let bbox = left_bbox.union(&right_bbox);

        BvhNode::Internal {
            left_bbox,
            right_bbox,
            left,
            right,
            bbox,
            split_axis: axis as u8,
        }
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
    /// Iterative BVH traversal with an explicit fixed-size stack and a
    /// single `closest` slot. Replaces the previous recursive descent,
    /// which (a) called itself per level — costing prologue/epilogue +
    /// Option<HitRecord> stack copies bubbling back through every frame
    /// — and (b) showed up at ~60% of total instructions in callgrind.
    ///
    /// Children are pushed *far first* so the *near* child is popped
    /// first; a near hit narrows `t_max` and lets the far child's AABB
    /// test reject the whole subtree on pop.
    fn hit<'a>(&'a self, ray: &Ray, t_range: Interval) -> Option<HitRecord<'a>> {
        let inv_dir = Vec3::new(
            1.0 / ray.direction.x,
            1.0 / ray.direction.y,
            1.0 / ray.direction.z,
        );
        let origin = ray.origin;

        // 64 levels covers any reasonable BVH (≈ 2^64 leaves). MaybeUninit
        // avoids a 512-byte memset per `hit()` call; the loop only ever
        // reads indices < `top`, which it owns.
        let mut stack: [MaybeUninit<&'a BvhNode>; 64] =
            unsafe { MaybeUninit::uninit().assume_init() };
        let mut top: usize = 0;

        let mut closest: Option<HitRecord<'a>> = None;
        let t_min = t_range.min;
        let mut t_max = t_range.max;

        if !self.bounding_box().hit_precomputed(&origin, &inv_dir, t_range) {
            return None;
        }
        stack[top].write(self);
        top += 1;

        while top > 0 {
            top -= 1;
            let node = unsafe { stack[top].assume_init() };
            let r = Interval::new(t_min, t_max);
            match node {
                BvhNode::Leaf { object, .. } => {
                    if let Some(h) = object.hit(ray, r) {
                        t_max = h.t;
                        closest = Some(h);
                    }
                }
                BvhNode::Internal {
                    left,
                    right,
                    left_bbox,
                    right_bbox,
                    split_axis,
                    ..
                } => {
                    let dir_axis = match *split_axis {
                        0 => ray.direction.x,
                        1 => ray.direction.y,
                        _ => ray.direction.z,
                    };
                    let (near, far, near_bbox, far_bbox) = if dir_axis >= 0.0 {
                        (left.as_ref(), right.as_ref(), left_bbox, right_bbox)
                    } else {
                        (right.as_ref(), left.as_ref(), right_bbox, left_bbox)
                    };
                    // Bbox tests use the inline parent-stored copies — no
                    // pointer chase to the children unless we descend.
                    let far_hit = far_bbox.hit_precomputed(&origin, &inv_dir, r);
                    let near_hit = near_bbox.hit_precomputed(&origin, &inv_dir, r);
                    // Push far first so near pops first.
                    if far_hit {
                        stack[top].write(far);
                        top += 1;
                    }
                    if near_hit {
                        stack[top].write(near);
                        top += 1;
                    }
                }
            }
        }
        closest
    }

    fn bounding_box(&self) -> Aabb {
        match self {
            BvhNode::Internal { bbox, .. } => *bbox,
            BvhNode::Leaf { bbox, .. } => *bbox,
        }
    }
}
