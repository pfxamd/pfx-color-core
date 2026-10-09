//! Reproducible geometric color schemes in Oklch hue degrees.
//!
//! Analogous, complementary, split-complementary, triadic, tetradic and square
//! are angular design conventions. The library does not assert that a scheme
//! is universally harmonious or accessible; test contrast separately.

use crate::gamut::GamutMap;
use crate::palettes::mapped_output;
use crate::spaces::{Color, ColorError, ColorSpace};

pub const MAX_HARMONY_COLORS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarmonyScheme {
    Analogous,
    Complementary,
    SplitComplementary,
    Triadic,
    Tetradic,
    Square,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarmonyOptions {
    pub analogous_angle: f64,
    pub split_angle: f64,
    pub tetradic_angle: f64,
    pub target_space: ColorSpace,
    pub gamut_map: GamutMap,
}

impl Default for HarmonyOptions {
    fn default() -> Self {
        Self {
            analogous_angle: 30.0,
            split_angle: 30.0,
            tetradic_angle: 60.0,
            target_space: ColorSpace::Srgb,
            gamut_map: GamutMap::OklchChroma,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarmonyColor {
    pub index: usize,
    pub hue_offset: f64,
    pub color: Color,
    pub mapped: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Harmony {
    pub scheme: HarmonyScheme,
    pub base_hue: f64,
    pub colors: Vec<HarmonyColor>,
}

fn generate(
    seed: Color,
    offsets: &[f64],
    scheme: HarmonyScheme,
    options: HarmonyOptions,
) -> Result<Harmony, ColorError> {
    if !(2..=MAX_HARMONY_COLORS).contains(&offsets.len()) {
        return Err(ColorError::InvalidCount);
    }
    if offsets.iter().any(|angle| !angle.is_finite()) {
        return Err(ColorError::InvalidAngle);
    }
    let [lightness, chroma, hue] = seed.to(ColorSpace::Oklch)?.channels();
    let base_hue = hue.rem_euclid(360.0);
    let mut colors = Vec::with_capacity(offsets.len());
    for (index, &hue_offset) in offsets.iter().enumerate() {
        let hue = (base_hue + hue_offset).rem_euclid(360.0);
        if !hue.is_finite() {
            return Err(ColorError::NonFiniteResult);
        }
        let source = Color::new(ColorSpace::Oklch, [lightness, chroma, hue], seed.alpha())?;
        let (color, mapped) = mapped_output(source, options.target_space, options.gamut_map)?;
        colors.push(HarmonyColor {
            index,
            hue_offset,
            color,
            mapped,
        });
    }
    Ok(Harmony {
        scheme,
        base_hue,
        colors,
    })
}

fn valid_angle(angle: f64) -> bool {
    angle.is_finite() && (0.0..=180.0).contains(&angle)
}

/// Build a named hue-offset scheme from the given color.
pub fn generate_harmony(
    seed: Color,
    scheme: HarmonyScheme,
    options: HarmonyOptions,
) -> Result<Harmony, ColorError> {
    if !valid_angle(options.analogous_angle)
        || !valid_angle(options.split_angle)
        || !valid_angle(options.tetradic_angle)
    {
        return Err(ColorError::InvalidAngle);
    }
    let offsets: Vec<f64> = match scheme {
        HarmonyScheme::Analogous => {
            vec![-options.analogous_angle, 0.0, options.analogous_angle]
        }
        HarmonyScheme::Complementary => vec![0.0, 180.0],
        HarmonyScheme::SplitComplementary => {
            vec![
                0.0,
                180.0 - options.split_angle,
                180.0 + options.split_angle,
            ]
        }
        HarmonyScheme::Triadic => vec![0.0, 120.0, 240.0],
        HarmonyScheme::Tetradic => {
            vec![
                0.0,
                options.tetradic_angle,
                180.0,
                180.0 + options.tetradic_angle,
            ]
        }
        HarmonyScheme::Square => vec![0.0, 90.0, 180.0, 270.0],
        HarmonyScheme::Custom => return Err(ColorError::InvalidScheme),
    };
    generate(seed, &offsets, scheme, options)
}

/// Build a custom hue-offset scheme; requires 2..=256 finite offsets.
pub fn generate_custom_harmony(
    seed: Color,
    hue_offsets: &[f64],
    options: HarmonyOptions,
) -> Result<Harmony, ColorError> {
    generate(seed, hue_offsets, HarmonyScheme::Custom, options)
}
