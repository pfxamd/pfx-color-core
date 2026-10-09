# PFx Color Core — portable integrations

The Rust mathematical engine remains in crates/color-core/ and the original
TypeScript v0.1.0 implementation and deployed PFx Colors UI are unchanged.

**No third-party runtime libraries:** crates/color-ffi/ links only the first-party
sibling crate, crates/color-core/. Both crates have no external Cargo dependencies.
The JavaScript binding only uses built-in WebAssembly APIs, and the native binding
uses a standard C header. Rust toolchain/Node/GitHub Actions are build/test tools.

## Native C ABI

Build with the stable Rust toolchain:

    cargo build -p pfx-color-ffi --release

Linux: target/release/libpfx_color_ffi.so and .a
macOS: libpfx_color_ffi.dylib and .a
Windows: corresponding .dll and import/static libraries (subject to target toolchain).

Include bindings/c/pfx_color_core.h in any C-compatible language. ABI version
can be detected through pfx_abi_version() and equals 1. The PfxColor struct is
40 bytes, with explicit reserved word and three f64 channels, encoded by
documented numeric space IDs. C callers own their stack-allocated colors;
owned palette, gradient and buffer handles must be freed by matching pfx_*_free
functions exactly once. Color input/output pointers may alias.

The C ABI exposes:
- sRGB, P3, Rec.2020, XYZ, Lab and OKLab conversion;
- CIE76/CIEDE2000/OK differences, WCAG ratio and luminance;
- interpolation, gamut check and mapping;
- tonal and two-color ramp palettes;
- six named harmony schemes;
- multi-stop gradient builders with normalized sample and sampleXY.

The Rust core additionally exposes anchored palettes and arbitrary hue-offset
harmonies. These two specialized functions have **not yet** been added to the
foreign-language ABI.

A native C executable is compiled and linked against the real Rust library in
GitHub CI, using bindings/c/smoke.c. The ABI uses status codes; numeric scalar
errors produce NAN and handle constructors return null.

## JavaScript / WebAssembly (browser and Node.js)

Compile with Rust's WebAssembly target installed:

    rustup target add wasm32-unknown-unknown
    cargo build -p pfx-color-ffi --target wasm32-unknown-unknown --release

This creates target/wasm32-unknown-unknown/release/pfx_color_ffi.wasm.
Copy that .wasm and bindings/javascript/pfx-color-core.mjs into your project.
They are self-contained artifacts, and no dependency on this Git repository
is needed at runtime.

Example JavaScript:

    import { createPfxColorCore } from "./pfx-color-core.mjs";

    const bytes = await fetch("./pfx_color_ffi.wasm").then(r => r.arrayBuffer());
    const core = await createPfxColorCore(bytes);
    const red = { space: "srgb", channels: [1, 0, 0], alpha: 1 };
    const blue = { space: "srgb", channels: [0, 0, 1], alpha: 1 };
    console.log(core.interpolate(red, blue, 0.5, { space: "oklab" }));

The module does NOT copy color math into JavaScript. It sends typed numeric values
through the ABI into Rust WASM. WASM is **not** automatically wired into the
existing PFx Colors UI; integration of the UI is a separate migration step.
The JS API includes convert, difference, contrast, luminance, interpolate,
isInGamut, mapGamut, tonalPalette, rampPalette, harmony and createGradient.
The JS gradient returns an owned object; call dispose() when finished.

Node.js integration smoke tests are run against the actual built .wasm binary
using built-in node:test, not a mock or JavaScript reimplementation.

## Security and ABI guarantees

- These native ABI functions require valid aligned pointers; passing arbitrary
  addresses or reusing freed handles is undefined behavior and cannot be made
  safe with runtime checks. Public JavaScript APIs manage their own pointers.
- All hue, color space and mapping IDs are explicitly encoded by this ABI.
  Enums inside Rust are intentionally not exported with compiler-dependent layout.
- Invalid space IDs or numeric inputs return errors. No implicit gamut clipping.
- Handle constructors return null when validation fails. Palette and gradient
  handles are opaque and must be freed exactly once by the owner.
- C ABI handles are not synchronized; callers must serialize cross-thread access.
- The ABI revision is 1, but these bindings have not yet been published as a
  production release. Further platform portability and browser checks should
  precede a release.
