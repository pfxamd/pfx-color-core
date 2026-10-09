# PFx Color Core — Rust computation engine

Independent, first-party color mathematics, design tools and portable interfaces with **zero external Cargo dependencies**.

## Supported in the Rust source

- f64 conversion: sRGB and linear sRGB, Display P3/linear, Rec.2020/linear, XYZ D50/D65, CIE Lab D50/**D65**, LCH, OKLab/OKLCH, HSL/HWB/HSV, Adobe RGB (A98) and ProPhoto RGB
- CIE76, CIEDE2000 and Delta-E OK; **opaque in-gamut sRGB** WCAG 2.2 contrast
- Premultiplied-alpha interpolation with shorter/longer/increasing/decreasing/**raw** hue paths, HSL/HWB/HSV powerless-hue treatment and explicit gamut checking/mapping
- Deterministic tonal/ramp/anchored palettes, named/custom harmonies, seeded 10-color Color Study, and 2–256-stop linear/radial/conic **unit-square** gradient sampling
- Opt-in first-party `css_expression.rs`: a bounded subset of typed `calc()` arithmetic and relative CSS colors, with numeric channel references, nested origins, explicit angle/percentage units, overflow and invalid-dimension rejection; no third-party parser
- Opt-in `CssColor` missing-component API: preserve `none` channel/alpha masks, modern absolute-color round trips, analogous-set interpolation and zero-filled numeric conversion without modifying the existing `Color` type
- Absolute CSS color parsing/formatting: HEX, legacy/modern RGB and HSL, HWB, Lab/LCH, OKLab/OKLCH, supported `color()` spaces, 148 named colors and `transparent`
- Deterministic RGBA8 palette extraction from **caller-decoded, unpremultiplied** pixel buffers: up to 32 perceptual swatches, 64 MiB maximum input, at most 500,000 samples

## Bindings

- `crates/color-ffi/`: versioned C ABI and WebAssembly exports of the same Rust computation engine
- `bindings/javascript/pfx-color-core.mjs`: browser/Node WASM interface using built-in APIs only, no third-party runtime packages
- `bindings/javascript/pfx-color-tools.mjs`: optional Rust-backed picker operations
- `bindings/javascript/pfx-color-workspace.mjs`: optional JavaScript workspace/history with Rust-performed color math

**The production `pfx-colors` application and published TypeScript `v0.1.0` are unchanged.** The standalone Rust bindings are not a stable production release.

## Test and build

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p pfx-color-ffi --release
cargo build -p pfx-color-ffi --target wasm32-unknown-unknown --release
node --test bindings/javascript/pfx-color-core.smoke.mjs
```

The [Rust CI workflow](../../.github/workflows/rust-core-ci.yml) additionally checks native C linking, real Rust-vs-TypeScript parity, picker/workspace smoke and downloadable Linux/WASM artifacts. [Browser CI](../../.github/workflows/rust-browser-ci.yml) validates real WASM in headless Chromium/Firefox at desktop and mobile **viewport sizes**, not the production React app.

## Not yet implemented or verified

- Full CSS Color 4 grammar: `none`, basic typed `calc()` and literal-origin relative colors are implemented in the **separate opt-in CSS API**. External `var()`, `currentColor`, other CSS math functions, full Color 4 resolution and integration with the legacy picker/workspace or arbitrary gradients remain unsupported
- Specialized spaces/algorithms such as OKHSL/OKHSV, Delta-E ITP, Jz/HCT; APCA is **excluded** from this Apache-2.0 Rust engine pending licensing/integration review
- Pixel-accurate CSS gradient geometry and rendering for arbitrary box dimensions and shapes
- Compressed PNG/JPEG/WebP decoding, ICC color management or guaranteed ColorThief visual equivalence (the **decoded RGBA8 extractor is implemented**)
- Complete production React UI parity and native Windows/macOS validation; a stable Rust/FFI production release

Invalid or unsupported syntax is rejected rather than silently reinterpreted; numeric conversions preserve extended ranges and gamut mapping is explicit.

See [verified compatibility matrix](../../docs/color-parity.md), [migration state and priorities](../../docs/rust-migration.md) and [portable integration](../../docs/portable-bindings.md).
