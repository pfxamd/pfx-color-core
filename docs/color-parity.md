# PFx Colors migration compatibility gate

Status: **experimental — production engine is unchanged**.

This report is an interface-level audit of:
- Rust science engine: crates/color-core/
- Native/WASM bridge: crates/color-ffi/ and bindings/javascript/
- Legacy TypeScript: src/engine/, src/tools/, src/workspace/
- Deployed UI integration: pfxamd/pfx-colors/src/app/app.tsx
- Production consumer pins a vendored TypeScript core at its own revision.

The executable numerical gate is bindings/javascript/legacy-parity.smoke.mjs.
It imports the actual ColorJsAdapter compiled from the repository AND the actual
pfx_color_ffi.wasm; **no mock calculations**. This comparison fixture uses
explicitly mapped color-space identifiers (\`p3\` versus \`display-p3\`) and
numeric color inputs. Tests restrict semantic comparison to operations defined
in both implementations. Passing the gate does **not** imply parity elsewhere. WCAG is tested as a separately identified, **bounded deviation**, not exact numerical parity.

## Coverage matrix

| Legacy feature | Rust status | Migration decision |
| --- | --- | --- |
| Numeric sRGB, linear sRGB, Display-P3, Rec.2020, XYZ, Lab, LCH, OKLab, OKLCH conversions | Present, executable parity gate | Candidate for opt-in numeric API only |
| CIE76, CIEDE2000, OK difference | Present, executable parity gate | Candidate for opt-in numeric API only |
| WCAG contrast of opaque in-gamut colors | Standard-conformant Rust calculation; legacy coefficient difference quantified in parity suite | **Not exact parity**; keep current UI until display/threshold behavior is explicitly accepted |
| Alpha-aware interpolation (shared numeric spaces) | Present, executable parity gate | Candidate for opt-in numeric API only |
| RGB gamut membership | Present, executable parity gate | Candidate for opt-in numeric API only |
| Tonal palettes, named harmonies and multi-stop gradients (shared cases) | Present, executable parity gate | Candidate for opt-in numeric API only |
| CSS absolute color parsing: HEX, rgb(), hsl(), hwb(), lab(), lch(), oklab(), oklch(), color() and 148 named colors | Implemented subset; 148-name full parity gate | Advanced CSS forms still block direct picker/workspace swap |
| CSS serialization and hex output | Implemented for the supported numeric forms, with explicit gamut mapping | Precision/missing-value semantics must still be checked |
| HSL / HSV / HWB | Implemented and cross-checked in numeric parity tests | Eligible for future opt-in adapter |
| Adobe RGB (1998) and ProPhoto RGB | W3C matrix/transfer implementations and native/WASM/CSS support; reference and legacy comparison fixtures added | Candidate for opt-in numeric API after complete browser parity |
| OKHSL / OKHSV, Lab-D65 | Missing | Keep TypeScript for these conversions |
| APCA contrast | Not bundled into the independent Apache-2.0 Rust engine due to separate upstream licensing | Keep TypeScript adapter for APCA-specific calculations |
| DeltaE ITP / Jz / HCT | Missing | Keep TypeScript |
| CSS Local-MINDE gamut mapping | W3C binary search with local MINDE added and cross-checked | Parity uses perceptual tolerance; keep old UI until threshold cases pass |
| CSS missing/none channel semantics, hue raw path | Missing | Keep TypeScript |
| Full CSS-compatible gradient serialization/geometry | Missing: Rust normalized unit square | Keep TypeScript |
| Arbitrary anchored palettes/custom harmony in WASM ABI | Implemented in Rust, C ABI and WASM with owned builders | Eligible for parity checking; no production UI switch |
| Seeded 10-color Color Study | Ported to Rust with first-party deterministic PRNG; seeded WASM-vs-TypeScript perceptual parity tests passed | Keep TypeScript app until browser/UI integration tests pass |
| Color-picker selection, channel editing and normalized hex | Opt-in Rust-backed JavaScript adapter with real baseline comparisons | Test independently; keep production UI on TypeScript until browser parity |
| Workspace mutations and undo/redo | Opt-in Rust-powered JS workspace implemented with regression tests | Keep deployed UI unchanged until browser-level tests and full feature parity |
| Optional image extraction | Original deterministic Rust RGBA8 palette extractor, with C ABI and JavaScript/WASM bindings | Not a byte-exact replacement for ColorThief; continue legacy integration pending visual/UI parity |

## Transition principles

1. Never switch PFx Colors' default engine based on a small numeric parity gate.
2. Keep the published UI and its vendored TypeScript dependency untouched.
3. Run the native Rust, C smoke, WASM smoke, TypeScript and parity CI gates.
4. Once numeric parity passes, future work can introduce a **separate opt-in
   adapter**, with explicit input compatibility and fallback only in the legacy
   layer. The Rust core remains completely free of third-party crates.
5. Finish advanced parsing/formatting, missing channels, additional spaces, workspace
   and Color Study with independent tests before removing the TypeScript path.
6. Browser-level UI parity and real device regression tests must precede any
   production default switch.

## Test tolerances

W3C chromaticity and white-point constants differ slightly across libraries.
Parity tests compare floating-point coordinates with **declared** tolerances
rather than requiring byte-exact equality. For LCH/OKLCH hues, comparisons use
circular angular distance; null / powerless hue is tracked separately.

WCAG coefficients: Rust implements WCAG 2.2 specified sRGB luminance coefficients (0.2126 / 0.7152 / 0.0722), whereas the legacy Color.js adapter yields slightly different values through its XYZ matrix. The suite reports the largest observed ratio difference and rejects deviations >= 0.05 over its defined fixtures. Do not use the tolerance to claim exact parity or interchangeability near a compliance threshold; preserve the old UI calculation until the migration policy is agreed.

Additional wide-gamut coverage: Adobe RGB (1998) uses a D65 reference and exact rational matrices; ProPhoto uses D50 with Bradford adaptation and its piecewise gamma toe. Rust, C ABI and WASM maintain distinct numeric identifiers; color() CSS parsing/serialization supports both without changing the original user input. Reference: W3C CSS Color 4, October 2026.

These are smoke/regression gates, not a substitute for official numerical
reference datasets, browser testing or a full format parser conformance suite.

## Browser runtime gate

A separate first-party test script, `scripts/browser-wasm-smoke.mjs`, executes the **real compiled Rust WebAssembly** in headless Chromium and Firefox at desktop and mobile viewport sizes. It checks color parsing, conversion, alpha-aware mixing, anchored palettes, custom harmonies, deterministic Color Study, picker channel editing, workspace history, and CSS gradient string formatting. The `Rust WASM Browser Compatibility` workflow installs Playwright **only as CI test infrastructure**, never as a dependency of the Rust engine or its browser-facing runtime. This test does not exercise the deployed React application or imply full CSS Color 4 conformance.
