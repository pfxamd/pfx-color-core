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


test("Rust CSS parser and formatter handle HEX, modern RGB, HSL, HWB and P3", () => {
  const hex = api.parseCss("#33669980");
  assert.equal(hex.space, "srgb");
  assert.equal(api.formatHex(hex), "#33669980");

  const red = api.parseCss("rgb(255 0 0 / 50%)");
  assert.deepEqual(red.channels, [1, 0, 0]);
  assert.equal(red.alpha, 0.5);

  const green = api.parseCss("hsl(120 100% 50%)");
  assert.equal(green.space, "hsl");
  const g = api.convert(green, "srgb");
  assert.ok(Math.abs(g.channels[1] - 1) < 1e-10);
  assert.equal(api.formatHex(green), "#00ff00");

  const blue = api.parseCss("hwb(240 0% 0%)");
  assert.equal(blue.space, "hwb");
  assert.equal(api.formatHex(blue), "#0000ff");

  const p3 = api.parseCss("color(display-p3 0.2 0.4 0.7 / .75)");
  assert.equal(p3.space, "display-p3");
  assert.equal(p3.alpha, 0.75);
  const serialized = api.formatCss(p3);
  assert.equal(api.parseCss(serialized).space, "display-p3");

  const ok = api.parseCss("oklch(64% 0.16 180deg / .4)");
  assert.equal(ok.space, "oklch");
  assert.equal(ok.channels[0], 0.64);
  assert.equal(ok.alpha, 0.4);
  assert.equal(api.formatHex(api.parseCss("transparent")), "#00000000");
});

test("Rust CSS parsing refuses unsupported grammar and invalid UTF-8-safe inputs", () => {
  for (const value of [
    "rgb(none 0 0)", "rgb(var(--r) 0 0)", "rgb(255 1)",
    "hsl(240 100 50)", "color(prophoto-rgb none 0.2 0.3)",
  ]) {
    assert.throws(() => api.parseCss(value), /CSS parse/);
  }
  assert.throws(() => api.parseCss(""), /1..1024/);
  assert.throws(() => api.parseCss("a".repeat(1025)), /1..1024/);
  assert.throws(() => api.formatCss(color("srgb", [Number.NaN, 0, 0])), /finite/);
});


test("CSS Local MINDE gamut mapping preserves gamut and alpha", () => {
  for (const wide of [
    color("display-p3", [0, 1, 0], 0.6),
    color("oklch", [0.75, 0.38, 40]),
    color("srgb", [1.2, -0.2, 0.5]),
  ]) {
    const mapped = api.mapGamut(wide, "srgb", "css");
    assert.ok(mapped.channels.every(v => v >= 0 && v <= 1));
    assert.equal(mapped.alpha, wide.alpha);
  }
});


test("Color Study generated by real WASM is seed-deterministic and bounded", () => {
  const seed = color("srgb", [0.2, 0.4, 0.6], 0.7);
  const opts = { randomSeed: 29381, lightness: 58, chroma: 58, hueRange: 58, toneRange: 58 };
  const first = api.colorStudy(seed, opts);
  const second = api.colorStudy(seed, opts);
  assert.deepEqual(first, second);
  assert.equal(first.colors.length, 10);
  for (const [index, item] of first.colors.entries()) {
    assert.equal(item.index, index);
    assert.equal(item.color.alpha, 0.7);
    assert.equal(item.oklch.length, 3);
    assert.ok(item.color.channels.every((v) => v >= 0 && v <= 1));
  }
  assert.throws(() => api.colorStudy(seed, { randomSeed: -1 }), /randomSeed/);
  assert.throws(() => api.colorStudy(seed, { lightness: Number.NaN }), /finite/);
});


test("HSL/HWB/HSV in-gamut checks use sRGB, including wide-gamut input", () => {
  const p3 = color("display-p3", [0, 1, 0], 0.8);
  for (const target of ["hsl", "hwb", "hsv"]) {
    assert.equal(api.isInGamut(p3, target), false);
    const mapped = api.mapGamut(p3, target, "css");
    assert.equal(mapped.space, target);
    assert.equal(mapped.alpha, 0.8);
    assert.equal(api.isInGamut(mapped, target), true);
  }
});

test("WASM anchoredPalette preserves custom middle anchor and metadata", () => {
  const red = color("srgb", [1, 0, 0]);
  const green = color("srgb", [0, 1, 0]);
  const blue = color("srgb", [0, 0, 1]);
  const palette = api.anchoredPalette([red, green, blue], {
    count: 5, space: "srgb", gamut: "clip",
  });
  assert.equal(palette.length, 5);
  assert.deepEqual(palette[2].color.channels, green.channels);
  assert.equal(palette[2].position, 0.5);
  assert.deepEqual(palette[0].color.channels, red.channels);
  assert.deepEqual(palette[4].color.channels, blue.channels);
  const stepped = api.steps(red, blue, 3, { space: "srgb" });
  assert.deepEqual(stepped[1].channels, [0.5, 0, 0.5]);
  assert.throws(() => api.anchoredPalette([red], { count: 5 }), /2..256/);
  assert.throws(() => api.anchoredPalette([red, green, blue], { count: 2 }), /count/);
});

test("WASM customHarmony handles arbitrary hue offsets without dependencies", () => {
  const seed = color("oklch", [0.6, 0.06, 350], 0.7);
  const output = api.customHarmony(seed, [-45, 0, 90], { target: "oklch" });
  assert.equal(output.length, 3);
  assert.deepEqual(output.map(entry => entry.hueOffset), [-45, 0, 90]);
  const expected = [305, 350, 80];
  output.forEach((entry, i) => {
    assert.ok(Math.abs(entry.color.channels[2] - expected[i]) < 1e-12);
    assert.equal(entry.color.alpha, 0.7);
    assert.equal(entry.mapped, false);
  });
  assert.throws(() => api.customHarmony(seed, [20]), /2..256/);
  assert.throws(() => api.customHarmony(seed, [0, Number.NaN]), /finite/);
});


test("real Rust WASM extracts perceptual palette from decoded RGBA8", () => {
  const rgba = new Uint8ClampedArray([
    ...Array(8).fill([255, 0, 0, 255]).flat(),
    ...Array(4).fill([0, 255, 0, 255]).flat(),
    ...Array(4).fill([0, 0, 255, 255]).flat(),
  ]);
  const result = api.extractImagePalette(rgba, 4, 4, { count: 3 });
  assert.equal(result.sampledPixels, 16);
  assert.equal(result.eligiblePixels, 16);
  assert.equal(result.colors.length, 3);
  assert.deepEqual(result.colors.map(e => e.population), [8, 4, 4]);
  assert.deepEqual(result.colors[0].color.channels, [1, 0, 0]);
  assert.equal(result.colors[0].color.alpha, 1);
  assert.ok(Math.abs(result.colors.reduce((s, c) => s + c.proportion, 0) - 1) < 1e-12);
  const repeat = api.extractImagePalette(rgba, 4, 4, { count: 3 });
  assert.deepEqual(repeat, result);
});

test("Rust image extraction handles transparency, white filtering and image regions", () => {
  const rgba = new Uint8Array([
    255, 0, 0, 255, 255, 255, 255, 255,
    0, 0, 255, 0, 0, 255, 0, 100,
  ]);
  const selected = api.extractImagePalette(rgba, 4, 1, {
    count: 3, ignoreNearWhite: true, alphaThreshold: 128,
  });
  assert.equal(selected.sampledPixels, 4);
  assert.equal(selected.eligiblePixels, 1);
  assert.deepEqual(selected.colors[0].color.channels, [1, 0, 0]);
  assert.deepEqual(api.extractImagePalette(rgba, 4, 1, {
    count: 3, region: { x: 0, y: 0, width: 1, height: 1 },
  }).colors[0].color.channels, [1, 0, 0]);
  assert.throws(() => api.extractImagePalette(rgba, 4, 1, {
    alphaThreshold: 255, ignoreNearWhite: true, region: { x: 1, y: 0, width: 1, height: 1 },
  }), /no eligible pixels/);
});

test("Rust image input validation rejects unsafe or malformed memory shapes", () => {
  const tiny = new Uint8Array([255, 0, 0, 255]);
  assert.throws(() => api.extractImagePalette(tiny, 2, 2), /RGBA8/);
  assert.throws(() => api.extractImagePalette(tiny, 0, 1), /width/);
  assert.throws(() => api.extractImagePalette(tiny, 1, 1, { count: 33 }), /count/);
  assert.throws(() => api.extractImagePalette(tiny, 1, 1, { stride: 0 }), /stride/);
  assert.throws(() => api.extractImagePalette(tiny, 1, 1, {
    region: { x: 1, y: 0, width: 1, height: 1 },
  }), /region.x/);
  assert.throws(() => api.extractImagePalette(tiny, 1, 1, { alphaThreshold: 256 }), /alphaThreshold/);
  assert.throws(() => api.extractImagePalette(tiny, 1, 1, { maxSamples: 500001 }), /maxSamples/);
  assert.throws(() => api.extractImagePalette(tiny, 9000, 9000), /64 MiB/);
});
