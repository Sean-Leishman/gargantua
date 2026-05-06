#[derive(Clone, Copy, Debug, Default)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Color {
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };

    pub fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    /// Convert to 8-bit RGB with gamma correction
    pub fn to_rgb_gamma(&self, gamma: f64) -> [u8; 3] {
        let inv_gamma = 1.0 / gamma;
        [
            (self.r.clamp(0.0, 1.0).powf(inv_gamma) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0).powf(inv_gamma) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0).powf(inv_gamma) * 255.0) as u8,
        ]
    }

    /// Convert to 8-bit RGB (no gamma, linear)
    pub fn to_rgb(&self) -> [u8; 3] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }

    /// Convert to 8-bit RGB using the proper sRGB OETF (piecewise: linear
    /// toe + 1/2.4 power). Input is assumed to be in [0,1] linear after
    /// any tone-mapping; values outside [0,1] are clamped.
    pub fn to_srgb_u8(&self) -> [u8; 3] {
        [
            srgb_encode(self.r),
            srgb_encode(self.g),
            srgb_encode(self.b),
        ]
    }
    /// Linear interpolation
    pub fn lerp(self, other: Color, t: f64) -> Color {
        self * (1.0 - t) + other * t
    }

    /// Compute luminance (perceived brightness)
    /// Uses standard Rec. 709 coefficients
    #[inline]
    pub fn luminance(&self) -> f64 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    /// Maximum component value
    #[inline]
    pub fn max_component(&self) -> f64 {
        self.r.max(self.g).max(self.b)
    }
}

// Arithmetic ops
impl std::ops::Add for Color {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Color::new(self.r + rhs.r, self.g + rhs.g, self.b + rhs.b)
    }
}

impl std::ops::AddAssign for Color {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Mul<f64> for Color {
    type Output = Self;
    fn mul(self, s: f64) -> Self {
        Color::new(self.r * s, self.g * s, self.b * s)
    }
}

impl std::ops::Mul<Color> for Color {
    type Output = Self;
    fn mul(self, rhs: Color) -> Self {
        Color::new(self.r * rhs.r, self.g * rhs.g, self.b * rhs.b)
    }
}

impl std::ops::Sub for Color {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Color::new(self.r - rhs.r, self.g - rhs.g, self.b - rhs.b)
    }
}

/// IEC 61966-2-1 sRGB encoding (linear → sRGB), 8-bit output. Clamps to [0,1].
#[inline]
fn srgb_encode(linear: f64) -> u8 {
    let l = linear.clamp(0.0, 1.0);
    let v = if l <= 0.003_130_8 {
        12.92 * l
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    (v * 255.0).round().clamp(0.0, 255.0) as u8
}

impl std::ops::Div<f64> for Color {
    type Output = Self;
    fn div(self, s: f64) -> Self {
        self * (1.0 / s)
    }
}
