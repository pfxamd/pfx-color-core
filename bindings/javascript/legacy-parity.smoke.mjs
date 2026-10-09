/**
 * Migration gate: compare the REAL TypeScript v0.1.0 engine (Color.js adapter)
 * against the REAL Rust engine compiled to WebAssembly.
 *
 * This deliberately does not replace either engine or modify the PFx Colors UI.
 * The tested intersection is limited to shared, well-defined numeric behavior.
 * Unsupported API areas are explicitly tracked in docs/color-parity.md.
 */
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import {
  ColorJsAdapter,
  createGradient as legacyGradient,
  generateHarmony as legacyHarmony,
  generateTonalPalette as legacyTonal,
  sampleGradient as legacyGradientSample,
} from "../../dist/index.js";
import { createPfxColorCore } from "./pfx-color-core.mjs";

const wasmBytes = await readFile(new URL(
  "../../target/wasm32-unknown-unknown/release/pfx_color_ffi.wasm",
  import.meta.url,
));
const rust = await createPfxColorCore(wasmBytes);
const legacy = new ColorJsAdapter();
const srgb = (channels, alpha = 1) => ({ space: "srgb", channels, alpha });
const ts = (input) => ({
  space: input.space === "display-p3" ? "p3" : input.space,
  coordinates: input.channels,
  alpha: input.alpha,
});
const jsSpace = (space) => space === "display-p3" ? "p3" : space;
const epsilon = {
  srgb: 0.0003,
  "srgb-linear": 0.0003,
  "display-p3": 0.0003,
  rec2020: 0.0003,
  "xyz-d65": 0.0003,
  "xyz-d50": 0.0003,
  lab: 0.06,
  lch: 0.06,
  oklab: 0.0005,
  oklch: 0.0005,
};

function compare(actual, expected, space, label) {
  const tolerance = epsilon[space];
  assert.ok(tolerance !== undefined, "No tolerance for " + space);
  assert.equal(actual.channels.length, 3);
  assert.equal(expected.coordinates.length, 3);
  for (let i = 0; i < 3; i += 1) {
    const a = actual.channels[i];
    const b = expected.coordinates[i];
    if (b === null) {
      // Legacy CSS Color models an undefined polar hue as null, while the
      // Rust numeric-only API currently uses 0 degrees.
      assert.ok(
        i === 2 && (space === "lch" || space === "oklch"),
        "Null only supported for powerless polar hue, at " + label,
      );
      continue;
    }
    const hue = i === 2 && (space === "lch" || space === "oklch");
    const difference = hue ?
      Math.abs((((a - b) + 540) % 360) - 180) :
      Math.abs(a - b);
    const allowed = hue ? 0.1 : tolerance;
    assert.ok(
      Number.isFinite(a) && difference < allowed,
      label + " " + space + "[" + i + "]: Rust " + a + ", TS " + b +
      ", difference " + difference + " >= " + allowed,
    );
  }
  assert.ok(Math.abs(actual.alpha - expected.alpha) < 1e-12, label + " alpha");
}

const samples = [
  srgb([0.2, 0.4, 0.6]),
  srgb([1, 0, 0]),
  srgb([0.9, 0.6, 0.1], 0.4),
  srgb([0.03, 0.08, 0.22]),
  srgb([0.7, 0.3, 0.85]),
  { space: "display-p3", channels: [0.23, 0.42, 0.71], alpha: 0.8 },
];

test("shared color-space conversions agree for chromatic reference samples", () => {
  for (const sample of samples) {
    for (const target of Object.keys(epsilon)) {
      const rustResult = rust.convert(sample, target);
      const tsResult = legacy.convert(ts(sample), jsSpace(target));
      compare(rustResult, tsResult, target, JSON.stringify(sample));
    }
  }
});

test("CIE76, CIEDE2000 and OK differences agree on shared inputs", () => {
  const pairs = [
    [samples[0], samples[1]],
    [samples[0], samples[2]],
    [samples[1], samples[3]],
    [samples[3], samples[4]],
    [samples[4], samples[5]],
  ];
  for (const [a, b] of pairs) {
    for (const [rustMethod, tsMethod] of [
      ["cie76", "76"],
      ["ciede2000", "2000"],
      ["ok", "ok"],
    ]) {
      const r = rust.difference(a, b, rustMethod);
      const t = legacy.difference(ts(a), ts(b), tsMethod).value;
      const allowed = rustMethod === "ok" ? 0.0006 : 0.04;
      assert.ok(
        Math.abs(r - t) < allowed,
        rustMethod + " Rust " + r + " TS " + t + " diff " + Math.abs(r - t),
      );
    }
  }
});

test("opaque WCAG luminance and contrast remain compatible", () => {
  const opaque = [srgb([0, 0, 0]), srgb([1, 1, 1]), ...samples.map(v => ({...v, alpha: 1}))];
  for (const a of opaque) {
    for (const b of opaque) {
      if (a.space === "display-p3" || b.space === "display-p3") continue;
      const rustRatio = rust.contrast(a, b);
      const tsRatio = legacy.contrast(ts(a), ts(b), "wcag21").value;
      assert.ok(Math.abs(rustRatio - tsRatio) < 0.0002, "WCAG " + rustRatio + " vs " + tsRatio);
    }
  }
});

test("selected in-gamut interpolation matches when space and hue path are explicit", () => {
  const a = srgb([0.25, 0.45, 0.60]);
  const b = srgb([0.78, 0.3, 0.25]);
  for (const space of ["srgb", "oklab", "oklch"]) {
    for (const fraction of [0, 0.2, 0.5, 0.8, 1]) {
      const result = rust.interpolate(a, b, fraction, { space, hue: "shorter" });
      const reference = legacy.interpolate(ts(a), ts(b), fraction, {
        space, hue: "shorter", outputSpace: space,
      });
      compare(result, reference, space, "interpolate " + fraction);
    }
  }
});

test("gamut-membership results match away from boundaries", () => {
  for (const sample of [
    srgb([0.1, 0.2, 0.3]),
    srgb([1.15, 0.1, 0.1]),
    { space: "display-p3", channels: [0, 1, 0], alpha: 1 },
    { space: "display-p3", channels: [0.4, 0.7, 0.3], alpha: 1 },
  ]) {
    for (const target of ["srgb", "display-p3"]) {
      const a = rust.isInGamut(sample, target);
      const b = legacy.isInGamut(ts(sample), jsSpace(target));
      assert.equal(a, b, JSON.stringify(sample) + " target " + target);
    }
  }
});

test("unmapped neutral tonal palettes match legacy values and step order", () => {
  const seed = { space: "oklch", channels: [0.55, 0, 0], alpha: 1 };
  const a = rust.tonalPalette(seed, { count: 7, minLightness: 0.2, maxLightness: 0.85 });
  const b = legacyTonal(ts(seed), { count: 7, minLightness: 0.2, maxLightness: 0.85 });
  assert.equal(a.length, b.colors.length);
  for (let i = 0; i < a.length; i += 1) {
    assert.equal(a[i].index, b.colors[i].index);
    assert.equal(a[i].position, b.colors[i].position);
    assert.equal(a[i].mapped, b.colors[i].mapped);
    compare(a[i].color, b.colors[i].value, "srgb", "tonal " + i);
  }
});

test("named harmony angles and neutral small-chroma results match", () => {
  const seed = { space: "oklch", channels: [0.6, 0.02, 35], alpha: 1 };
  for (const [rustScheme, legacyScheme] of [
    ["complementary", "complementary"], ["triadic", "triadic"],
    ["analogous", "analogous"], ["square", "square"],
  ]) {
    const r = rust.harmony(seed, rustScheme);
    const t = legacyHarmony(ts(seed), legacyScheme);
    assert.equal(r.length, t.colors.length);
    for (let i = 0; i < r.length; i += 1) {
      assert.equal(r[i].hueOffset, t.colors[i].hueOffset);
      assert.equal(r[i].mapped, t.colors[i].mapped);
      compare(r[i].color, t.colors[i].value, "srgb", legacyScheme + " " + i);
    }
  }
});

test("multi-stop gradient samples agree in shared, in-gamut rectangular space", () => {
  const colors = [srgb([0.2, 0.4, 0.6]), srgb([0.35, 0.55, 0.35]), srgb([0.7, 0.3, 0.25])];
  const inputs = [0, 0.5, 1].map((position, i) => ({ position, color: colors[i] }));
  const rustGradient = rust.createGradient(inputs, { space: "srgb", gamut: "clip" });
  try {
    const tsGradient = legacyGradient(
      inputs.map(s => ({ position: s.position, color: ts(s.color) })),
      { interpolationSpace: "srgb", targetSpace: "srgb" },
    );
    for (const position of [0, 0.1, 0.25, 0.5, 0.75, 0.9, 1]) {
      const a = rustGradient.sample(position);
      const b = legacyGradientSample(tsGradient, position);
      compare(a, b, "srgb", "gradient " + position);
    }
  } finally {
    rustGradient.dispose();
  }
});
