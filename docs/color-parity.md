# PFx Colors — verified compatibility and migration gate

**Audited baseline:** 2026-10-09 against [Rust engine commit `7b382b115de33a43845a5dea9409a57e68d2210f`](https://github.com/pfxamd/pfx-color-core/commit/7b382b115de33a43845a5dea9409a57e68d2210f).
**Status:** experimental independent Rust engine; **PFx Colors production UI still uses its pinned TypeScript engine**. The Rust API, C ABI, WASM bridge and optional JS tools are not a released drop-in replacement.

This is a code-and-test audit, **not** a claim of full CSS Color 4 conformance or feature parity. Reference implementation: `src/engine/`, `src/tools/`, `src/workspace/`. Rust: `crates/color-core/`; C/WASM: `crates/color-ffi/`; JS bindings: `bindings/javascript/`.

## Verified capability matrix

| Capability | Implemented in first-party Rust / interfaces | Evidence and limit for migration |
| --- | --- | --- |
| Numeric color conversion | sRGB/linear, Display-P3/linear, Rec.2020/linear, XYZ D65/D50, Lab D50, **Lab D65**, LCH, OKLab/OKLCH, HSL/HWB/HSV, Adobe RGB (A98), ProPhoto RGB | `spaces/mod.rs`, `conversion/mod.rs`; conversions and legacy parity tests. **Lab D65, A98 and ProPhoto are implemented**, not missing. |
| Perceptual differences | CIE76, CIEDE2000 and Delta-E OK | `difference/mod.rs` and test fixtures; ITP/Jz/HCT are not implemented. |
| WCAG contrast | WCAG 2.2 relative luminance/contrast for **opaque, in-sRGB-gamut** colors | `contrast/mod.rs`; bounded comparison with TypeScript. Not numerically identical at every threshold: do not silently replace production pass/fail behavior. |
| APCA contrast | **Not included in the independent Rust core** | APCA-specific code was removed from Rust on licensing review. The legacy TypeScript adapter remains responsible for APCA. |
| Alpha-aware interpolation | Rectangular and polar interpolation; shorter/longer/increasing/decreasing/**raw** hue paths; HSL/HWB/HSV hue index handling, powerless-hue behavior and premultiplied non-hue channels | `interpolation/mod.rs`, `tests/hue-paths.rs`, Rust-versus-real-Color.js HSL/OKLCH raw-hue parity tests. **Raw hue is implemented**. The opt-in `CssColor` API now carries explicit missing-component masks; the numeric API intentionally does not. |
| Gamut operations | Membership, clipping, Oklch chroma reduction and CSS Local MINDE | `gamut/mod.rs`; mapping remains explicit; not full browser render parity. |
| Palettes and design tools | Tonal/ramp/anchored palettes, named/custom harmonies, seeded 10-color Color Study | `palettes/`, `harmony/`, `study/`; native, C, WASM and targeted legacy parity checks. |
| Gradients | Ordered 2–256 stops, hard-stop behavior, linear/radial/conic models; progress and unit-square XY sampling | `gradients/mod.rs` and WASM interface. **Not pixel-equivalent to CSS gradients for arbitrary box ratios, shapes, positioning or rendering.** |
| CSS color input | HEX; legacy/modern RGB and HSL; HWB; Lab/LCH/OKLab/OKLCH; supported `color()` absolute spaces; 148 standard named colors plus `transparent` | `css/mod.rs`, `tests/css-compat.rs`, JS/legacy parity smoke tests. CSS `hsv()` is **not** accepted (HSV is a numeric space). Advanced grammar remains unsupported. |
| CSS missing values and limited CSS expressions (opt-in) | **Implemented for absolute modern color syntax** using separate `CssColor` with channel/alpha mask, Rust parsing/formatting, analogous-component and set carry-forward, premultiplied mixing, ordinary zero-valued numeric conversion, additive 48-byte C ABI and WASM JS methods | `css_missing.rs`, `css_expression.rs`, `tests/css-missing.rs`, `tests/css-expressions.rs`, `css_missing_ffi.rs`, `pfx-color-core.smoke.mjs`, native C and browser smoke. Existing numeric `parse_css` and picker/workspace intentionally still reject `none`. The opt-in parser additionally evaluates a **bounded subset** of typed `calc()` and relative colors; it does **not** implement external `var()`, full CSS Color 4 grammar, or production UI migration. |
| CSS output | Color strings and explicit-mapped HEX output; opt-in JS workspace serializes basic gradient definitions | `css/mod.rs` and `pfx-color-workspace.mjs`. Gradient-string smoke tests **do not prove browser rendering parity**. |
| Image palette extraction | **Implemented:** deterministic perceptual palette from caller-decoded, unpremultiplied RGBA8 pixel buffers; up to 32 colors, 64 MiB input and 500,000 samples, alpha/region/white filters | `images/mod.rs`, `tests/images.rs`, native C smoke, Node WASM and real browser ImageData tests. **Not** an encoded PNG/JPEG/WebP decoder, ICC pipeline, or byte/visual-exact ColorThief replacement. |
| Portable integration | First-party C ABI (v1), real Rust WASM wrapper with no third-party runtime packages; opt-in picker and JS workspace/history | `color-ffi/`, `pfx-color-core.mjs`, `pfx-color-tools.mjs`, `pfx-color-workspace.mjs`. Workspace state/history is handled in JS; color math is Rust. No production React swap. |
| Platform tests | Linux native Rust/C linking; Node WASM; headless Chromium and Firefox at desktop and mobile **viewport sizes** | CI on audited commit passed. These are not physical mobile-device tests or verified native Windows/macOS builds. |

## Confirmed remaining work — ordered migration gates

1. **CSS missing-component gate — initial opt-in API completed**: dedicated `CssColor` / `parse_css_missing` / `format_css_missing` / `interpolate_css_missing`, additive C and WASM functions. Remaining: broaden CSS syntax, strengthen conformance fixtures (especially automatic powerless channels and cross-space corner cases), and explicitly integrate/verify picker, workspace and gradient semantics. Numeric `Color` still deliberately cannot preserve missing fields.
2. **Advanced CSS grammar**: first-party `css_expression.rs` now implements typed arithmetic (`+`, `-`, `*`, `/`, nested `calc()`) and relative-origin channels for supported `rgb`, `hsl`, `hwb`, Lab/LCH, OKLab/OKLCH and `color()` spaces in the opt-in `parse_css_missing` API. Limits: 1024 ASCII bytes, bounded recursion, rejection of invalid mixed dimensions/nonfinite values; **no `var()`, `currentColor`, arbitrary CSS math functions, profiles, or fully general Color 4 expressions**. Legacy numeric `parse_css` remains unchanged.
3. **Specialized algorithms and spaces**: OKHSL/OKHSV, Delta-E ITP, Jz-family and HCT are absent. APCA requires an independently resolved licensing/integration decision; it must not be silently copied into Apache-2.0 Rust.
4. **Gradient semantics**: implement and test true CSS layout geometry (real box dimensions, radial sizing/shape, positioning, repeating behavior where relevant) and browser-rendered visual comparisons. Current unit-square sampling is correct only for its documented model.
5. **Image pipeline**: decoded RGBA8 clustering exists. Encoded-format decoding, embedded ICC color management and comparable legacy UI palette behavior remain outside the core. Establish actual visual acceptance fixtures before changing the image tool.
6. **Production integration**: independently test a feature-gated Rust adapter in the real React UI, picker interactions, undo/redo, palettes, gradients and image extraction. Check accessibility, fallback, loading, bundle/performance and browser-specific behavior before switching the default.
7. **Release/platform proof**: validate native Windows/macOS (if officially supported), publish and version the Rust/FFI API only after backward-compatibility and UI acceptance gates. Published `v0.1.0` is TypeScript only.

## Executed validation — precise scope

At the audit baseline, these GitHub Actions runs are green:

- [Rust Color Core CI](https://github.com/pfxamd/pfx-color-core/actions/runs/37953015854): Rust formatting/tests/clippy/build, native C linked executable, compiled WASM in Node, real TypeScript parity, optional picker/workspace smoke, and standalone Linux/WASM artifacts.
- [Rust WASM Browser Compatibility](https://github.com/pfxamd/pfx-color-core/actions/runs/37953015951): real compiled WASM in headless Chromium and Firefox, desktop and mobile **viewports**, including decoded-image palette extraction.
- [Color Core CI](https://github.com/pfxamd/pfx-color-core/actions/runs/37953015739): legacy TypeScript checks.

The original audit's three green workflow runs apply to `7b382b1`; later `none` implementation commits require their own green run results. they do **not** validate a deployed React UI migration, actual mobile devices, untested CSS Color 4 syntax, arbitrary ICC profiles, or native Windows/macOS.

The executable `bindings/javascript/legacy-parity.smoke.mjs` compares real Rust WASM with the repository's actual ColorJs adapter, with documented numeric tolerances. HSL/OKLCH raw hue was explicitly cross-checked with actual Color.js. A bounded WCAG contrast difference is allowed in the test suite: this is **not exact threshold equivalence**.

## Safe migration rule

Keep `pfxamd/pfx-colors` on its vendored TypeScript snapshot (`vendor/PFx-Color-Core/PINNED_REVISION`) until the missing gates are individually addressed and the **real application** passes acceptance tests. The independent Rust engine must retain zero external Cargo dependencies and color mathematics must not be duplicated in browser JavaScript. No automatic production cutover.
