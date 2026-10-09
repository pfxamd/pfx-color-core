//! Validated color values and explicitly identified coordinate spaces.
//!
//! Channel values may be negative or larger than 1.0; they are not clipped.
//! Coordinates are currently required to be finite. CSS `none`/missing
//! channels are deliberately *not* interpreted as numeric zero. A future
//! parsing layer must represent missing channels explicitly.

use std::fmt;

/// Named coordinate systems supported by the first Rust math milestone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    Srgb,
    SrgbLinear,
    DisplayP3,
    DisplayP3Linear,
    Rec2020,
    Rec2020Linear,
    XyzD65,
    XyzD50,
    Lab,
    Lch,
    Oklab,
    Oklch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorError {
    NonFiniteChannel,
    InvalidAlpha,
    NonFiniteResult,
    RequiresOpaque,
    OutOfGamut,
    InvalidFraction,
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteChannel => f.write_str("color channels must all be finite"),
            Self::InvalidAlpha => f.write_str("alpha must be finite and between 0 and 1"),
            Self::NonFiniteResult => f.write_str("conversion produced a non-finite channel"),
            Self::RequiresOpaque => f.write_str("WCAG contrast requires opaque colors"),
            Self::OutOfGamut => f.write_str("color is outside the sRGB reference gamut"),
            Self::InvalidFraction => {
                f.write_str("interpolation fraction must be finite and between 0 and 1")
            }
        }
    }
}

impl std::error::Error for ColorError {}

/// A validated, unclipped f64 color value.
///
/// For rectangular RGB spaces, channels are generally normalized to 0..1
/// when inside the gamut, but extended values are intentionally permitted.
/// Lab lightness uses 0..100; Oklab lightness uses 0..1. Polar hues are degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    space: ColorSpace,
    channels: [f64; 3],
    alpha: f64,
}

impl Color {
    pub fn new(space: ColorSpace, channels: [f64; 3], alpha: f64) -> Result<Self, ColorError> {
        if channels.iter().any(|value| !value.is_finite()) {
            return Err(ColorError::NonFiniteChannel);
        }
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(ColorError::InvalidAlpha);
        }
        Ok(Self {
            space,
            channels,
            alpha,
        })
    }

    #[must_use]
    pub const fn space(self) -> ColorSpace {
        self.space
    }

    #[must_use]
    pub const fn channels(self) -> [f64; 3] {
        self.channels
    }

    #[must_use]
    pub const fn alpha(self) -> f64 {
        self.alpha
    }

    /// Convert without modifying the original, performing gamut mapping,
    /// or rounding to an 8-bit RGB representation.
    pub fn to(self, target: ColorSpace) -> Result<Self, ColorError> {
        crate::conversion::convert(self, target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_finite_channels_and_alpha() {
        assert_eq!(
            Color::new(ColorSpace::Srgb, [f64::NAN, 0.0, 0.0], 1.0),
            Err(ColorError::NonFiniteChannel)
        );
        assert_eq!(
            Color::new(ColorSpace::Srgb, [0.0, 0.0, 0.0], 1.1),
            Err(ColorError::InvalidAlpha)
        );
        assert_eq!(
            Color::new(ColorSpace::Srgb, [0.0, 0.0, 0.0], -0.1),
            Err(ColorError::InvalidAlpha)
        );
        assert!(Color::new(ColorSpace::Srgb, [-0.5, 1.3, 0.4], 0.35).is_ok());
    }
}
