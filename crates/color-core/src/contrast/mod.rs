//! WCAG 2.2 contrast for *opaque*, in-gamut sRGB colors.
//!
//! The ratio is symmetric and ranges from 1:1 to 21:1. This function refuses
//! alpha compositing and out-of-sRGB-gamut inputs instead of silently assuming
//! a backdrop or clipping an extended-gamut color.
//! https://www.w3.org/TR/WCAG22/#dfn-contrast-ratio

use crate::math::decode_srgb;
use crate::spaces::{Color, ColorError, ColorSpace};

/// Relative sRGB luminance in the WCAG 2.2 reference domain.
pub fn relative_luminance(color: Color) -> Result<f64, ColorError> {
    if color.alpha() != 1.0 {
        return Err(ColorError::RequiresOpaque);
    }
    let srgb = color.to(ColorSpace::Srgb)?.channels();
    if srgb.iter().any(|v| !(0.0..=1.0).contains(v)) {
        // An out-of-gamut input requires an explicit mapping decision.
        // Tolerances are for the practically unavoidable matrix round-off.
        if srgb.iter().any(|v| *v < -1e-9 || *v > 1.0 + 1e-9) {
            return Err(ColorError::OutOfGamut);
        }
    }
    let r = decode_srgb(srgb[0].clamp(0.0, 1.0));
    let g = decode_srgb(srgb[1].clamp(0.0, 1.0));
    let b = decode_srgb(srgb[2].clamp(0.0, 1.0));
    Ok(0.2126 * r + 0.7152 * g + 0.0722 * b)
}

/// WCAG 2.2 relative-luminance contrast ratio. Both inputs must be opaque
/// and inside the sRGB reference gamut. Does not automatically claim a
/// text element passes any specific WCAG success criterion.
pub fn contrast_ratio(a: Color, b: Color) -> Result<f64, ColorError> {
    let a_lum = relative_luminance(a)?;
    let b_lum = relative_luminance(b)?;
    let (lighter, darker) = if a_lum > b_lum {
        (a_lum, b_lum)
    } else {
        (b_lum, a_lum)
    };
    Ok((lighter + 0.05) / (darker + 0.05))
}
