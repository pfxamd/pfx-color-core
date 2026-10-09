# PFx Color Core / Rust

A portable color math implementation in Rust with **zero external Cargo dependencies**. This starts a new engine alongside (not replacing) the TypeScript v0.1.0 baseline.

## Supported in first math milestone
- Encoded and linear sRGB, Display P3, CSS Rec.2020
- CIE XYZ D65 and D50 with Bradford chromatic adaptation
- CIE Lab/LCH and perceptual Oklab/OKLCH
- f64 precision and non-clipping conversion with validated values and alpha
- Native Rust API; compiler validation for wasm32-unknown-unknown

## Not yet implemented
- CSS color parsing and missing-channel representation
- Gamut mapping, color difference, contrast and interpolation
- Gradient and palette generation in Rust
- Browser JavaScript binding and native C ABI

`cargo test --workspace` runs Rust tests. The existing React app remains connected to the published TypeScript reference core.

Reference: [CSS Color 4, September 2026](https://www.w3.org/TR/2026/CRD-css-color-4-20260901/).
