//! PFx Color Core: portable, dependency-free color mathematics.
//!
//! This is the new Rust implementation. The existing TypeScript implementation
//! remains intact as a reference while the Rust implementation is validated.
//!
//! Conversions use unbounded f64 channels. No conversion silently clips or
//! gamut-maps a color to an RGB display range.
//!
//! Mathematical reference: CSS Color Module Level 4, 2026-09-01,
//! https://www.w3.org/TR/2026/CRD-css-color-4-20260901/

pub mod contrast;
pub mod conversion;
pub mod css;
pub mod css_missing;
pub mod css_expression;
pub mod cylindrical;
pub mod difference;
pub mod gamut;
pub mod gradients;
pub mod harmony;
pub mod images;
pub mod interpolation;
pub mod math;
pub mod palettes;
pub mod spaces;
pub mod study;

pub use contrast::{contrast_ratio, relative_luminance};
pub use conversion::convert;
pub use css::{format_css, format_hex, parse_css};
pub use css_missing::{
    format_css_missing, interpolate_css_missing, parse_css_missing, CssColor, MISSING_ALL,
};
pub use difference::{difference, DifferenceMethod};
pub use gamut::{is_in_gamut, map_to_gamut, GamutMap};
pub use gradients::{Gradient, GradientKind, GradientOptions, GradientSample, GradientStop};
pub use harmony::{
    generate_custom_harmony, generate_harmony, Harmony, HarmonyColor, HarmonyOptions, HarmonyScheme,
};
pub use images::{
    extract_image_palette, ExtractedColor, ImageError, ImagePalette, ImagePaletteOptions,
    ImageRegion, MAX_IMAGE_PALETTE_COLORS, MAX_IMAGE_SAMPLES,
};
pub use interpolation::{interpolate, HueMethod};
pub use palettes::{
    anchored_palette, ramp_palette, tonal_palette, Palette, PaletteColor, PaletteMode, RampOptions,
    TonalOptions, MAX_PALETTE_COLORS,
};
pub use spaces::{Color, ColorError, ColorSpace};
pub use study::{
    generate_color_study, ColorStudy, ColorStudyColor, ColorStudyOptions, STUDY_COUNT,
};
