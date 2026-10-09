# Rust engine migration and scientific contract

The TypeScript color engine and deployed PFx Colors UI remain unchanged.
Rust starts as a separate first-party computation implementation under
`crates/color-core/` within this repository.

## No external libraries

The Rust crate has no Cargo dependencies or dev-dependencies.
Its standard library (`std`) is part of the Rust language toolchain.
GitHub CI tools are build infrastructure, not runtime dependencies.

## Core boundaries

- `math/`: numeric primitives, linear matrices, signed transfer functions, white points.
- `spaces/`: explicit space identifiers, validated color coordinates and alpha.
- `conversion/`: D65 XYZ routing, D50 Bradford adaptation, CIE Lab/LCH and Oklab/OKLCH transforms.
- `difference/`: CIE76, CIEDE2000 and Delta-E OK.
- `contrast/`: WCAG opaque sRGB luminance and contrast ratio, with explicit refusal of unspecified compositing.
- `interpolation/`: alpha-premultiplied interpolation in color-space coordinates with explicit hue paths.
- `gamut/`: bounded RGB checks, clipping and a documented Oklch radial chroma strategy.
- `palettes/`: deterministic tonal palettes, ramps and anchored ramps, count bounded to 256.
- `harmony/`: Oklch geometric hue-offset schemes with configurable angles.
- `gradients/`: normalized unit-square geometry, stable hard-stop ordering, color-space/alpha-aware sampling.

No React, JavaScript packages, browser APIs, host filesystem, display pipeline
or profile-management logic belongs in the computational core.

## Defined behavior

1. Calculations use `f64` and **never** round to 8-bit RGB.
2. Out-of-gamut numeric channels are preserved; rendering and mapping are separate concerns.
3. Alpha stays unchanged across coordinate conversion.
4. Non-finite input and results are rejected.
5. The current numeric API does **not** parse CSS values. CSS missing values (`none`) must
   eventually use an explicit representation. They cannot be treated as numeric zero by a parser.
6. For mathematical polar conversion, zero-chroma hue is stored as 0 degrees by documented
   convention; it is not claimed to be the CSS `none` value.
7. CSS Rec.2020 uses the BT.1886 2.4 transfer function defined by CSS Color 4.
   It is **not** interchangeable with the broadcast camera OETF.

## Standards and validation

- [W3C CSS Color 4, 2026-09-01](https://www.w3.org/TR/2026/CRD-css-color-4-20260901/)
- [Original PFx TypeScript regression fixtures](../tests/engine/color-engine.test.ts)
- [Björn Ottosson — Oklab](https://bottosson.github.io/posts/oklab/)

Validate with `cargo test --workspace --all-targets`.
The CI separately compiles `wasm32-unknown-unknown`; a production WASM binding,
JavaScript glue, and C ABI are **not yet implemented**.

## Migration stages

1. Reference-tested mathematical base and color-space conversions (implemented).
2. Gamut checking/mapping, interpolation, perceptual difference and WCAG contrast (implemented as separate modules).
3. Palette, harmony and gradient generators (implemented and verified by new boundary/behavior tests).
4. C ABI and WASM bindings and cross-platform integration suites.
5. Replace the TypeScript engine only after verifying behavioral equivalence.

## Limitations of milestone 2

- Gamut mapping provides explicit clipping and a constant-Oklch-lightness/hue chroma search; the latter does not implement the W3C Local-MINDE algorithm.
- WCAG 2.2 luminance/contrast is defined for opaque, sRGB-in-gamut colors. Callers must explicitly composite transparent colors and map wide-gamut colors before invoking this API.
- The current numeric-only Color representation cannot preserve CSS missing component / `none` semantics during interpolation.
- Tests use published numeric fixtures and boundary cases; this is not yet a full browser compatibility or visual perception validation suite.

## Milestone 3 contracts

- Palettes are deterministic; no hidden random sampling or global state. They return typed colors and a mapping indicator.
- Harmony is geometric hue rotation, not an assertion of perceptual quality or accessible contrast.
- Gradient duplicate stop positions are stable; at an exact duplicate coordinate, the last supplied stop wins.
- Gradient geometry is defined for a normalized unit square; CSS rendering and serialization will be implemented in separate platform bindings.
- Existing TypeScript tools and the deployed UI are not replaced during the Rust migration.

## Releasing

The current GitHub tag `v0.1.0` refers to the existing TypeScript package, **not**
the new Rust crate. Rust versioning and its first tagged release should only be
assigned after its own validation gates pass.
