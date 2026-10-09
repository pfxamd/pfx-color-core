# PFx Color Core

Independent, UI-free TypeScript color engine developed for **PFx Colors** and other PFx applications.

## Features

- Color parsing, conversion, contrast (WCAG 2.1 / APCA), perceptual difference, interpolation, gamut mapping
- Color picker/channel operations and alpha handling
- Tonal palettes, ramps, harmonies and structured color studies
- Linear, radial and conic gradient definitions, sampling and CSS serialization
- Reusable workspace with undo/redo and immutable state snapshots
- An optional image-palette extractor adapter contract

## Structure

- `src/engine/` — engine, adapter interfaces and color operations
- `src/tools/` — color-domain generators and transformations
- `src/workspace/` — UI-independent state with history
- `tests/` — regression tests

`src/` does **not** contain React or UI imports.

## Development

Requires Node.js 22+.

```bash
npm install
npm run check
npm run build
```

The build produces `dist/index.js` (ESM) and TypeScript declarations.

## Usage

```ts
import { colorEngine, generateHarmony, PfxColorsWorkspace } from "@pfxamd/color-core";
const oklch = colorEngine.color.convert("#336699", "oklch");
const harmony = generateHarmony("#336699", "triadic");
const workspace = new PfxColorsWorkspace("#336699");
workspace.generateTonalPalette({ count: 9 });
```

## Distribution into PFx Colors

PFx Colors keeps a **revision-pinned vendor snapshot** of this repo in `vendor/PFx-Color-Core/`, so its build is deterministic and never fetches a live dependency at runtime. New core releases can be copied into the app only after passing tests.

## Dependency policy

**v0.1.0 is a behavior-preserving extraction.** The color-computation adapter uses `colorjs.io` (0.7.1). The optional image-extraction adapter uses `colorthief` (3.5.0). Both are external runtime dependencies. A future, independent milestone can replace them with tested PFx-owned implementations. This repository does not currently claim zero external dependencies.

## Provenance

Extraction baseline: [PFx Colors commit `4c2b63735af4`](https://github.com/pfxamd/pfx-colors/commit/4c2b63735af4e6f684dc806201fd7f5967e1e229).

## License

Apache-2.0. Copyright 2026 PFxamd.

## Rust computational engine (in progress)

A new dependency-free Rust engine is being developed in [`crates/color-core/`](crates/color-core/). It currently provides the mathematical foundation and reference-tested color-space conversions. The published `v0.1.0` and the PFx Colors UI continue using the existing TypeScript implementation. See [Rust migration plan](docs/rust-migration.md).
