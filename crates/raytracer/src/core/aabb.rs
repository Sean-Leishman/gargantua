use super::{Interval, Point3, Ray, Vec3};
use wide::f64x4;

/// Branchless min/max without NaN handling. The compiler turns `f64::min` /
/// `f64::max` into `vcmpunord + vblendv` chains because of IEEE 754-2008
/// minNum/maxNum NaN semantics. None of our AABB inputs can be NaN
/// (positions are finite; `inv_dir` is at worst ±∞ when a ray axis is 0,
/// which is well-defined), so we can use the cheaper compare-and-select.
#[inline(always)]
fn lt_min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}
#[inline(always)]
fn gt_max(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

/// Axis-Aligned Bounding Box
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Point3,
    pub max: Point3,
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb {
        min: Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
        max: Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
    };

    #[inline]
    pub fn new(min: Point3, max: Point3) -> Self {
        Self { min, max }
    }

    /// Create AABB from two arbitrary corner points
    pub fn from_points(a: Point3, b: Point3) -> Self {
        Self {
            min: Point3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
            max: Point3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
        }
    }

    /// Get interval for given axis (0=x, 1=y, 2=z)
    #[inline]
    pub fn axis_interval(&self, axis: usize) -> Interval {
        match axis {
            0 => Interval::new(self.min.x, self.max.x),
            1 => Interval::new(self.min.y, self.max.y),
            _ => Interval::new(self.min.z, self.max.z),
        }
    }

    /// Fast ray-box intersection test (optimized slab method)
    /// Branchless implementation for better SIMD/pipelining
    #[inline]
    pub fn hit(&self, ray: &Ray, t_range: Interval) -> bool {
        // Unrolled and branchless for better vectorization
        let inv_dx = 1.0 / ray.direction.x;
        let inv_dy = 1.0 / ray.direction.y;
        let inv_dz = 1.0 / ray.direction.z;

        let tx1 = (self.min.x - ray.origin.x) * inv_dx;
        let tx2 = (self.max.x - ray.origin.x) * inv_dx;
        let ty1 = (self.min.y - ray.origin.y) * inv_dy;
        let ty2 = (self.max.y - ray.origin.y) * inv_dy;
        let tz1 = (self.min.z - ray.origin.z) * inv_dz;
        let tz2 = (self.max.z - ray.origin.z) * inv_dz;

        // Branchless min/max selection
        let t_min_x = tx1.min(tx2);
        let t_max_x = tx1.max(tx2);
        let t_min_y = ty1.min(ty2);
        let t_max_y = ty1.max(ty2);
        let t_min_z = tz1.min(tz2);
        let t_max_z = tz1.max(tz2);

        let t_enter = t_min_x.max(t_min_y).max(t_min_z).max(t_range.min);
        let t_exit = t_max_x.min(t_max_y).min(t_max_z).min(t_range.max);

        t_enter < t_exit
    }

    /// Intersection test with precomputed inverse direction (BVH traversal hot path).
    ///
    /// 4-wide SIMD slab method via `wide::f64x4`. Lane 3 carries `t_range.min`
    /// in the "low t" packing and `t_range.max` in the "high t" packing, so
    /// the horizontal reduce gives `t_enter` and `t_exit` directly without
    /// any scalar fixup or memory round-trip.
    #[inline]
    pub fn hit_precomputed(&self, ray_origin: &Point3, inv_dir: &Vec3, t_range: Interval) -> bool {
        // mn[3]=t_range.min, mx[3]=t_range.max; o[3]=0, inv[3]=1 so:
        //   t1[3] = (t_range.min - 0) * 1 = t_range.min
        //   t2[3] = (t_range.max - 0) * 1 = t_range.max
        // After the per-lane min/max:
        //   t_lo[3] = min(t_range.min, t_range.max) = t_range.min   (entry floor)
        //   t_hi[3] = max(t_range.min, t_range.max) = t_range.max   (exit ceiling)
        // so reduce_max(t_lo) = t_enter, reduce_min(t_hi) = t_exit.
        let mn = f64x4::from([self.min.x, self.min.y, self.min.z, t_range.min]);
        let mx = f64x4::from([self.max.x, self.max.y, self.max.z, t_range.max]);
        let o = f64x4::from([ray_origin.x, ray_origin.y, ray_origin.z, 0.0]);
        let inv = f64x4::from([inv_dir.x, inv_dir.y, inv_dir.z, 1.0]);

        let t1 = (mn - o) * inv;
        let t2 = (mx - o) * inv;

        // Per-lane min/max: NaN-free fast packed min/max (single AVX op each).
        let t_lo = t1.fast_min(t2);
        let t_hi = t1.fast_max(t2);

        // Horizontal reduce. wide doesn't expose reduce_max/min, so we read
        // the lanes via as_array_ref (which is a borrow, no copy) and fold
        // with our NaN-free scalar helpers.
        let lo = t_lo.as_array_ref();
        let hi = t_hi.as_array_ref();
        let t_enter = gt_max(gt_max(gt_max(lo[0], lo[1]), lo[2]), lo[3]);
        let t_exit = lt_min(lt_min(lt_min(hi[0], hi[1]), hi[2]), hi[3]);

        t_enter < t_exit
    }

    /// Union of two bounding boxes
    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb {
            min: Point3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            max: Point3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        }
    }

    /// Expand box by a small padding
    pub fn pad(&self, delta: f64) -> Aabb {
        Aabb {
            min: self.min - Vec3::new(delta, delta, delta),
            max: self.max + Vec3::new(delta, delta, delta),
        }
    }

    /// Longest axis (0=x, 1=y, 2=z)
    pub fn longest_axis(&self) -> usize {
        let extent = self.max - self.min;
        if extent.x > extent.y && extent.x > extent.z {
            0
        } else if extent.y > extent.z {
            1
        } else {
            2
        }
    }

    /// Surface area (for SAH)
    pub fn surface_area(&self) -> f64 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }

    /// Center point
    #[inline]
    pub fn center(&self) -> Point3 {
        Point3::from((self.min.coords + self.max.coords) * 0.5)
    }
}

impl Default for Aabb {
    fn default() -> Self {
        Self::EMPTY
    }
}
