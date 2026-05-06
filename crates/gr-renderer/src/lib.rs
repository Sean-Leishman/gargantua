// gr-renderer is now a thin CLI wrapper over `raytracer::curved`. The
// renderer itself lives in `raytracer::curved`; re-export the most useful
// items so existing callers (and the integration test) can keep their imports
// short.

pub use raytracer::curved::{
    AccretionDisk, Camera, GeodesicRay, RayOutcome, RenderOptions, render, render_with_disk,
    shade_outcome, shade_outcome_linear,
};
