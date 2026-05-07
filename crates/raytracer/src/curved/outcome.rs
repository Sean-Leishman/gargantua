use nalgebra::Vector3;

#[derive(Debug)]
pub enum RayOutcome {
    Horizon,
    Escaped {
        final_direction: Vector3<f64>,
    },
    /// Photon traversed the accretion disk; intensity is line-integrated
    /// emission with gravitational redshift applied.
    Disk {
        intensity: f64,
        color_temp: f64,
    },
    /// Photon hit a `Hittable` scene object. `color` is the linear-space
    /// shaded result (currently a one-bounce facing-ratio shading on the
    /// material's albedo / emission — *not* a recursive path trace).
    Scene {
        color: [f64; 3],
    },
    MaxSteps,
}
