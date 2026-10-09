/**
 * Optional first-party PFx Colors workspace backed by the Rust color engine.
 * State/history logic is JavaScript; every color calculation uses Rust WASM.
 * No external runtime dependencies; legacy deployed app is unchanged.
 * SPDX-License-Identifier: Apache-2.0
 */
import { createPfxColorTools } from "./pfx-color-tools.mjs";

const schemeIds = Object.freeze({
  analogous: "analogous", complementary: "complementary",
  "split-complementary": "splitComplementary",
  triadic: "triadic", tetradic: "tetradic", square: "square",
});
const coreSpace = space => space === "p3" ? "display-p3" : space;
function toInput(value) {
  return { space: coreSpace(value.space), channels: [...value.coordinates], alpha: value.alpha };
}
function checkedHistory(value) {
  if (!Number.isInteger(value) || value < 1 || value > 1000) {
    throw new RangeError("History limit must be an integer between 1 and 1000");
  }
  return value;
}
function checkedPosition(value) {
  if (!Number.isFinite(value) || value < 0 || value > 1) {
    throw new RangeError("Gradient position must be between 0 and 1");
  }
  return value;
}
function wrapPalette(mode, result, tools) {
  return {
    mode,
    colors: result.map(entry => ({
      index: entry.index,
      position: entry.position,
      value: tools.parse(entry.color),
      hex: tools.formatHex(entry.color),
      mapped: entry.mapped,
    })),
  };
}

/**
 * A separate opt-in workspace with the existing getState/undo/redo semantics.
 * Not a drop-in production swap; CSS gradient rendering is still legacy.
 */
export function createPfxColorsWorkspace(core, initialColor = "#ff0014", historyLimit = 100) {
  checkedHistory(historyLimit);
  const tools = createPfxColorTools(core);
  let state = { color: tools.selectColor(initialColor), palette: null, harmony: null, gradient: null };
  let past = [];
  let future = [];
  const clone = value => structuredClone(value);
  const current = () => clone(state);
  const commit = next => {
    past.push(clone(state));
    if (past.length > historyLimit) past.shift();
    future = [];
    state = next;
    return current();
  };
  const api = {
    getState: current,
    canUndo: () => past.length > 0,
    canRedo: () => future.length > 0,
    setColor(input) {
      return commit({ ...state, color: tools.selectColor(input) });
    },
    generateTonalPalette(options = {}) {
      const result = core.tonalPalette(toInput(state.color.source), {
        count: options.count ?? 9,
        minLightness: options.minLightness ?? 0.12,
        maxLightness: options.maxLightness ?? 0.96,
        chromaScale: options.chromaScale ?? 1,
        target: coreSpace(options.targetSpace ?? "srgb"),
        gamut: "css",
      });
      return commit({ ...state, palette: wrapPalette("tonal", result, tools) });
    },
    generateRampPalette(second, options = {}) {
      const result = core.rampPalette(toInput(state.color.source), toInput(tools.parse(second)), {
        count: options.count ?? 9,
        space: coreSpace(options.interpolationSpace ?? "oklch"),
        target: coreSpace(options.targetSpace ?? "srgb"),
        hue: options.hue ?? "shorter",
        gamut: "css",
      });
      return commit({ ...state, palette: wrapPalette("ramp", result, tools) });
    },
    generateHarmony(scheme, options = {}) {
      const target = coreSpace(options.targetSpace ?? "srgb");
      if (!Object.hasOwn(schemeIds, scheme)) throw new RangeError("Unknown harmony scheme");
      const seed = toInput(state.color.source);
      const raw = core.harmony(seed, schemeIds[scheme], {
        target, gamut: "css",
        analogousAngle: options.analogousAngle ?? 30,
        splitAngle: options.splitAngle ?? 30,
        tetradicAngle: options.tetradicAngle ?? 60,
      });
      const baseHue = core.convert(seed, "oklch").channels[2];
      const colors = raw.map(item => ({
        index: item.index,
        hueOffset: item.hueOffset,
        value: tools.parse(item.color),
        hex: tools.formatHex(item.color),
        mapped: item.mapped,
      }));
      return commit({ ...state, harmony: { scheme, baseHue, colors } });
    },
    generatePaletteFromHarmony(options = {}) {
      if (!state.harmony) throw new Error("No harmony available");
      const anchors = state.harmony.colors.map(entry => toInput(entry.value));
      const result = core.anchoredPalette(anchors, {
        count: options.count ?? anchors.length,
        space: coreSpace(options.interpolationSpace ?? "oklch"),
        target: coreSpace(options.targetSpace ?? "srgb"),
        hue: options.hue ?? "shorter",
        gamut: "css",
      });
      return commit({ ...state, palette: wrapPalette("anchors", result, tools) });
    },
    createGradient(stops, options = {}) {
      if (!Array.isArray(stops) || stops.length < 2 || stops.length > 256) {
        throw new RangeError("Gradient requires 2..256 stops");
      }
      const target = coreSpace(options.targetSpace ?? "srgb");
      const mapped = stops.map((entry, order) => {
        const position = checkedPosition(entry.position);
        const source = tools.parse(entry.color);
        const original = toInput(source);
        const value = core.isInGamut(original, target)
          ? core.convert(original, target)
          : core.mapGamut(original, target, "css");
        return { order, position, source, value: tools.parse(value), hex: tools.formatHex(value) };
      }).sort((a, b) => a.position - b.position || a.order - b.order)
        .map(({ order, ...entry }, index) => ({ index, ...entry }));
      const geometry = {
        type: options.type ?? "linear",
        angle: ((options.angle ?? 90) % 360 + 360) % 360,
        centerX: options.centerX ?? 0.5,
        centerY: options.centerY ?? 0.5,
        interpolationSpace: options.interpolationSpace ?? "oklch",
        targetSpace: options.targetSpace ?? "srgb",
        hue: options.hue,
        stops: mapped,
      };
      // Validate through the Rust-owned gradient builder, not JS math.
      const test = core.createGradient(mapped.map(entry => ({
        position: entry.position, color: toInput(entry.source),
      })), {
        kind: geometry.type, angle: geometry.angle,
        centerX: geometry.centerX, centerY: geometry.centerY,
        space: coreSpace(geometry.interpolationSpace), target, hue: geometry.hue ?? "shorter",
        gamut: "css",
      });
      test.dispose();
      return commit({ ...state, gradient: geometry });
    },
    createGradientFromPalette(options = {}) {
      if (!state.palette) throw new Error("No palette available");
      return this.createGradient(state.palette.colors.map(entry => ({
        color: toInput(entry.value), position: entry.position,
      })), options);
    },
    createGradientFromHarmony(options = {}) {
      if (!state.harmony) throw new Error("No harmony available");
      const last = state.harmony.colors.length - 1;
      return this.createGradient(state.harmony.colors.map((entry, index) => ({
        color: toInput(entry.value), position: index / last,
      })), options);
    },
    setColorFromPalette(index) {
      const entry = state.palette?.colors[index];
      if (!entry) throw new RangeError("Palette color index out of range");
      return this.setColor(toInput(entry.value));
    },
    setColorFromHarmony(index) {
      const entry = state.harmony?.colors[index];
      if (!entry) throw new RangeError("Harmony color index out of range");
      return this.setColor(toInput(entry.value));
    },
    setColorFromGradient(position) {
      if (!state.gradient) throw new Error("No gradient available");
      const def = state.gradient;
      const handle = core.createGradient(def.stops.map(entry => ({
        position: entry.position, color: toInput(entry.source),
      })), {
        kind: def.type, angle: def.angle, centerX: def.centerX,
        centerY: def.centerY, space: coreSpace(def.interpolationSpace),
        target: coreSpace(def.targetSpace), hue: def.hue ?? "shorter", gamut: "css",
      });
      try {
        return this.setColor(handle.sample(checkedPosition(position)));
      } finally {
        handle.dispose();
      }
    },
    undo() {
      const previous = past.pop();
      if (!previous) return current();
      future.push(clone(state));
      state = previous;
      return current();
    },
    redo() {
      const next = future.pop();
      if (!next) return current();
      past.push(clone(state));
      if (past.length > historyLimit) past.shift();
      state = next;
      return current();
    },
    clearHistory() {
      past = [];
      future = [];
      return current();
    },
  };
  return Object.freeze(api);
}
