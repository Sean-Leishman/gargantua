/// A closed interval [min, max]
#[derive(Clone, Copy, Debug)]
pub struct Interval {
    pub min: f64,
    pub max: f64,
}

impl Interval {
    pub const EMPTY: Interval = Interval {
        min: f64::INFINITY,
        max: f64::NEG_INFINITY,
    };
    pub const UNIVERSE: Interval = Interval {
        min: f64::NEG_INFINITY,
        max: f64::INFINITY,
    };

    #[inline]
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn size(&self) -> f64 {
        self.max - self.min
    }

    #[inline]
    pub fn contains(&self, x: f64) -> bool {
        self.min <= x && x <= self.max
    }

    #[inline]
    pub fn surrounds(&self, x: f64) -> bool {
        self.min < x && x < self.max
    }

    #[inline]
    pub fn clamp(&self, x: f64) -> f64 {
        x.clamp(self.min, self.max)
    }

    /// Expand interval by delta on each side
    #[inline]
    pub fn expand(&self, delta: f64) -> Self {
        Self::new(self.min - delta, self.max + delta)
    }

    /// Union of two intervals (smallest interval containing both)
    #[inline]
    pub fn union(&self, other: &Interval) -> Self {
        Self::new(self.min.min(other.min), self.max.max(other.max))
    }

    /// Intersection of two intervals
    #[inline]
    pub fn intersect(&self, other: &Interval) -> Self {
        Self::new(self.min.max(other.min), self.max.min(other.max))
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min > self.max
    }
}

impl Default for Interval {
    fn default() -> Self {
        Self::EMPTY
    }
}
