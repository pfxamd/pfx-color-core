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
pub mod difference;
pub mod gamut;
pub mod interpolation;
pub mod math;
pub mod spaces;

pub use contrast::{contrast_ratio, relative_luminance};
pub use conversion::convert;
pub use difference::{difference, DifferenceMethod};
pub use gamut::{is_in_gamut, map_to_gamut, GamutMap};
pub use interpolation::{interpolate, HueMethod};
pub use spaces::{Color, ColorError, ColorSpace};
