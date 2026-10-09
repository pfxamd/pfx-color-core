# PFx Color Core / Rust

A portable, first-party color science implementation in Rust with **zero external Cargo dependencies**. The Rust implementation is developed alongside the TypeScript v0.1.0 baseline; the deployed PFx Colors UI remains unchanged.

## Implemented

- Encoded and linear sRGB, Display P3, CSS Rec.2020
- CIE XYZ D65/D50 (Bradford adaptation), Lab/LCH, Oklab/OKLCH
- Unclipped f64 conversions with finite numeric coordinates and explicit alpha
- CIE76, CIEDE2000, Delta-E OK
- WCAG 2.2 luminance and ratio for opaque, sRGB-in-gamut colors
- Premultiplied-alpha interpolation, with four polar hue paths
- Explicit gamut checks, clipping and Oklch chroma reduction
- **Palettes:** tonal scales, two-anchor ramps and multiple-anchor ramps
- **Harmonies:** analogous, complementary, split-complementary, triadic, tetradic, square, custom hue offsets
- **Gradients:** ordered and duplicate-position hard stops, color-space interpolation, linear/radial/conic sampling in a normalized unit square
- Native Rust and wasm32-unknown-unknown compilation checks

## API example

```rust
use pfx_color_core::{
    Color, ColorSpace, TonalOptions, tonal_palette,
    RampOptions, ramp_palette,
    HarmonyScheme, HarmonyOptions, generate_harmony,
    Gradient, GradientStop, GradientOptions
};

fn main() -> Result<(), pfx_color_core::ColorError> {
    let red = Color::new(ColorSpace::Srgb, [1.0, 0.0, 0.0], 1.0)?;
    let blue = Color::new(ColorSpace::Srgb, [0.0, 0.0, 1.0], 1.0)?;
    let tonal = tonal_palette(red, TonalOptions::default())?;
    let ramp = ramp_palette(red, blue, RampOptions::default())?;
    let harmony = generate_harmony(red, HarmonyScheme::Triadic, HarmonyOptions::default())?;
    let gradient = Gradient::new(
        &[
            GradientStop { position: 0.0, color: red },
            GradientStop { position: 1.0, color: blue }
        ],
        GradientOptions::default()
    )?;
    let midpoint = gradient.sample(0.5)?;
    println!("{} {} {} {:?}", tonal.colors.len(), ramp.colors.len(), harmony.colors.len(), midpoint.color);
    Ok(())
}
```

## Design and safety boundaries

- Palette and harmony generation is **deterministic**. Harmony schemes are angle conventions, not scientifically guaranteed aesthetically pleasing or accessible combinations.
- Explicit RGB target-space gamut mapping; no conversion implicitly clips color channels.
- Only numeric colors are supported. CSS parsing, missing channels (`none`) and complete CSS color-mix semantics are not yet implemented.
- WCAG contrast refuses partially transparent or out-of-sRGB-gamut values; callers must explicitly handle backgrounds and gamut mapping.
- Gradient spatial sampling has *documented unit-square geometry*, not pixel-perfect CSS box geometry. CSS serializing/rendering belongs in web bindings.
- The Oklch chroma mapper is **not** the CSS Local-MINDE algorithm.
- Image extraction, ICC profile support, browser JS glue, C ABI and application bindings are not yet implemented. Compiling a wasm32 target alone does not create a browser library.
- Rust `v0.1.0` is a development crate version, not a released Rust API; existing GitHub tag `v0.1.0` refers to the original TypeScript release.

## Verify

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
cargo build --workspace --target wasm32-unknown-unknown --release
```

Sources: [W3C CSS Color 4](https://www.w3.org/TR/css-color-4/), [WCAG 2.2](https://www.w3.org/TR/WCAG22/) and [Sharma et al. CIEDE2000 data](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/).
