//! Explicit RGB gamut checks and mapping.
//!
//! Mapping is never performed implicitly by Color::to(). The simple Oklch
//! constant-lightness/constant-hue method here uses radial chroma bisection,
//! and is *not* the Local-MINDE algorithm defined by CSS Color 4.
//! https://www.w3.org/TR/css-color-4/#gamut-mapping

use crate::spaces::{Color, ColorError, ColorSpace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamutMap {
    /// Clamp each encoded/linear RGB channel into [0,1], changing hue/lightness.
    Clip,
    /// Reduce Oklch chroma by bisection while keeping lightness and hue.
    OklchChroma,
}

fn bounded_rgb(space: ColorSpace) -> bool {
    matches!(
        space,
        ColorSpace::Srgb
            | ColorSpace::SrgbLinear
            | ColorSpace::DisplayP3
            | ColorSpace::DisplayP3Linear
            | ColorSpace::Rec2020
            | ColorSpace::Rec2020Linear
    )
}

fn channels_in_gamut(channels: [f64; 3]) -> bool {
    // Float matrices may produce tiny out-of-bound coordinates for white/black.
    channels.iter().all(|v| *v >= -1e-9 && *v <= 1.0 + 1e-9)
}

/// Whether the converted color is inside the RGB unit cube of the target.
/// Unbounded spaces (XYZ, Lab, LCH, Oklab, Oklch) return true.
pub fn is_in_gamut(input: Color, target: ColorSpace) -> Result<bool, ColorError> {
    if !bounded_rgb(target) {
        return Ok(true);
    }
    Ok(channels_in_gamut(input.to(target)?.channels()))
}

/// Map to a target with a specified strategy. For unbounded targets this is
/// equivalent to converting color coordinates. Does not mutate input.
pub fn map_to_gamut(
    input: Color,
    target: ColorSpace,
    method: GamutMap,
) -> Result<Color, ColorError> {
    let converted = input.to(target)?;
    if !bounded_rgb(target) || channels_in_gamut(converted.channels()) {
        return Ok(converted);
    }

    match method {
        GamutMap::Clip => {
            let channels = converted.channels().map(|v| v.clamp(0.0, 1.0));
            Color::new(target, channels, input.alpha())
        }
        GamutMap::OklchChroma => {
            let source = input.to(ColorSpace::Oklch)?.channels();
            if source[0] <= 0.0 {
                return Color::new(target, [0.0; 3], input.alpha());
            }
            if source[0] >= 1.0 {
                return Color::new(target, [1.0; 3], input.alpha());
            }

            let mut low = 0.0;
            let mut high = source[1].max(0.0);
            // A neutral color with L in (0, 1) lies inside an RGB gamut.
            // Maximize chroma along the same Oklch hue and lightness.
            for _ in 0..48 {
                let mid = (low + high) / 2.0;
                let candidate = Color::new(
                    ColorSpace::Oklch,
                    [source[0], mid, source[2]],
                    input.alpha(),
                )?;
                if is_in_gamut(candidate, target)? {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            let mapped = Color::new(
                ColorSpace::Oklch,
                [source[0], low, source[2]],
                input.alpha(),
            )?
            .to(target)?;
            // Final floating-point guard: output must lie exactly in [0,1].
            Color::new(
                target,
                mapped.channels().map(|v| v.clamp(0.0, 1.0)),
                input.alpha(),
            )
        }
    }
}
