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
- sRGB, P3, Rec.2020, XYZ, Lab and OKLab conversion, plus HSL/HWB/HSV numeric controls;
- CSS absolute color parsing (with supported syntax) and coordinate/hex formatting;
- CIE76/CIEDE2000/OK differences, WCAG ratio and luminance;
- interpolation, gamut check and mapping (clip, radial Oklch or W3C Local MINDE);
- tonal, two-color ramp, and arbitrary anchored palettes;
- six named harmony schemes and custom hue offsets;
- multi-stop gradient builders with normalized sample and sampleXY.

The C and WebAssembly interfaces also support arbitrary anchored palettes and
custom hue-offset harmonies through opaque builders. Each builder has an
explicit destructor and owns its copied input data.

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
The JS API includes parseCss, formatCss, formatHex, convert, difference, contrast, luminance, interpolate,
isInGamut, mapGamut, tonalPalette, rampPalette, harmony and createGradient.
The JS gradient returns an owned object; call dispose() when finished.

Node.js integration smoke tests are run against the actual built .wasm binary
using built-in node:test, not a mock or JavaScript reimplementation.

## Decoded image palette extraction

The independent Rust core can extract up to 32 perceptually clustered
dominant colors from unpremultiplied, row-major RGBA8 pixels. The host must
decode compressed image files first: this is **not a PNG/JPEG/WebP decoder**
and is not a byte-identical port of ColorThief.

Input is capped at 64 MiB (16 million RGBA pixels), with a maximum of
500,000 sampled pixels. Options include stride, sample limit, alpha
threshold, near-white filtering and an optional rectangular region.
Transparent pixels below the configured threshold are not counted.
Color order is by descending sampled population and the result is
deterministic. C consumers own the input buffer and free the returned
opaque PfxImagePalette handle. A matching pfx_image_buffer_new / free pair
exists for WebAssembly uploads; original input pixels may be freed
immediately after pfx_image_new returns.

Browser example using decoded ImageData pixels:

    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const result = core.extractImagePalette(
      imageData.data, imageData.width, imageData.height,
      { count: 6, alphaThreshold: 128, maxSamples: 80000,
        ignoreNearWhite: true }
    );
    console.log(result.colors[0].color);
    console.log(result.colors[0].population, result.colors[0].proportion);

The JavaScript wrapper copies pixels into a temporary Rust-owned WASM
buffer, runs clustering wholly in Rust, collects copied palette metadata,
and frees both input and result allocations before returning.
Integration tests exercise real native C linking, Node.js WebAssembly,
and browser ImageData on Chromium and Firefox at desktop/mobile sizes.
The deployed PFx Colors image-extraction feature is **unchanged**.

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


## Absolute CSS parsing and unsupported input

The parser supports HEX, modern/legacy RGB and HSL, HWB, Lab, LCH, Oklab,
Oklch, the supported color() spaces and all 148 standard named-color keywords.
Its grammar is bounded to 1024 bytes. Invalid syntax, missing-channel none,
calc(), var(), and relative color syntax are **rejected**, not approximated.

The C buffer API uses pfx_buffer_new and pfx_buffer_free to provide a safe
owner-managed byte region for WebAssembly consumers. pfx_css_parse accepts
UTF-8 bytes; pfx_css_format writes UTF-8 plus a final NUL and reports bytes
written (excluding terminator). Caller pointers must be valid and the buffer
capacity must exceed the returned byte count.

The JS interface handles byte ownership, UTF-8 encoding and decoding. All
numerical color operations occur in Rust. Example:

    const picked = core.parseCss("hsl(210 65% 55% / 75%)");
    const canonical = core.formatCss(picked);
    const hex = core.formatHex(picked, "css");

Full parser compatibility, CSS missing-component modeling, source-accurate
serialization of unsupported forms, and UI integration remain migration gates.


## Seeded Color Study

The first-party Rust engine contains the PFx Color Study ten-swatch generator.
Unlike the previous browser feature, its API accepts a **deterministic u32
random seed**; the wrapper exposes this as colorStudy(seedColor, options), with
lightness, chroma, hueRange, toneRange, target, gamut and randomSeed options.
It returns {scheme, colors}; each color contains {index, color, mapped, oklch}.
No external random-number or palette dependencies are used. The existing
browser Color Study is not changed or replaced by these bindings yet.


## Verified standalone build bundles

After the full Rust CI workflow succeeds, the workflow attaches two downloadable
artifacts on its GitHub Actions run page (retained for 30 days):

- pfx-color-core-wasm-{commit}: the real compiled .wasm, its JavaScript wrapper,
  Apache-2.0 license and integration documentation.
- pfx-color-core-linux-{commit}: native .so and .a, matching C header,
  license and documentation.

These are build outputs, not dynamic online dependencies. Copy the relevant
compiled library and accompanying interface into the destination project;
it should not need the PFx Git repository at runtime.

The Rust CI compiles and tests the native program, WebAssembly JavaScript
integration, scientific fixtures, deterministic Color Study and cross-engine
compatibility before bundling. A successful Linux binary build alone does
not claim validation on macOS or Windows; those native targets need their
own compiler and integration runs.

## Opt-in PFx Colors picker bridge

The optional `bindings/javascript/pfx-color-tools.mjs` module wraps the Rust WASM API with the original high-level picker operations (`selectColor`, `setColorChannel`, `setColorAlpha`, `mapColorToGamut`). It accepts CSS strings and numeric color inputs, and returns familiar `hex`, `source`, `values`, `alpha` and `gamut` fields. It maps the legacy `p3` identifier to the Rust `display-p3` identifier, and calls Rust for all parsing, conversion, gamut mapping and formatting. Unsupported CSS `none`/missing channels are explicitly rejected. This bridge remains opt-in and is not imported by the deployed app.

After initialising the core with `createPfxColorCore(wasmBytes)`, import `createPfxColorTools` and call `const tools = createPfxColorTools(core);` followed by `tools.selectColor("#336699")`.

Regression tests compare the bridge against the real TypeScript implementation, using the compiled Rust WASM build: `node --test bindings/javascript/pfx-color-tools.smoke.mjs`.

## Opt-in workspace state and history

An optional first-party workspace orchestration layer lives in `bindings/javascript/pfx-color-workspace.mjs`. `createPfxColorsWorkspace(core, initialColor, historyLimit)` returns methods corresponding to the current UI workspace (`getState`, `setColor`, `generateTonalPalette`, `generateRampPalette`, `generateHarmony`, `generatePaletteFromHarmony`, `createGradient`, `createGradientFromPalette`, `createGradientFromHarmony`, `setColorFromPalette`, `setColorFromHarmony`, `setColorFromGradient`, `undo`, `redo`, `clearHistory`). Numeric color calculations and parsing are delegated to Rust WASM. History is copied and managed in JavaScript; states are independent snapshots. The module is not imported by the production website. CSS gradient string serialization, CSS missing channels, advanced color spaces, full browser parity and image extraction remain separate work.

## Browser verification

The separate `Rust WASM Browser Compatibility` CI workflow compiles the first-party Rust library to `wasm32-unknown-unknown` and runs `scripts/browser-wasm-smoke.mjs` using Chromium and Firefox at 1440×900 and 390×844 viewports. Playwright is installed only for GitHub Actions testing. It is **not** bundled with the WebAssembly module, runtime JavaScript bindings or existing PFx Colors website. This exercise verifies the standalone and opt-in runtime, not yet the production React UI.
