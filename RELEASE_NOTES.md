# PFx Color Core v0.1.0

First standalone release of the color-computation core extracted from PFx Colors.

### Included
- Color input, parsing and conversion across supported color spaces
- WCAG 2.1 and APCA contrast, perceptual difference algorithms, interpolation, gamut handling
- Tonal palettes, ramps, harmonies and seeded color studies
- Linear, radial and conic gradients with stop sampling and CSS export
- UI-independent workspace state, undo and redo
- Public TypeScript exports and regression tests

### Compatibility
- Node.js 22+ for development/build
- ESM and TypeScript declarations in the release package
- Designed for deterministic vendoring by PFx Colors; no React dependency in the core

### Dependency notice
The initial extraction deliberately preserves the original algorithms via **colorjs.io 0.7.1**. The optional image extraction adapter uses **colorthief 3.5.0**. This release is **not** a zero-dependency or fully PFx-native calculation implementation.

### Validation
GitHub Actions tests, TypeScript validation and package build must pass before the release is published.

License: Apache-2.0.
