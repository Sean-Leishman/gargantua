//! Path vertex data structure for BDPT

use crate::core::{Color, Point3, Vec3};
use crate::material::Material;

/// Type of path vertex
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VertexType {
    /// Camera vertex (first vertex of camera subpath)
    Camera,
    /// Light vertex (first vertex of light subpath)
    Light,
    /// Surface vertex (intersection with geometry)
    Surface,
}

/// A vertex along a path (either camera or light subpath).
///
/// Borrows its `material` from the scene; `'a` is the scene lifetime.
/// A path vertex is only ever created and consumed within a single
/// `sample_pixel` call, so the lifetime stays trivial.
#[derive(Clone)]
pub struct PathVertex<'a> {
    /// World position of this vertex
    pub point: Point3,
    /// Surface normal at this vertex (for surface vertices)
    pub normal: Vec3,
    /// Material at this vertex (None for camera/light endpoints)
    pub material: Option<&'a dyn Material>,
    /// UV texture coordinates
    pub uv: (f64, f64),
    /// Whether ray hit from outside (front face)
    pub front_face: bool,
    /// Cumulative throughput from path origin to this vertex
    pub throughput: Color,
    /// Forward PDF (probability of sampling this vertex from previous)
    pub pdf_fwd: f64,
    /// Reverse PDF (probability of sampling previous vertex from this)
    pub pdf_rev: f64,
    /// Whether this vertex is on a delta distribution (specular)
    pub is_delta: bool,
    /// Incoming direction (from previous vertex)
    pub wi: Vec3,
    /// Outgoing direction (to next vertex, if any)
    pub wo: Vec3,
    /// Type of vertex
    pub vertex_type: VertexType,
}

impl<'a> PathVertex<'a> {
    /// Create a new camera vertex
    pub fn camera(point: Point3, direction: Vec3) -> Self {
        Self {
            point,
            normal: Vec3::zeros(),
            material: None,
            uv: (0.0, 0.0),
            front_face: true,
            throughput: Color::WHITE,
            pdf_fwd: 1.0,
            pdf_rev: 0.0,
            is_delta: true, // Camera is treated as delta
            wi: Vec3::zeros(),
            wo: direction.normalize(),
            vertex_type: VertexType::Camera,
        }
    }

    /// Create a new light vertex
    pub fn light(point: Point3, normal: Vec3, emission: Color, pdf_pos: f64, pdf_dir: f64) -> Self {
        Self {
            point,
            normal,
            material: None,
            uv: (0.0, 0.0),
            front_face: true,
            throughput: emission,
            pdf_fwd: pdf_pos * pdf_dir,
            pdf_rev: 0.0,
            is_delta: false,
            wi: Vec3::zeros(),
            wo: Vec3::zeros(),
            vertex_type: VertexType::Light,
        }
    }

    /// Create a surface vertex
    pub fn surface(
        point: Point3,
        normal: Vec3,
        material: &'a dyn Material,
        uv: (f64, f64),
        front_face: bool,
        wi: Vec3,
        throughput: Color,
        pdf_fwd: f64,
        is_delta: bool,
    ) -> Self {
        Self {
            point,
            normal,
            material: Some(material),
            uv,
            front_face,
            throughput,
            pdf_fwd,
            pdf_rev: 0.0,
            is_delta,
            wi,
            wo: Vec3::zeros(),
            vertex_type: VertexType::Surface,
        }
    }

    /// Check if this vertex is connectible (non-delta interior vertex)
    pub fn is_connectible(&self) -> bool {
        match self.vertex_type {
            VertexType::Camera | VertexType::Light => true,
            VertexType::Surface => !self.is_delta,
        }
    }

    /// Check if this is a light source vertex
    pub fn is_light(&self) -> bool {
        self.vertex_type == VertexType::Light
    }

    /// Check if this is a camera vertex
    pub fn is_camera(&self) -> bool {
        self.vertex_type == VertexType::Camera
    }

    /// Check if this is a surface vertex
    pub fn is_surface(&self) -> bool {
        self.vertex_type == VertexType::Surface
    }

    /// Get the emitted radiance at this vertex (if it's emissive)
    pub fn emitted(&self) -> Color {
        if let Some(mat) = self.material {
            mat.emitted(self.uv.0, self.uv.1, self.point)
        } else if self.is_light() {
            self.throughput // For light vertices, throughput is the emission
        } else {
            Color::BLACK
        }
    }
}
