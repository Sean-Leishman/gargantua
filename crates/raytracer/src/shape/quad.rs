use crate::core::{Aabb, HitRecord, Hittable, Interval, Point3, Ray, SurfaceSample, Vec3};
use crate::material::Material;
use rand::Rng;
use std::sync::Arc;

/// A quadrilateral (parallelogram) defined by a corner and two edge vectors
#[derive(Clone)]
pub struct Quad {
    /// Corner point
    q: Point3,
    /// First edge vector
    u: Vec3,
    /// Second edge vector
    v: Vec3,
    /// Material
    material: Arc<dyn Material>,
    /// Bounding box
    bbox: Aabb,
    /// Normal vector
    normal: Vec3,
    /// D coefficient in plane equation Ax + By + Cz = D
    d: f64,
    /// Precomputed for hit testing
    w: Vec3,
}

impl Quad {
    pub fn new<M: Material + 'static>(q: Point3, u: Vec3, v: Vec3, material: M) -> Self {
        Self::with_arc_material(q, u, v, Arc::new(material))
    }

    pub fn with_arc_material(q: Point3, u: Vec3, v: Vec3, material: Arc<dyn Material>) -> Self {
        let n = u.cross(&v);
        let normal = n.normalize();
        let d = normal.dot(&q.coords);
        let w = n / n.dot(&n);

        // Compute bounding box from the four corners
        let p0 = q;
        let p1 = q + u;
        let p2 = q + v;
        let p3 = q + u + v;

        let min = Point3::new(
            p0.x.min(p1.x).min(p2.x).min(p3.x),
            p0.y.min(p1.y).min(p2.y).min(p3.y),
            p0.z.min(p1.z).min(p2.z).min(p3.z),
        );
        let max = Point3::new(
            p0.x.max(p1.x).max(p2.x).max(p3.x),
            p0.y.max(p1.y).max(p2.y).max(p3.y),
            p0.z.max(p1.z).max(p2.z).max(p3.z),
        );

        // Pad thin bounding boxes
        let delta = 0.0001;
        let bbox = Aabb::from_points(
            Point3::new(min.x - delta, min.y - delta, min.z - delta),
            Point3::new(max.x + delta, max.y + delta, max.z + delta),
        );

        Self {
            q,
            u,
            v,
            material,
            bbox,
            normal,
            d,
            w,
        }
    }
}

impl Hittable for Quad {
    fn hit<'a>(&'a self, ray: &Ray, t_range: Interval) -> Option<HitRecord<'a>> {
        let denom = self.normal.dot(&ray.direction);

        // Ray is parallel to the plane
        if denom.abs() < 1e-8 {
            return None;
        }

        // Find the intersection point
        let t = (self.d - self.normal.dot(&ray.origin.coords)) / denom;

        if !t_range.surrounds(t) {
            return None;
        }

        let intersection = ray.at(t);
        let planar_hitpt = intersection - self.q;

        // Check if hit point is inside the quad using the local coordinate system
        let alpha = self.w.dot(&planar_hitpt.cross(&self.v));
        let beta = self.w.dot(&self.u.cross(&planar_hitpt));

        // Check bounds [0, 1] for both coordinates
        if alpha < 0.0 || alpha > 1.0 || beta < 0.0 || beta > 1.0 {
            return None;
        }

        Some(HitRecord::new(
            ray,
            intersection,
            self.normal,
            t,
            (alpha, beta),
            &*self.material,
        ))
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }

    fn pdf_value(&self, origin: Point3, direction: Vec3) -> f64 {
        if let Some(hit) = self.hit(
            &Ray::new(origin, direction),
            Interval::new(0.001, f64::INFINITY),
        ) {
            let area = self.u.cross(&self.v).magnitude();
            let distance_squared = hit.t * hit.t * direction.magnitude_squared();
            let cosine = direction.dot(&hit.normal).abs() / direction.magnitude();

            if cosine < 1e-8 {
                return 0.0;
            }

            distance_squared / (cosine * area)
        } else {
            0.0
        }
    }

    fn random_direction(&self, origin: Point3) -> Vec3 {
        let mut rng = rand::thread_rng();
        let random_point = self.q + self.u * rng.r#gen::<f64>() + self.v * rng.r#gen::<f64>();
        (random_point - origin).normalize()
    }

    fn sample_surface(&self) -> Option<SurfaceSample> {
        let mut rng = rand::thread_rng();
        let s: f64 = rng.r#gen();
        let t: f64 = rng.r#gen();

        let point = self.q + self.u * s + self.v * t;

        Some(SurfaceSample {
            point,
            normal: self.normal,
            pdf: 1.0 / self.area(),
        })
    }

    fn area(&self) -> f64 {
        self.u.cross(&self.v).magnitude()
    }
}

/// A 3D box made of 6 quads
#[derive(Clone)]
pub struct BoxShape {
    sides: Vec<Quad>,
    bbox: Aabb,
}

impl BoxShape {
    /// Create a box from two opposite corners
    pub fn new<M: Material + Clone + 'static>(a: Point3, b: Point3, material: M) -> Self {
        let mat = Arc::new(material);

        let min = Point3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
        let max = Point3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));

        let dx = Vec3::new(max.x - min.x, 0.0, 0.0);
        let dy = Vec3::new(0.0, max.y - min.y, 0.0);
        let dz = Vec3::new(0.0, 0.0, max.z - min.z);

        let sides = vec![
            // Front face (z = max.z)
            Quad::with_arc_material(Point3::new(min.x, min.y, max.z), dx, dy, mat.clone()),
            // Back face (z = min.z)
            Quad::with_arc_material(Point3::new(max.x, min.y, min.z), -dx, dy, mat.clone()),
            // Left face (x = min.x)
            Quad::with_arc_material(Point3::new(min.x, min.y, min.z), dz, dy, mat.clone()),
            // Right face (x = max.x)
            Quad::with_arc_material(Point3::new(max.x, min.y, max.z), -dz, dy, mat.clone()),
            // Top face (y = max.y)
            Quad::with_arc_material(Point3::new(min.x, max.y, max.z), dx, -dz, mat.clone()),
            // Bottom face (y = min.y)
            Quad::with_arc_material(Point3::new(min.x, min.y, min.z), dx, dz, mat.clone()),
        ];

        let bbox = Aabb::from_points(min, max);

        Self { sides, bbox }
    }
}

impl Hittable for BoxShape {
    fn hit<'a>(&'a self, ray: &Ray, t_range: Interval) -> Option<HitRecord<'a>> {
        let mut closest: Option<HitRecord<'a>> = None;
        let mut closest_t = t_range.max;

        for side in &self.sides {
            if let Some(hit) = side.hit(ray, Interval::new(t_range.min, closest_t)) {
                closest_t = hit.t;
                closest = Some(hit);
            }
        }

        closest
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }
}
