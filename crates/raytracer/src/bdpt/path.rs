//! Path data structure for BDPT (chain of vertices)

use super::vertex::PathVertex;
use crate::core::Color;

/// A path consisting of a chain of vertices.
///
/// `'a` is the lifetime of the scene whose materials these vertices borrow.
#[derive(Clone, Default)]
pub struct Path<'a> {
    vertices: Vec<PathVertex<'a>>,
}

impl<'a> Path<'a> {
    /// Create an empty path
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
        }
    }

    /// Create a path with preallocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(capacity),
        }
    }

    /// Add a vertex to the path
    pub fn push(&mut self, vertex: PathVertex<'a>) {
        self.vertices.push(vertex);
    }

    /// Get the number of vertices in the path
    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    /// Check if the path is empty
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Get a vertex by index
    pub fn get(&self, index: usize) -> Option<&PathVertex<'a>> {
        self.vertices.get(index)
    }

    /// Get a mutable vertex by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PathVertex<'a>> {
        self.vertices.get_mut(index)
    }

    /// Get the last vertex
    pub fn last(&self) -> Option<&PathVertex<'a>> {
        self.vertices.last()
    }

    /// Get a mutable reference to the last vertex
    pub fn last_mut(&mut self) -> Option<&mut PathVertex<'a>> {
        self.vertices.last_mut()
    }

    /// Iterate over vertices
    pub fn iter(&self) -> impl Iterator<Item = &PathVertex<'a>> {
        self.vertices.iter()
    }

    /// Get the total throughput of the path (from start to end)
    pub fn throughput(&self) -> Color {
        self.vertices.last().map(|v| v.throughput).unwrap_or(Color::BLACK)
    }

    /// Truncate path to given length
    pub fn truncate(&mut self, len: usize) {
        self.vertices.truncate(len);
    }
}

impl<'a> std::ops::Index<usize> for Path<'a> {
    type Output = PathVertex<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.vertices[index]
    }
}

impl<'a> std::ops::IndexMut<usize> for Path<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.vertices[index]
    }
}
