//! Alpha-aware color interpolation in rectangular and polar color spaces.
//!
//! The interpolation design follows CSS Color 4: channels use premultiplied
//! alpha, and hue is not premultiplied. The current Color type has no CSS
//! missing-channel representation, so this API does not claim complete CSS
//! color-mix() equivalence.
//! https://www.w3.org/TR/css-color-4/#interpolation

use crate::spaces::{Color, ColorError, ColorSpace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HueMethod {
    Shorter,
    Longer,
    Increasing,
    Decreasing,
}

fn hue_delta(mut from: f64, mut to: f64, method: HueMethod) -> f64 {
    from = from.rem_euclid(360.0);
    to = to.rem_euclid(360.0);
    let mut delta = to - from;
    match method {
        HueMethod::Shorter => {
            if delta > 180.0 {
                delta -= 360.0;
            } else if delta < -180.0 {
                delta += 360.0;
            }
        }
        HueMethod::Longer => {
            if (0.0..180.0).contains(&delta) {
                delta -= 360.0;
            } else if (-180.0..=0.0).contains(&delta) {
                delta += 360.0;
            }
        }
        HueMethod::Increasing => delta = delta.rem_euclid(360.0),
        HueMethod::Decreasing => delta = -(-delta).rem_euclid(360.0),
    }
    delta
}

/// Mix two colors with a fraction in [0,1]. Results remain in the specified
/// interpolation space and are not gamut mapped or quantized.
///
/// Exact endpoints are preserved, including coordinates with zero alpha.
/// For intermediate 0-alpha results, the premultiplied components are zero.
/// Polar hue follows the given hue interpolation path and is not multiplied
/// by alpha; powerless achromatic hues use the other color's hue.
pub fn interpolate(
    start: Color,
    end: Color,
    fraction: f64,
    space: ColorSpace,
    hue_method: HueMethod,
) -> Result<Color, ColorError> {
    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
        return Err(ColorError::InvalidFraction);
    }
    let from = start.to(space)?;
    let to = end.to(space)?;
    if fraction == 0.0 {
        return Ok(from);
    }
    if fraction == 1.0 {
        return Ok(to);
    }
    let mut a = from.channels();
    let mut b = to.channels();
    let polar = matches!(space, ColorSpace::Lch | ColorSpace::Oklch);
    if polar {
        // CSS powerless-hue cutoffs, scaled to the lightness range.
        let epsilon = if space == ColorSpace::Oklch { 4e-6 } else { 4e-4 };
        if a[1].abs() < epsilon || from.alpha() == 0.0 {
            a[2] = b[2];
        }
        if b[1].abs() < epsilon || to.alpha() == 0.0 {
            b[2] = a[2];
        }
    }
    let weight_a = (1.0 - fraction) * from.alpha();
    let weight_b = fraction * to.alpha();
    let alpha = weight_a + weight_b;
    let mut mixed = [0.0; 3];
    for i in 0..3 {
        if polar && i == 2 {
            mixed[i] = (a[i] + hue_delta(a[i], b[i], hue_method) * fraction)
                .rem_euclid(360.0);
        } else {
            let numerator = a[i] * weight_a + b[i] * weight_b;
            mixed[i] = if alpha == 0.0 {
                numerator
            } else {
                numerator / alpha
            };
        }
    }
    Color::new(space, mixed, alpha).map_err(|_| ColorError::NonFiniteResult)
}
