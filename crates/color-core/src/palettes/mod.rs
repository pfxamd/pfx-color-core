//! Deterministic, reference-space-aware palettes.
//!
//! Palette sampling is a design algorithm, not a scientific claim that any
//! resulting palette is aesthetically optimal. All gamut mapping is explicit.
//! Results use Color values, never CSS strings or 8-bit hex encodings.

use crate::gamut::{is_in_gamut, map_to_gamut, GamutMap};
use crate::interpolation::{interpolate, HueMethod};
use crate::spaces::{Color, ColorError, ColorSpace};

pub const MAX_PALETTE_COLORS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteMode {
    Tonal,
    Ramp,
    Anchors,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteColor {
    pub index: usize,
    pub position: f64,
    pub color: Color,
    pub mapped: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub mode: PaletteMode,
    pub colors: Vec<PaletteColor>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TonalOptions {
    pub count: usize,
    pub min_lightness: f64,
    pub max_lightness: f64,
    pub chroma_scale: f64,
    pub target_space: ColorSpace,
    pub gamut_map: GamutMap,
}

impl Default for TonalOptions {
    fn default() -> Self {
        Self {
            count: 9,
            min_lightness: 0.12,
            max_lightness: 0.96,
            chroma_scale: 1.0,
            target_space: ColorSpace::Srgb,
            gamut_map: GamutMap::OklchChroma,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RampOptions {
    pub count: usize,
    pub interpolation_space: ColorSpace,
    pub target_space: ColorSpace,
    pub hue_method: HueMethod,
    pub gamut_map: GamutMap,
}

impl Default for RampOptions {
    fn default() -> Self {
        Self {
            count: 9,
            interpolation_space: ColorSpace::Oklch,
            target_space: ColorSpace::Srgb,
            hue_method: HueMethod::Shorter,
            gamut_map: GamutMap::OklchChroma,
        }
    }
}

pub(crate) fn validate_count(count: usize) -> Result<(), ColorError> {
    if (2..=MAX_PALETTE_COLORS).contains(&count) {
        Ok(())
    } else {
        Err(ColorError::InvalidCount)
    }
}

pub(crate) fn mapped_output(
    source: Color,
    target_space: ColorSpace,
    method: GamutMap,
) -> Result<(Color, bool), ColorError> {
    let mapped = !is_in_gamut(source, target_space)?;
    if mapped {
        Ok((map_to_gamut(source, target_space, method)?, true))
    } else {
        Ok((source.to(target_space)?, false))
    }
}

fn sample(
    mode: PaletteMode,
    count: usize,
    target_space: ColorSpace,
    method: GamutMap,
    mut f: impl FnMut(f64) -> Result<Color, ColorError>,
) -> Result<Palette, ColorError> {
    validate_count(count)?;
    let mut colors = Vec::with_capacity(count);
    for index in 0..count {
        let position = index as f64 / (count - 1) as f64;
        let source = f(position)?;
        let (color, mapped) = mapped_output(source, target_space, method)?;
        colors.push(PaletteColor {
            index,
            position,
            color,
            mapped,
        });
    }
    Ok(Palette { mode, colors })
}

/// Generate equal-spaced Oklch lightness steps retaining seed hue/chroma.
/// Lightness is a unit interval; chroma is nonnegative and unbounded.
/// Out-of-target-gamut colors are mapped only with the requested method.
pub fn tonal_palette(seed: Color, options: TonalOptions) -> Result<Palette, ColorError> {
    validate_count(options.count)?;
    let min = options.min_lightness;
    let max = options.max_lightness;
    if !min.is_finite()
        || !max.is_finite()
        || !(0.0..=1.0).contains(&min)
        || !(0.0..=1.0).contains(&max)
        || min >= max
        || !options.chroma_scale.is_finite()
        || options.chroma_scale < 0.0
    {
        return Err(ColorError::InvalidRange);
    }
    let [_, chroma, hue] = seed.to(ColorSpace::Oklch)?.channels();
    let chroma = chroma * options.chroma_scale;
    sample(
        PaletteMode::Tonal,
        options.count,
        options.target_space,
        options.gamut_map,
        |position| {
            Color::new(
                ColorSpace::Oklch,
                [min + (max - min) * position, chroma, hue],
                seed.alpha(),
            )
        },
    )
}

/// Interpolate two anchors. By default the interpolation is perceptual Oklch,
/// with CSS shorter-hue path and premultiplied alpha.
pub fn ramp_palette(
    first: Color,
    last: Color,
    options: RampOptions,
) -> Result<Palette, ColorError> {
    sample(
        PaletteMode::Ramp,
        options.count,
        options.target_space,
        options.gamut_map,
        |position| {
            interpolate(
                first,
                last,
                position,
                options.interpolation_space,
                options.hue_method,
            )
        },
    )
}

/// Interpolate evenly spaced anchors. The first and last are exact anchors;
/// intermediate anchors are reproduced exactly when their position coincides
/// with a sampled index, otherwise the output samples between them.
pub fn anchored_palette(
    anchors: &[Color],
    options: RampOptions,
) -> Result<Palette, ColorError> {
    validate_count(options.count)?;
    if anchors.len() < 2 || anchors.len() > options.count {
        return Err(ColorError::InvalidCount);
    }
    let segments = anchors.len() - 1;
    sample(
        PaletteMode::Anchors,
        options.count,
        options.target_space,
        options.gamut_map,
        |position| {
            let scaled = position * segments as f64;
            let segment = (scaled.floor() as usize).min(segments - 1);
            let fraction = if position == 1.0 {
                1.0
            } else {
                scaled - segment as f64
            };
            interpolate(
                anchors[segment],
                anchors[segment + 1],
                fraction,
                options.interpolation_space,
                options.hue_method,
            )
        },
    )
}
