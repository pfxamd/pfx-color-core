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

pub mod conversion;
pub mod math;
pub mod spaces;

pub use conversion::convert;
pub use spaces::{Color, ColorError, ColorSpace};
