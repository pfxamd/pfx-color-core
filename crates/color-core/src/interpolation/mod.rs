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
    /// Direct interpolation of hue coordinates without circular adjustment.
    Raw,
}

fn hue_delta(mut from: f64, mut to: f64, method: HueMethod) -> f64 {
    if method == HueMethod::Raw {
        return to - from;
    }
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
        HueMethod::Raw => unreachable!("handled above"),
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
    // The hue channel is the third coordinate in LCH and the first
    // coordinate in HSL, HWB and HSV.
    let hue_index = match space {
        ColorSpace::Lch | ColorSpace::Oklch => Some(2),
        ColorSpace::Hsl | ColorSpace::Hwb | ColorSpace::Hsv => Some(0),
        _ => None,
    };
    if let Some(index) = hue_index {
        let powerless = |channels: [f64; 3]| match space {
            ColorSpace::Oklch => channels[1].abs() < 4e-6,
            ColorSpace::Lch => channels[1].abs() < 4e-4,
            ColorSpace::Hsl | ColorSpace::Hsv => channels[1].abs() < 1e-9,
            ColorSpace::Hwb => channels[1] + channels[2] >= 100.0 - 1e-9,
            _ => false,
        };
        let missing_a = powerless(a) || from.alpha() == 0.0;
        let missing_b = powerless(b) || to.alpha() == 0.0;
        if missing_a && !missing_b {
            a[index] = b[index];
        } else if missing_b && !missing_a {
            b[index] = a[index];
        }
    }
    let weight_a = (1.0 - fraction) * from.alpha();
    let weight_b = fraction * to.alpha();
    let alpha = weight_a + weight_b;
    let mut mixed = [0.0; 3];
    for i in 0..3 {
        if hue_index == Some(i) {
            let angle = a[i] + hue_delta(a[i], b[i], hue_method) * fraction;
            mixed[i] = if hue_method == HueMethod::Raw {
                angle
            } else {
                angle.rem_euclid(360.0)
            };
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
