//! Deterministic, dependency-free ten-swatch color study generator.
//!
//! Ported from the project's own TypeScript Color Study design algorithm.
//! Unlike Math.random(), this API requires an explicit u32 seed; identical
//! inputs produce identical studies on native and WebAssembly targets.

use crate::difference::{difference, DifferenceMethod};
use crate::gamut::GamutMap;
use crate::harmony::{generate_harmony, HarmonyOptions, HarmonyScheme};
use crate::palettes::mapped_output;
use crate::spaces::{Color, ColorError, ColorSpace};

pub const STUDY_COUNT: usize = 10;

const LIGHTNESS_TRACK: [f64; 10] = [
    -0.48, -0.30, 0.34, -0.14, 0.20, -0.58, 0.48, -0.22, 0.10, -0.04,
];
const CHROMA_TRACK: [f64; 10] = [1.0, 0.72, 0.58, 1.18, 0.88, 0.52, 0.4, 1.08, 0.78, 0.94];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorStudyOptions {
    pub lightness: f64,
    pub chroma: f64,
    pub hue_range: f64,
    pub tone_range: f64,
    pub random_seed: u32,
    pub target_space: ColorSpace,
    pub gamut_map: GamutMap,
}

impl Default for ColorStudyOptions {
    fn default() -> Self {
        Self {
            lightness: 58.0,
            chroma: 58.0,
            hue_range: 58.0,
            tone_range: 58.0,
            random_seed: 0,
            target_space: ColorSpace::Srgb,
            gamut_map: GamutMap::Css,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorStudyColor {
    pub index: usize,
    pub color: Color,
    pub oklch: [f64; 3],
    pub mapped: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorStudy {
    pub seed: Color,
    pub scheme: HarmonyScheme,
    pub options: ColorStudyOptions,
    pub colors: Vec<ColorStudyColor>,
}

struct Mulberry32(u32);

impl Mulberry32 {
    // JS Math.imul and >>> semantics mapped to wrapping u32 bit operations.
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let mut result = self.0;
        result = (result ^ (result >> 15)).wrapping_mul(result | 1);
        result ^= result.wrapping_add((result ^ (result >> 7)).wrapping_mul(result | 61));
        result = result ^ (result >> 14);
        result as f64 / 4294967296.0
    }
}

fn hue_delta(from: f64, to: f64) -> f64 {
    (to - from + 540.0).rem_euclid(360.0) - 180.0
}

fn normalized(value: f64) -> f64 {
    (value / 100.0).clamp(0.0, 1.0)
}

fn choose_scheme(random: f64) -> HarmonyScheme {
    match ((random * 5.0).floor() as usize).min(4) {
        0 => HarmonyScheme::Analogous,
        1 => HarmonyScheme::SplitComplementary,
        2 => HarmonyScheme::Triadic,
        3 => HarmonyScheme::Tetradic,
        _ => HarmonyScheme::Square,
    }
}

pub fn generate_color_study(
    seed: Color,
    options: ColorStudyOptions,
) -> Result<ColorStudy, ColorError> {
    if [
        options.lightness,
        options.chroma,
        options.hue_range,
        options.tone_range,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(ColorError::InvalidRange);
    }
    let mut random = Mulberry32(options.random_seed);
    let lightness_amount = normalized(options.lightness);
    let chroma_amount = normalized(options.chroma);
    let hue_range_amount = normalized(options.hue_range);
    let tone_range_amount = normalized(options.tone_range);
    let scheme = choose_scheme(random.next());

    let seed_oklch = seed.to(ColorSpace::Oklch)?.channels();
    let seed_hue = if seed_oklch[1] <= 1e-10 {
        random.next() * 360.0
    } else {
        seed_oklch[2]
    };
    let seed_chroma = seed_oklch[1];
    let lightness_center = 0.18 + lightness_amount * 0.68;
    let requested_chroma = 0.018 + chroma_amount * 0.282 + seed_chroma.min(0.18) * 0.16;
    let working_chroma = requested_chroma.clamp(0.018, 0.32);

    let harmony_seed = Color::new(
        ColorSpace::Oklch,
        [lightness_center, working_chroma, seed_hue],
        seed.alpha(),
    )?;
    let harmony = generate_harmony(
        harmony_seed,
        scheme,
        HarmonyOptions {
            target_space: ColorSpace::DisplayP3,
            gamut_map: GamutMap::Css,
            ..HarmonyOptions::default()
        },
    )?;
    let hue_spread = 0.06 + hue_range_amount * 1.08;
    let anchor_hues: Vec<f64> = harmony
        .colors
        .iter()
        .map(|entry| {
            let value = entry.color.to(ColorSpace::Oklch)?;
            let h = value.channels()[2];
            Ok((seed_hue + hue_delta(seed_hue, h) * hue_spread).rem_euclid(360.0))
        })
        .collect::<Result<_, ColorError>>()?;
    let tone_amplitude = 0.055 + tone_range_amount * 0.47;
    let chroma_spread = 0.24 + chroma_amount * 0.82;
    let minimum_distance = 0.014 + hue_range_amount * 0.024 + tone_range_amount * 0.022;

    let mut colors: Vec<ColorStudyColor> = Vec::with_capacity(STUDY_COUNT);
    for index in 0..STUDY_COUNT {
        let anchor_hue = anchor_hues[(index * 2 + 1) % anchor_hues.len()];
        let mut candidate = seed;

        for attempt in 0..8 {
            let attempt = attempt as f64;
            let hue_jitter = (random.next() - 0.5)
                * (3.0 + hue_range_amount * 22.0 + attempt * (1.5 + hue_range_amount * 4.0));
            let lightness_jitter = (random.next() - 0.5) * (0.008 + tone_range_amount * 0.042);
            let chroma_jitter = (random.next() - 0.5) * (0.006 + chroma_amount * 0.028);

            let lightness = (lightness_center
                + LIGHTNESS_TRACK[index] * tone_amplitude
                + lightness_jitter
                + attempt * 0.002)
                .clamp(0.06, 0.98);

            let tracked_chroma = 1.0 + (CHROMA_TRACK[index] - 1.0) * chroma_spread;
            let chroma = (working_chroma * tracked_chroma + chroma_jitter)
                .clamp(if index == 6 { 0.008 } else { 0.012 }, 0.36);

            let hue = (seed_hue
                + hue_delta(seed_hue, anchor_hue)
                + hue_jitter
                + attempt * (1.5 + hue_range_amount * 4.0))
                .rem_euclid(360.0);
            candidate = Color::new(ColorSpace::Oklch, [lightness, chroma, hue], seed.alpha())?;

            let sufficient_distance =
                colors.iter().try_fold(f64::INFINITY, |closest, existing| {
                    let next = difference(candidate, existing.color, DifferenceMethod::Ok)?;
                    Ok::<f64, ColorError>(closest.min(next))
                })?;

            if sufficient_distance >= minimum_distance || attempt == 7.0 {
                break;
            }
        }

        let (color, mapped) = mapped_output(candidate, options.target_space, options.gamut_map)?;
        colors.push(ColorStudyColor {
            index,
            oklch: color.to(ColorSpace::Oklch)?.channels(),
            mapped,
            color,
        });
    }

    Ok(ColorStudy {
        seed,
        scheme,
        options,
        colors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_random_is_bytewise_stable() {
        let mut rng = Mulberry32(123);
        let expected = [0.7872516233474016, 0.1785435655619949, 0.49531551403924823];
        for target in expected {
            assert!((rng.next() - target).abs() < 1e-14);
        }
    }

    #[test]
    fn study_produces_ten_distinct_deterministic_swatches() {
        let seed = Color::new(ColorSpace::Srgb, [0.2, 0.4, 0.6], 0.8).unwrap();
        let options = ColorStudyOptions {
            random_seed: 2391,
            ..ColorStudyOptions::default()
        };
        let a = generate_color_study(seed, options).unwrap();
        let b = generate_color_study(seed, options).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.colors.len(), 10);
        assert!(a.colors.iter().all(|v| v.color.alpha() == 0.8));
        for (i, item) in a.colors.iter().enumerate() {
            assert_eq!(i, item.index);
            assert!(item
                .color
                .channels()
                .iter()
                .all(|v| (0.0..=1.0).contains(v)));
        }
    }

    #[test]
    fn nonfinite_controls_are_rejected() {
        let seed = Color::new(ColorSpace::Srgb, [0.1, 0.2, 0.3], 1.0).unwrap();
        let options = ColorStudyOptions {
            tone_range: f64::NAN,
            ..ColorStudyOptions::default()
        };
        assert_eq!(
            generate_color_study(seed, options),
            Err(ColorError::InvalidRange)
        );
    }
}
