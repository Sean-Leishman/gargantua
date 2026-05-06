//! Curved-spacetime renderer built on `gr-core`.
//!
//! Gated behind the `curved` cargo feature.

pub mod disk;
pub mod outcome;

pub use disk::AccretionDisk;
pub use outcome::RayOutcome;
