# PFx Color Core / Rust

A portable color science implementation in Rust with **zero external Cargo dependencies**. It is developed alongside the existing TypeScript v0.1.0 baseline; the deployed PFx Colors UI is unchanged.

## Supported

- Encoded and linear sRGB, Display P3, CSS Rec.2020
- CIE XYZ D65/D50 (Bradford adaptation), Lab/LCH and Oklab/OKLCH
- `f64` precision, explicit validated alpha, unbounded color-space conversions
- Color differences: CIE76, CIEDE2000, Delta-E OK
- WCAG 2.2 luminance and contrast ratio, for opaque in-gamut sRGB colors
- Premultiplied-alpha interpolation in configurable color spaces with four hue paths
- Explicit gamut checks, RGB clipping and Oklch chroma-reduction mapping
- Build checks on native Rust and the `wasm32-unknown-unknown` target

## API examples

```rust
use pfx_color_core::{
    Color, ColorSpace, DifferenceMethod, difference, contrast_ratio,
    GamutMap, map_to_gamut, HueMethod, interpolate
};

let red = Color::new(ColorSpace::Srgb, [1.0, 0.0, 0.0], 1.0)?;
let white = Color::new(ColorSpace::Srgb, [1.0, 1.0, 1.0], 1.0)?;
let ratio = contrast_ratio(red, white)?;
let distance = difference(red, white, DifferenceMethod::Ciede2000)?;
let midpoint = interpolate(red, white, 0.5, ColorSpace::Oklab, HueMethod::Shorter)?;
let mapped = map_to_gamut(midpoint, ColorSpace::Srgb, GamutMap::OklchChroma)?;
# Ok::<(), pfx_color_core::ColorError>(())
```

## Explicit limitations

- Color input is numeric only; CSS parsing, missing components (`none`) and CSS color-mix() semantics are **not** fully implemented.
- WCAG contrast rejects transparent or out-of-sRGB-gamut inputs; the caller must explicitly composite/map first. APCA is not implemented.
- The Oklch chroma-reduction method is **not** the W3C Local-MINDE algorithm. Mapping changes color values; conversion never does.
- Image color analysis, palette/harmony/gradient tools and color profiles are not yet part of the Rust API.
- Building to a WASM target **does not** provide a JavaScript binding. Browser and C ABI bindings remain to be implemented.

## Validate

`cargo fmt --all -- --check`
`cargo test --workspace --all-targets`
`cargo clippy --workspace --all-targets -- -D warnings`

References: [W3C CSS Color 4](https://www.w3.org/TR/css-color-4/),
[WCAG 2.2](https://www.w3.org/TR/WCAG22/),
[Sharma et al. CIEDE2000 fixtures](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/).
