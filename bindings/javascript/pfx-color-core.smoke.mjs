// Actual WebAssembly tests. Only Node.js built-ins; no npm dependencies.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { createPfxColorCore } from "./pfx-color-core.mjs";

const bytes = await readFile(new URL(
  "../../target/wasm32-unknown-unknown/release/pfx_color_ffi.wasm",
  import.meta.url,
));
const api = await createPfxColorCore(bytes);
const color = (space, channels, alpha = 1) => ({ space, channels, alpha });
const black = color("srgb", [0, 0, 0]);
const white = color("srgb", [1, 1, 1]);

test("loads compiled Rust WASM without third-party JS dependencies", () => {
  assert.equal(typeof api.convert, "function");
  assert.equal(typeof api.harmony, "function");
  assert.equal(typeof api.createGradient, "function");
});

test("matches sRGB to OKLab numerical reference", () => {
  const result = api.convert(color("srgb", [0x33 / 255, 0x66 / 255, 0x99 / 255]), "oklab");
  const reference = [0.49931445584520834, -0.03304348760594705, -0.09296659206477714];
  result.channels.forEach((value, index) =>
    assert.ok(Math.abs(value - reference[index]) < 2e-11));
  assert.equal(result.space, "oklab");
  assert.equal(result.alpha, 1);
});

test("computes WCAG 21:1; rejects alpha and out-of-gamut inputs", () => {
  assert.ok(Math.abs(api.contrast(black, white) - 21) < 1e-12);
  assert.ok(Math.abs(api.luminance(white) - 1) < 1e-12);
  assert.throws(() => api.contrast(color("srgb", [0, 0, 0], 0.5), white));
  assert.throws(() => api.contrast(color("display-p3", [0, 1, 0]), white));
});

test("CIEDE2000 reference distance and explicit gamut checks", () => {
  assert.ok(Math.abs(api.difference(
    color("lab", [50, 2.6772, -79.7751]),
    color("lab", [50, 0, -82.7485]),
    "ciede2000",
  ) - 2.0425) < 0.00005);
  const p3 = color("display-p3", [0, 1, 0], 0.7);
  assert.equal(api.isInGamut(p3), false);
  const mapped = api.mapGamut(p3);
  assert.equal(api.isInGamut(mapped), true);
  assert.equal(mapped.alpha, 0.7);
});

test("preserves premultiplied alpha in Rust interpolation", () => {
  const mid = api.interpolate(color("srgb", [1, 0, 0], 0), color("srgb", [0, 0, 1]), 0.5, {
    space: "srgb",
  });
  assert.deepEqual(mid.channels, [0, 0, 1]);
  assert.equal(mid.alpha, 0.5);
});

test("produces typed palettes and named harmony from Rust", () => {
  const tonal = api.tonalPalette(color("oklch", [0.55, 0.3, 40]), { count: 7 });
  assert.equal(tonal.length, 7);
  assert.equal(tonal[0].position, 0);
  assert.equal(tonal[6].position, 1);
  assert.equal(tonal[3].color.space, "srgb");

  const ramp = api.rampPalette(black, white, { count: 3, space: "srgb" });
  assert.equal(ramp.length, 3);
  assert.deepEqual(ramp[1].color.channels, [0.5, 0.5, 0.5]);

  const harmony = api.harmony(color("oklch", [0.6, 0.08, 20]), "triadic");
  assert.equal(harmony.length, 3);
  assert.deepEqual(harmony.map((entry) => entry.hueOffset), [0, 120, 240]);
});

test("samples normalized gradients, and frees native handle safely", () => {
  const gradient = api.createGradient([
    { position: 0, color: black },
    { position: 1, color: white },
  ], { kind: "linear", angle: 90, space: "srgb", gamut: "clip" });
  assert.deepEqual(gradient.sample(0.5).channels, [0.5, 0.5, 0.5]);
  assert.deepEqual(gradient.sampleXY(1, 0.5).channels, [1, 1, 1]);
  gradient.dispose();
  gradient.dispose();
  assert.throws(() => gradient.sample(0.3), /disposed/);
});

test("rejects invalid inputs without silently masking errors", () => {
  assert.throws(() => api.convert(black, "fake"), /Unknown/);
  assert.throws(() => api.convert(color("srgb", [Number.NaN, 0, 0]), "lab"));
  assert.throws(() => api.tonalPalette(black, { count: 257 }), /count/);
  assert.throws(() => api.harmony(black, "custom"), /Unknown/);
  assert.throws(() => api.createGradient([{ position: 0, color: white }]), /2..256/);
  assert.throws(() => api.createGradient([
    { position: 0, color: white },
    { position: 2, color: black },
  ]), /status/);
  assert.throws(() => api.interpolate(black, white, -0.5), /status/);
  assert.throws(() => api.difference(black, white, "invalid"), /Unknown/);
});
