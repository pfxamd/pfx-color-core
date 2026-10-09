import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import {
  selectColor as legacySelect, setColorChannel as legacyChannel,
  setColorAlpha as legacyAlpha, mapColorToGamut as legacyMap,
} from "../../dist/index.js";
import { createPfxColorCore } from "./pfx-color-core.mjs";
import { createPfxColorTools } from "./pfx-color-tools.mjs";

const bytes = await readFile(new URL(
  "../../target/wasm32-unknown-unknown/release/pfx_color_ffi.wasm",
  import.meta.url,
));
const core = await createPfxColorCore(bytes);
const picker = createPfxColorTools(core);

test("opt-in Rust picker matches legacy hex and color coordinates", () => {
  for (const css of ["#336699", "#ff0014", "tomato", "hsl(120 100% 50%)",
    "rgb(255 100 5)", "rgba(20,70,140,.4)"]) {
    const actual = picker.selectColor(css);
    const expected = legacySelect(css);
    assert.equal(actual.hex, expected.hex, css);
    assert.equal(actual.source.alpha, expected.source.alpha, css);
    assert.equal(actual.values.hsl.space, "hsl");
    assert.equal(actual.values.oklch.space, "oklch");
    for (const target of ["srgb", "oklab", "oklch"]) {
      for (let i = 0; i < 3; i++) {
        const a = actual.values[target].coordinates[i];
        const b = expected.values[target].coordinates[i];
        if (b == null && i === 2) continue;
        const tolerance = target === "srgb" ? 0.0003 : 0.001;
        assert.ok(Math.abs(a - b) < tolerance,
          css + " " + target + " channel " + i + " " + a + " vs " + b);
      }
    }
  }
});

test("opt-in Rust picker channel and alpha editing matches legacy", () => {
  const actual = picker.setColorChannel("#ff0000", "hsl", 0, 120);
  const expected = legacyChannel("#ff0000", "hsl", 0, 120);
  assert.equal(actual.hex, expected.hex);
  assert.equal(actual.hex, "#00ff00");
  assert.equal(picker.setColorAlpha("#336699", 0.4).hex,
    legacyAlpha("#336699", 0.4).hex);
  assert.equal(picker.setColorAlpha("#336699", 2).alpha, 1);
  assert.equal(picker.setColorAlpha("#336699", -1).alpha, 0);
});

test("opt-in Rust picker gamut mapping matches legacy perceptually", () => {
  const src = { space: "oklch", coordinates: [0.7, 0.35, 35], alpha: 1 };
  const mapped = picker.mapColorToGamut(src, "srgb");
  const old = legacyMap(src, "srgb");
  assert.ok(/^#[a-f0-9]{6}$/.test(mapped.hex));
  assert.equal(mapped.gamut.srgb, true);
  assert.equal(mapped.source.space, "srgb");
  const a = core.parseCss(mapped.hex);
  const b = core.parseCss(old.hex);
  assert.ok(core.difference(a, b, "ok") < 0.05);
});

test("opt-in Rust picker rejects missing channels and invalid indices", () => {
  assert.throws(() => createPfxColorTools({}), /Initialized/);
  assert.throws(() => picker.selectColor("rgb(none 0 0)"), /CSS parse/);
  assert.throws(() => picker.selectColor({ space: "srgb", coordinates: [0, null, 0] }), /finite/);
  assert.throws(() => picker.setColorChannel("#fff", "hsl", 5, 20), /Channel index/);
  assert.throws(() => picker.setColorAlpha("#fff", Number.NaN), /finite/);
});
