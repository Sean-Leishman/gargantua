use gr_core::SpacetimePoint;

/// Volumetric accretion disk model.
///
/// Uses spherical coordinates (t, r, θ, φ). The disk occupies the region
/// r_inner ≤ R ≤ r_outer where R = r sin θ is the cylindrical radius, and
/// has Gaussian vertical structure with scale height H.
pub struct AccretionDisk {
    /// Inner edge (ISCO), in geometric units. Default: 6M (Schwarzschild ISCO for M=1).
    pub r_inner: f64,
    /// Outer edge. Default: 20.0.
    pub r_outer: f64,
    /// Gaussian scale height H. Default: 1.0.
    pub scale_height: f64,
    /// Central density normalisation ρ₀. Default: 1.0.
    pub density_0: f64,
    /// Representative temperature T₀ (dimensionless proxy). Default: 1.0.
    pub temperature_0: f64,
}

impl Default for AccretionDisk {
    fn default() -> Self {
        Self {
            r_inner: 6.0,
            r_outer: 20.0,
            scale_height: 1.0,
            density_0: 1.0,
            temperature_0: 1.0,
        }
    }
}

impl AccretionDisk {
    /// Number density at a spacetime point.
    ///
    /// Model: ρ = ρ₀ · (r_in/R)^α · exp(−z²/(2H²))
    /// where R = r sin θ (cylindrical radius), z = r cos θ (height above equator),
    /// and α = 2 (surface density ∝ R^−1 gives this after vertical integration).
    /// Returns 0 outside the radial band [r_inner, r_outer].
    pub fn density(&self, pos: &SpacetimePoint) -> f64 {
        let r = pos[1];
        let theta = pos[2];

        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        // Cylindrical coords from spherical
        let cap_r = r * sin_theta.abs(); // cylindrical radius R = r |sin θ|
        let z = r * cos_theta;           // height above equatorial plane

        if cap_r < self.r_inner || cap_r > self.r_outer {
            return 0.0;
        }

        let radial = (self.r_inner / cap_r).powi(2); // α = 2
        let vertical = (-z * z / (2.0 * self.scale_height * self.scale_height)).exp();

        self.density_0 * radial * vertical
    }

    /// Emissivity (specific intensity proxy) at a spacetime point.
    ///
    /// j ∝ ρ² · T, where we use T ∝ (r_in/R)^(3/4) as a thin-disk blackbody
    /// temperature scaling. Returns j = ρ² · (r_in/R)^(3/4).
    pub fn emission(&self, pos: &SpacetimePoint) -> f64 {
        let rho = self.density(pos);
        if rho == 0.0 {
            return 0.0;
        }

        let r = pos[1];
        let theta = pos[2];
        let cap_r = r * theta.sin().abs();

        if cap_r < self.r_inner {
            return 0.0;
        }

        let temp_proxy = (self.r_inner / cap_r).powf(0.75);
        rho * rho * temp_proxy
    }
}
