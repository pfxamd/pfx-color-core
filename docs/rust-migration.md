# PFx Color Core — Rust migration state

**Code audit:** 2026-10-09, `main` at [`7b382b115d`](https://github.com/pfxamd/pfx-color-core/commit/7b382b115de33a43845a5dea9409a57e68d2210f).
**Production:** existing TypeScript `v0.1.0` and the deployed PFx Colors UI are **unchanged**. Rust has **no stable production release** and is not plugged into the default UI.

## Implemented (verified in source and tests)

- **Dependency-free Rust math** (`crates/color-core/`): f64 conversions for sRGB, linear RGB, P3, Rec.2020, XYZ D50/D65, Lab D50/**D65**, LCH, OKLab/OKLCH, HSL/HWB/HSV, Adobe RGB (A98) and ProPhoto.
- **Difference, contrast and gamut**: CIE76, CIEDE2000, Delta-E OK; opaque sRGB WCAG 2.2; explicit clip/Oklch/CSS Local MINDE mapping.
- **Interpolation**: alpha-aware rectangular/polar mixing, shorter/longer/increasing/decreasing/**raw** hue methods, HSL/HWB/HSV hue semantics and powerless-hue handling. The separate opt-in `CssColor` layer now handles explicitly missing channel/alpha values in supported modern absolute CSS syntax; the legacy numeric API does not.
- **Design functions**: deterministic tonal/ramp/anchored palettes, standard/custom harmonies, seeded 10-swatch Color Study, and 2–256-stop linear/radial/conic gradient sampling in normalized coordinates.
- **Opt-in modern CSS grammar**: typed `calc()` arithmetic and relative-color syntax from literal/absolute or nested relative origins are evaluated in first-party Rust via `css_expression.rs`. Channel references resolve to unitless numbers, percentage/angle typing is checked, and nonfinite/unsupported operations are rejected. External CSS variable resolution is intentionally absent.
- **Opt-in CSS missing components**: `css_missing.rs` preserves per-channel and alpha `none` masks, serializes them, borrows analogous components on interpolation, uses zero for ordinary numeric conversions, and exposes additive C/WASM operations. Existing numeric parsing and production picker remain unchanged.
- **Bounded CSS color IO**: absolute CSS formats (HEX, RGB/HSL/HWB, Lab/LCH/OKLab/OKLCH, supported `color()` spaces, 148 named colors) and formatting. Not a complete CSS Color 4 interpreter.
- **Decoded RGBA8 image palette extractor**: deterministic Oklab clustering of caller-decoded pixel buffers; 1–32 swatches, bounded memory and samples, region/alpha/near-white options. Not an image file decoder, an ICC color-managed pipeline or a drop-in ColorThief implementation.
- **Portable consumption**: versioned C ABI, WASM JS wrapper without external runtime packages, optional Rust-backed picker and JavaScript workspace/history. Color math remains inside Rust.
- **Independent CI**: tested Rust/C/native Linux, Node WASM and parity with real TypeScript; headless Chromium and Firefox at desktop and mobile viewport sizes.

## Current engineering contracts

1. Preserve f64 coordinates without byte quantization until explicit HEX export; no implicit clipping.
2. Treat gamut mapping as an explicit decision and reject invalid/non-finite input.
3. Keep stable numeric wire identifiers for color spaces/hue methods; avoid exporting compiler-layout Rust enums through the C ABI.
4. Keep input/output buffer lifetimes and owned handles explicit. Callers must respect pointer ownership.
5. Implement color calculations in the Rust crate only; JavaScript may orchestrate UI, history and transport, not duplicate algorithms.
6. Use no third-party Cargo crates in the computational engine or C/WASM bridge.
7. Leave the current TypeScript production engine pinned until **real UI** feature parity, acceptance and regression gates pass.
8. Keep APCA out of the Apache-2.0 Rust core pending a clear separate license/integration decision.

## Next work — do not reimplement completed features

| Priority | Work not yet complete | Required acceptance |
| --- | --- | --- |
| P0 (initial API implemented) | Complete `none` conformance and UI propagation | Dedicated opt-in parser/formatter/interpolator and C/WASM support exist. Extend powerless/cross-space fixtures and propagate deliberately into optional picker, workspace and gradient integrations before claiming full compatibility |
| P1 (bounded subset implemented) | Complete CSS expression conformance | `css_expression.rs` implements typed arithmetic and literal/nested relative colors in the opt-in API. Still needed: wider CSS math functions and variable resolution contracts (`var()`, `currentColor`, external environment), advanced grammar, reference fixture coverage and UI integration |
| P2 | Accurate CSS gradient layout | Real-size geometry and visual pixel regression in Chromium/Firefox; normalized unit-square calculations alone do not qualify |
| P3 | Missing specialized color spaces/differences | Prioritized, separately licensed reference-driven additions (e.g. OKHSL/OKHSV, ITP/Jz/HCT). APCA is a separate licensing decision |
| P4 | Image/color-management compatibility | Decode/ICC responsibilities clarified; representative images and visual legacy comparison before changing the UI image tool |
| P5 | Actual PFx Colors integration | Opt-in React bridge, interaction and browser regression, feature/UX/performance acceptance, planned fallback; default remains TypeScript |
| P6 | Release portability | Validated target-specific Windows/macOS builds if claimed; versioned Rust/FFI contract and a release only after preceding gates |

## Verified CI checkpoint

For `7b382b1` on 2026-10-09:

- [Rust Color Core CI — passed](https://github.com/pfxamd/pfx-color-core/actions/runs/37953015854)
- [Rust WASM Browser Compatibility — passed](https://github.com/pfxamd/pfx-color-core/actions/runs/37953015951)
- [Legacy TypeScript Color Core CI — passed](https://github.com/pfxamd/pfx-color-core/actions/runs/37953015739)

The browser suite runs **real WASM** in headless desktop/mobile **viewports**, not the deployed React application or real mobile devices. Linux C linkage does not prove native Windows/macOS portability.

### Reproduction commands

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
cargo build --workspace --target wasm32-unknown-unknown --release
node --test bindings/javascript/pfx-color-core.smoke.mjs
```

The complete CI also builds the original TypeScript reference, compares real Rust WASM against it, and runs opt-in picker/workspace plus browser tests.

See [audited capability matrix](color-parity.md) and [portable integration guide](portable-bindings.md).

The published `v0.1.0` tag is the **TypeScript extraction**, not a release of the Rust engine.
