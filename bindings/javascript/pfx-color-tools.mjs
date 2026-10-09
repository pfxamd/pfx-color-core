/**
 * Experimental color-picker functions backed by first-party Rust WASM.
 * This is opt-in and never changes the deployed PFx Colors application.
 * Apache-2.0; zero npm runtime dependencies.
 */
const SPACES = Object.freeze(["srgb", "p3", "hsl", "oklab", "oklch"]);
const toCore = space => space === "p3" ? "display-p3" : space;
const toLegacy = space => space === "display-p3" ? "p3" : space;

export function createPfxColorTools(core) {
  if (!core || ["parseCss", "convert", "formatCss", "formatHex", "isInGamut",
    "mapGamut"].some(key => typeof core[key] !== "function")) {
    throw new TypeError("Initialized Rust WebAssembly core is required");
  }
  function normalize(value) {
    if (typeof value === "string") return core.parseCss(value);
    const channels = value?.channels ?? value?.coordinates;
    if (!Array.isArray(channels) || channels.length !== 3
      || channels.some(v => typeof v !== "number" || !Number.isFinite(v))) {
      throw new TypeError("Color requires three finite numeric coordinates");
    }
    if (typeof value.space !== "string") throw new TypeError("Color space required");
    const alpha = value.alpha ?? 1;
    if (!Number.isFinite(alpha) || alpha < 0 || alpha > 1) {
      throw new RangeError("Alpha must be between 0 and 1");
    }
    return { space: toCore(value.space), channels: [...channels], alpha };
  }
  function value(raw) {
    return {
      space: toLegacy(raw.space),
      coordinates: [...raw.channels],
      alpha: raw.alpha,
      css: core.formatCss(raw),
      hex: core.formatHex(raw, "css"),
    };
  }
  function formatHex(input) {
    return core.formatHex(normalize(input), "css");
  }
  function selectColor(input, spaces = SPACES) {
    if (!Array.isArray(spaces)) throw new TypeError("spaces must be an array");
    const raw = normalize(input);
    const values = Object.fromEntries(spaces.map(space =>
      [space, value(core.convert(raw, toCore(space)))]));
    return {
      source: value(raw),
      hex: formatHex(raw),
      alpha: raw.alpha,
      values,
      gamut: {
        srgb: core.isInGamut(raw, "srgb"),
        p3: core.isInGamut(raw, "display-p3"),
      },
    };
  }
  return Object.freeze({
    parse: input => value(normalize(input)),
    convert: (input, space) => value(core.convert(normalize(input), toCore(space))),
    formatHex,
    selectColor,
    setColorChannel(input, space, index, newValue) {
      if (!Number.isInteger(index) || index < 0 || index > 2) {
        throw new RangeError("Channel index must be 0, 1 or 2");
      }
      if (!Number.isFinite(newValue)) throw new TypeError("Channel value must be finite");
      const raw = core.convert(normalize(input), toCore(space));
      const channels = [...raw.channels];
      channels[index] = newValue;
      return selectColor({ ...raw, channels });
    },
    setColorAlpha(input, alpha) {
      if (!Number.isFinite(alpha)) throw new TypeError("Alpha must be finite");
      return selectColor({ ...normalize(input), alpha: Math.min(1, Math.max(0, alpha)) });
    },
    mapColorToGamut(input, target = "srgb") {
      return selectColor(core.mapGamut(normalize(input), toCore(target), "css"));
    },
  });
}
