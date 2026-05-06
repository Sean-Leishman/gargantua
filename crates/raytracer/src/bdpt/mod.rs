//! Bidirectional Path Tracing (BDPT) with Multiple Importance Sampling
//!
//! This module implements BDPT, which traces paths from both the camera and light
//! sources, then connects them to efficiently render challenging lighting scenarios
//! like caustics and indirect illumination from concentrated sources.

mod camera_path;
mod connection;
mod geometry;
mod light_path;
mod mis;
mod path;
mod renderer;
mod vertex;

pub use camera_path::generate_camera_path;
pub use connection::connect_paths;
pub use geometry::{geometry_term, visible};
pub use light_path::generate_light_path;
pub use mis::compute_mis_weight;
pub use path::Path;
pub use renderer::BdptRenderer;
pub use vertex::{PathVertex, VertexType};
