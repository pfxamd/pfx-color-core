import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { PfxColorsWorkspace } from "../../dist/index.js";
import { createPfxColorCore } from "./pfx-color-core.mjs";
import { createPfxColorsWorkspace, pfxGradientToCss } from "./pfx-color-workspace.mjs";

const bytes = await readFile(new URL(
  "../../target/wasm32-unknown-unknown/release/pfx_color_ffi.wasm",
  import.meta.url,
));
const core = await createPfxColorCore(bytes);
const fresh = () => createPfxColorsWorkspace(core, "#336699");

test("Rust opt-in workspace preserves source HEX, copies state and undo/redo", () => {
  const r = fresh();
  const ts = new PfxColorsWorkspace("#336699");
  assert.equal(r.getState().color.hex, ts.getState().color.hex);
  const state = r.getState();
  state.color.hex = "invalid";
  assert.equal(r.getState().color.hex, "#336699");
  r.setColor("#ff0000");
  ts.setColor("#ff0000");
  assert.equal(r.getState().color.hex, ts.getState().color.hex);
  assert.equal(r.canUndo(), ts.canUndo());
  assert.equal(r.undo().color.hex, ts.undo().color.hex);
  assert.equal(r.redo().color.hex, ts.redo().color.hex);
  r.clearHistory();
  assert.equal(r.canUndo(), false);
  assert.equal(r.canRedo(), false);
});

test("Rust opt-in workspace generates palettes and harmonies with matching shape", () => {
  const r = fresh();
  const ts = new PfxColorsWorkspace("#336699");
  r.generateTonalPalette({ count: 6 });
  ts.generateTonalPalette({ count: 6 });
  const a = r.getState().palette;
  const b = ts.getState().palette;
  assert.equal(a.mode, b.mode);
  assert.equal(a.colors.length, b.colors.length);
  assert.deepEqual(a.colors.map(v => v.position), b.colors.map(v => v.position));
  assert.equal(a.colors[3].value.space, "srgb");
  r.generateHarmony("triadic");
  ts.generateHarmony("triadic");
  assert.equal(r.getState().harmony.colors.length, ts.getState().harmony.colors.length);
  r.generatePaletteFromHarmony();
  ts.generatePaletteFromHarmony();
  assert.equal(r.getState().palette.colors.length, ts.getState().palette.colors.length);
});

test("Rust opt-in workspace gradients and color selection survive history operations", () => {
  const r = fresh();
  r.createGradient([
    { color: "#ff0000", position: 0 },
    { color: "#00ff00", position: 0.5 },
    { color: "#0000ff", position: 1 },
  ], { interpolationSpace: "srgb" });
  assert.equal(r.getState().gradient.stops.length, 3);
  assert.equal(r.setColorFromGradient(0.5).color.hex, "#00ff00");
  assert.equal(r.undo().color.hex, "#336699");
  assert.equal(r.redo().color.hex, "#00ff00");
});

test("Rust opt-in workspace respects limits and invalid operations", () => {
  assert.throws(() => createPfxColorsWorkspace(core, "#336699", 0), /History limit/);
  const r = fresh();
  assert.throws(() => r.setColorFromPalette(0), /out of range/);
  assert.throws(() => r.generatePaletteFromHarmony(), /No harmony/);
  assert.throws(() => r.setColorFromGradient(0.5), /No gradient/);
  assert.throws(() => r.createGradient([
    { color: "#000000", position: 0 },
    { color: "#ffffff", position: 2 },
  ]), /position/);
});

test("workspace serializes valid linear radial and conic CSS gradient geometry", () => {
  const stops = [
    { color: "#ff0000", position: 0 },
    { color: "#0000ff", position: 1 },
  ];
  const linear = fresh();
  linear.createGradient(stops, { type: "linear", angle: 90 });
  assert.match(pfxGradientToCss(linear.getState().gradient), /^linear-gradient\(90deg in oklch,/);
  const radial = fresh();
  radial.createGradient(stops, { type: "radial", centerX: 0.25, centerY: 0.75 });
  assert.ok(pfxGradientToCss(radial.getState().gradient).includes("circle at 25% 75% in oklch"));
  const conic = fresh();
  conic.createGradient(stops, { type: "conic", angle: 45, centerX: 0.2, centerY: 0.3 });
  assert.ok(pfxGradientToCss(conic.getState().gradient).includes("from 45deg at 20% 30% in oklch"));
});
