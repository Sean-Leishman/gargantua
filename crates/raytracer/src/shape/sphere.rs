use crate::core::{Aabb, HitRecord, Hittable, Interval, Onb, Point3, Ray, SurfaceSample, Vec3};
use crate::material::Material;
use rand::Rng;
use std::f64::consts::PI;
use std::sync::Arc;

/// A sphere primitive
#[derive(Clone)]
pub struct Sphere {
    center: Point3,
    radius: f64,
    material: Arc<dyn Material>,
    bbox: Aabb,
}

impl Sphere {
    pub fn new<M: Material + 'static>(center: Point3, radius: f64, material: M) -> Self {
        let r_vec = Vec3::new(radius, radius, radius);
        let bbox = Aabb::from_points(center - r_vec, center + r_vec);

        Self {
            center,
            radius,
            material: Arc::new(material),
            bbox,
        }
    }

    /// Create sphere with pre-wrapped Arc material
    pub fn with_arc_material(center: Point3, radius: f64, material: Arc<dyn Material>) -> Self {
        let r_vec = Vec3::new(radius, radius, radius);
        let bbox = Aabb::from_points(center - r_vec, center + r_vec);

        Self {
            center,
            radius,
            material,
            bbox,
        }
    }

    /// Compute UV coordinates for a point on the sphere
    fn get_uv(p: &Vec3) -> (f64, f64) {
        let theta = (-p.y).acos();
        let phi = (-p.z).atan2(p.x) + PI;

        let u = phi / (2.0 * PI);
        let v = theta / PI;
        (u, v)
    }
}

impl Hittable for Sphere {
    fn hit<'a>(&'a self, ray: &Ray, t_range: Interval) -> Option<HitRecord<'a>> {
        let oc = ray.origin - self.center;
        let a = ray.direction.magnitude_squared();
        let half_b = oc.dot(&ray.direction);
        let c = oc.magnitude_squared() - self.radius * self.radius;

        let discriminant = half_b * half_b - a * c;
        if discriminant < 0.0 {
            return None;
        }

        let sqrt_d = discriminant.sqrt();

        // Find the nearest root in the acceptable range
        let mut root = (-half_b - sqrt_d) / a;
        if !t_range.surrounds(root) {
            root = (-half_b + sqrt_d) / a;
            if !t_range.surrounds(root) {
                return None;
            }
        }

        let point = ray.at(root);
        let outward_normal = (point - self.center) / self.radius;
        let uv = Self::get_uv(&outward_normal);

        Some(HitRecord::new(
            ray,
            point,
            outward_normal,
            root,
            uv,
            &*self.material,
        ))
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }

    fn pdf_value(&self, origin: Point3, direction: Vec3) -> f64 {
        // Check if ray actually hits the sphere
        if self
            .hit(
                &Ray::new(origin, direction),
                Interval::new(0.001, f64::INFINITY),
            )
            .is_none()
        {
            return 0.0;
        }

        // Solid angle subtended by sphere
        let dist_squared = (self.center - origin).magnitude_squared();
        let cos_theta_max = (1.0 - self.radius * self.radius / dist_squared).sqrt();
        let solid_angle = 2.0 * PI * (1.0 - cos_theta_max);

        1.0 / solid_angle
    }

    fn random_direction(&self, origin: Point3) -> Vec3 {
        let direction = self.center - origin;
        let dist_squared = direction.magnitude_squared();
        let onb = Onb::from_w(direction);
        Self::random_to_sphere(self.radius, dist_squared, &onb)
    }

    fn sample_surface(&self) -> Option<SurfaceSample> {
        // Sample uniformly on the unit sphere, then scale
        let mut rng = rand::thread_rng();

        // Use spherical coordinates with uniform sampling
        let z: f64 = rng.r#gen_range(-1.0..1.0);
        let r = (1.0 - z * z).sqrt();
        let phi: f64 = rng.r#gen_range(0.0..2.0 * PI);

        let normal = Vec3::new(r * phi.cos(), r * phi.sin(), z);
        let point = self.center + normal * self.radius;

        Some(SurfaceSample {
            point,
            normal,
            pdf: 1.0 / self.area(),
        })
    }

    fn area(&self) -> f64 {
        4.0 * PI * self.radius * self.radius
    }
}

impl Sphere {
    /// Generate a random direction toward a sphere using cone sampling
    fn random_to_sphere(radius: f64, dist_squared: f64, onb: &Onb) -> Vec3 {
        let mut rng = rand::thread_rng();
        let r1: f64 = rng.r#gen();
        let r2: f64 = rng.r#gen();

        let cos_theta_max = (1.0 - radius * radius / dist_squared).sqrt();
        let z = 1.0 + r2 * (cos_theta_max - 1.0);

        let phi = 2.0 * PI * r1;
        let x = phi.cos() * (1.0 - z * z).sqrt();
        let y = phi.sin() * (1.0 - z * z).sqrt();

        onb.local(x, y, z)
    }
}
