# PFx Color Core — Rust computation engine

First-party color mathematics, design tools and portable interfaces, with **zero external Cargo dependencies**.

## Core support

- sRGB, linear sRGB, Display P3, Rec.2020, XYZ D50/D65, CIE Lab/LCH, OKLab/OKLCH, HSL/HWB/HSV
- CIE76, CIEDE2000, Delta-E OK and opaque sRGB WCAG contrast
- Premultiplied-alpha interpolation, explicit gamut checking, clipping, Oklch chroma reduction, and CSS Local MINDE
- Deterministic tonal/anchored/ramp palettes, named/custom harmonies, linear/radial/conic gradient sampling
- Seeded 10-color Color Study for the existing UI's design controls
- Absolute CSS color parsing/serialization, HEX (with explicit gamut mapping) and 148 named CSS colors

## Bindings and use

- The Rust engine in `crates/color-core/` exposes typed computational functions.
- The `crates/color-ffi/` crate exposes the same engine through the versioned C ABI and WebAssembly.
- The independent `bindings/javascript/pfx-color-core.mjs` module uses built-in browser WebAssembly APIs with no runtime packages.
- The opt-in `pfx-color-tools.mjs` and `pfx-color-workspace.mjs` modules orchestrate picker/state/history behavior without duplicating color mathematics in JavaScript.
- The deployed `pfx-colors` application and published TypeScript v0.1.0 package remain unchanged.

## Validation

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p pfx-color-ffi --release
cargo build -p pfx-color-ffi --target wasm32-unknown-unknown --release
node --test bindings/javascript/pfx-color-core.smoke.mjs
```

Further parity tests require building the TypeScript reference package. The repository's CI verifies the native C executable, WebAssembly under Node.js, real Rust-vs-TypeScript comparisons and an additional headless Chromium/Firefox run.

## Explicitly unsupported or incomplete

- Full CSS Color 4 grammar (notably missing `none` components, relative colors, `calc()`, `var()` and unimplemented spaces)
- APCA, DeltaE ITP/Jz/HCT, and some specialized color spaces
- Image extraction and ICC color profile management
- Pixel-perfect CSS gradient layout for arbitrary boxes, and native production React UI integration
- A public, stable Rust or FFI production release

These unsupported inputs are rejected rather than silently reinterpreted. Numeric color conversions preserve extended ranges; gamut mapping is always requested explicitly.

See [compatibility matrix](../../docs/color-parity.md), [portable bindings](../../docs/portable-bindings.md) and [W3C CSS Color 4](https://www.w3.org/TR/css-color-4/).
