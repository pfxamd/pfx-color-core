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
  hsl: 0.005,
  hwb: 0.005,
  hsv: 0.005,
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
    const hue = (i === 2 && (space === "lch" || space === "oklch")) ||
      (i === 0 && (space === "hsl" || space === "hsv" || space === "hwb"));
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

test("WCAG contrast stays bounded; legacy and standard coefficients differ", () => {
  // WCAG 2.2 defines the sRGB luminance factors 0.2126, 0.7152, 0.0722.
  // The legacy Color.js adapter instead computes luminance via its higher-
  // precision sRGB->XYZ matrix. Its ratio is NOT exactly the WCAG factor
  // result, so exact parity would compromise the standard-compliant core.
  const opaque = [srgb([0, 0, 0]), srgb([1, 1, 1]), ...samples.map(v => ({ ...v, alpha: 1 }))];
  let largestDifference = 0;
  let compared = 0;
  for (const a of opaque) {
    for (const b of opaque) {
      if (a.space === "display-p3" || b.space === "display-p3") continue;
      const rustRatio = rust.contrast(a, b);
      const tsRatio = legacy.contrast(ts(a), ts(b), "wcag21").value;
      largestDifference = Math.max(largestDifference, Math.abs(rustRatio - tsRatio));
      compared += 1;
    }
  }
  console.log("WCAG baseline: " + compared + " pairs; max legacy-standard delta = " + largestDifference);
  assert.ok(
    largestDifference < 0.05,
    "WCAG ratio divergence exceeds documented compatibility bound: " + largestDifference,
  );
  const white = srgb([1, 1, 1]);
  const black = srgb([0, 0, 0]);
  assert.ok(Math.abs(rust.contrast(white, black) - 21) < 1e-12);
  assert.ok(Math.abs(rust.contrast(black, srgb([1, 0, 0])) - 5.252) < 1e-12);
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


test("CSS absolute colors parsed by Rust agree with the legacy adapter", () => {
  const inputs = [
    "#336699", "#ff7a0066", "#f06", "transparent", "aliceblue", "goldenrod", "rebeccapurple",
    "rgb(255 0 0 / 50%)", "rgba(20, 70, 140, 0.4)",
    "hsl(140 55% 45%)", "hsla(20, 65%, 37%, .8)",
    "hwb(250 15% 20% / .9)",
    "lab(65% 20 -35)", "lch(65% 35 220)",
    "oklab(55% .08 -.04)", "oklch(64% .11 200 / .8)",
    "color(display-p3 0.2 0.4 0.6 / .6)",
  ];
  for (const css of inputs) {
    const result = rust.parseCss(css);
    const old = legacy.parse(css);
    const resultSpace = result.space;
    if (css === "transparent") {
      assert.equal(result.alpha, 0);
      assert.equal(old.alpha, 0);
      continue;
    }
    compare(result, old, resultSpace, css);
  }
});

test("HSL, HSV and HWB numeric conversions agree with legacy color models", () => {
  const seeds = [srgb([0.2, 0.4, 0.6]), srgb([0.8, 0.3, 0.15])];
  for (const seed of seeds) {
    for (const target of ["hsl", "hsv", "hwb"]) {
      compare(rust.convert(seed, target), legacy.convert(ts(seed), target), target, "UI space " + target);
    }
  }
});


test("CSS gamut reduction stays perceptually close to legacy CSS mapping", () => {
  for (const seed of [
    { space: "display-p3", channels: [0, 1, 0], alpha: 1 },
    { space: "oklch", channels: [0.7, 0.31, 30], alpha: 1 },
    { space: "oklch", channels: [0.6, 0.32, 255], alpha: 1 },
    { space: "srgb", channels: [1.1, -0.2, 0.4], alpha: 1 },
  ]) {
    const rustResult = rust.mapGamut(seed, "srgb", "css");
    const oldResult = legacy.mapToGamut(ts(seed), { targetSpace: "srgb", method: "css" });
    const oldColor = { space: oldResult.space, channels: oldResult.coordinates, alpha: oldResult.alpha };
    const delta = rust.difference(rustResult, oldColor, "ok");
    assert.ok(delta < 0.05,
      "CSS gamut mapping discrepancy from legacy exceeds 0.05 DeltaEOK: " + delta);
    assert.equal(rust.isInGamut(rustResult, "srgb"), true);
  }
});


test("all 148 CSS named color keywords match legacy and canonical HEX", async () => {
  const reference = JSON.parse(await readFile(
    new URL("./css-names.fixture.json", import.meta.url), "utf8"));
  assert.equal(Object.keys(reference).length, 148);
  for (const [name, hex] of Object.entries(reference)) {
    const parsed = rust.parseCss(name);
    assert.equal(parsed.space, "srgb", name);
    assert.equal(rust.formatHex(parsed), hex, name);
    assert.equal(legacy.formatHex(name).toLowerCase(), hex, "legacy " + name);
  }
});
