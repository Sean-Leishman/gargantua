pub mod accel;
pub mod bdpt;
pub mod camera;
pub mod core;
#[cfg(feature = "curved")]
pub mod curved;
pub mod flat;
pub mod material;
pub mod output;
pub mod pdf;
pub mod scene;
pub mod shape;

pub mod prelude {
    pub use crate::accel::BvhNode;
    pub use crate::bdpt::BdptRenderer;
    pub use crate::camera::{Camera, PerspectiveCamera, ThinLensCamera};
    pub use crate::core::{point3, vec3, Color, Hittable, Point3, Ray, Vec3};
    pub use crate::flat::{Background, FlatRenderer, SamplingStrategy};
    pub use crate::material::{Dielectric, DiffuseLight, Glossy, Lambertian, Material, Metal};
    pub use crate::output::{HdrBuffer, ImageBuffer, ToneMap};
    pub use crate::pdf::{CosinePdf, HittablePdf, MixturePdf, Pdf, SpherePdf, UniformHemispherePdf};
    pub use crate::scene::{LightList, World};
    pub use crate::shape::{BoxShape, ConstantMedium, Isotropic, Quad, Sphere};
}
