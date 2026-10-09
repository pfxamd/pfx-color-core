# PFx Color Core — migration state

The original `v0.1.0` TypeScript package and published PFx Colors website remain
unchanged. Rust is maintained as an independent, first-party implementation in
the same repository. **No third-party Cargo dependencies** are permitted in
the engine or C/WASM bindings. Native and browser code uses the exact same
Rust computational functions, with no reimplementation of color math in JS.

## Repository structure

- `crates/color-core/src/math`, `spaces`, `conversion`: f64 matrix math,
  transfer curves, XYZ D65/D50, CIE Lab/LCH and OKLab/OKLCH
- `crates/color-core/src/css`, `cylindrical`: absolute CSS color parsing,
  named colors, HEX/CSS output, HSL/HWB/HSV
- `difference`, `contrast`, `interpolation`, `gamut`: CIE76/CIEDE2000/
  Delta-E OK, opaque sRGB WCAG contrast, premultiplied alpha interpolation and
  explicit gamut mapping (clip, Oklch chroma, CSS Local MINDE)
- `palettes`, `harmony`, `gradients`, `study`: deterministic color design
  algorithms, including the seedable 10-color PFx Colors Study
- `crates/color-ffi`: versioned native C ABI and WebAssembly exports
- `bindings/c`: stable C headers and linked C smoke test
- `bindings/javascript/pfx-color-core.mjs`: zero-runtime-package WASM loader
- `bindings/javascript/pfx-color-tools.mjs`: opt-in picker and Color Study
- `bindings/javascript/pfx-color-workspace.mjs`: opt-in tool state, palette,
  harmony, gradient orchestration and undo/redo; not included in production
- `scripts/browser-wasm-smoke.mjs`: independent Chromium/Firefox validation

## Engineering contracts

1. Full internal numeric precision is f64, with no RGB byte quantization
   until a user explicitly requests HEX output.
2. Conversion does not silently clip. Mapping is a caller's explicit choice.
3. Color space IDs are versioned across the FFI (do not export Rust enum ABI).
4. Invalid and non-finite input is rejected, not silently coerced.
5. Alpha stays intact during conversion; interpolation uses premultiplied
   alpha with explicit hue-path semantics.
6. The independent engine contains no React, browser, DOM or external
   third-party computation code.
7. Opaque C handles have explicit ownership and release functions.
8. Existing TypeScript and released website remain unchanged until the
   corresponding browser/UI regression gates have passed.

## Active validation

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
cargo build --workspace --target wasm32-unknown-unknown --release
node --test bindings/javascript/pfx-color-core.smoke.mjs
```

GitHub Actions additionally runs C-ABI smoke, real Rust WASM-versus-TypeScript
comparison, opt-in picker/workspace tests, and independent headless
Chromium/Firefox checks at desktop and mobile viewport sizes. Browser runners
are CI-only test infrastructure, not runtime dependencies.

## Remaining migration blockers

- CSS missing channel (`none`), relative-color syntax, `calc()`/`var()`
  and other unsupported CSS Color 4 grammar
- Additional spaces and algorithms: A98, ProPhoto, Lab D65, OKHSL/OKHSV,
  APCA, Delta-E ITP/Jz/HCT
- External ICC profiles and image palette extraction
- Complete CSS gradient pixel geometry and browser rendering semantics
- Actual React UI migration and user-interaction regression verification

See [color parity matrix](color-parity.md),
[bindings and independent build instructions](portable-bindings.md),
[W3C CSS Color 4](https://www.w3.org/TR/css-color-4/) and
[WCAG 2.2](https://www.w3.org/TR/WCAG22/).

The published `v0.1.0` tag refers to TypeScript, **not** a Rust release.
