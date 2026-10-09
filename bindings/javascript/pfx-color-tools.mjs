/**
 * Experimental color-picker functions backed by first-party Rust WASM.
 * This is opt-in and never changes the deployed PFx Colors application.
 * Apache-2.0; zero npm runtime dependencies.
 */
const SPACES = Object.freeze(["srgb", "p3", "hsl", "oklab", "oklch"]);
const toCore = space => space === "p3" ? "display-p3" : space;
const toLegacy = space => space === "display-p3" ? "p3" : space;

// The legacy UI retains 4-digit #RGBA when each byte is compressible.
function legacyHex(value) {
  if (/^#([0-9a-f]{2}){4}$/.test(value)
      && [1, 3, 5, 7].every(i => value[i] === value[i + 1])) {
    return "#" + value[1] + value[3] + value[5] + value[7];
  }
  return value;
}

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
      hex: legacyHex(core.formatHex(raw, "css")),
    };
  }
  function formatHex(input) {
    return legacyHex(core.formatHex(normalize(input), "css"));
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
    /**
     * Rust-powered version of the Home panel's deterministic 10-color study.
     * All color calculations are in Rust; the seed controls its PRNG.
     */
    generateColorStudy(input, options = {}) {
      const source = normalize(input);
      const seed = options.randomSeed ?? Math.floor(Math.random() * 4294967296);
      if (!Number.isInteger(seed) || seed < 0 || seed > 0xffffffff) {
        throw new RangeError("randomSeed must be a 32-bit unsigned integer");
      }
      const clampControl = (value, label) => {
        if (!Number.isFinite(value)) throw new TypeError(label + " must be finite");
        return Math.min(100, Math.max(0, value));
      };
      const controls = {
        lightness: clampControl(options.lightness ?? 58, "lightness"),
        chroma: clampControl(options.chroma ?? 58, "chroma"),
        hueRange: clampControl(options.hueRange ?? 58, "hueRange"),
        toneRange: clampControl(options.toneRange ?? 58, "toneRange"),
      };
      const study = core.colorStudy(source, {
        ...controls,
        randomSeed: seed,
        target: toCore(options.targetSpace ?? "srgb"),
        gamut: "css",
      });
      return {
        seedHex: formatHex(source),
        scheme: study.scheme === "splitComplementary" ? "split-complementary" : study.scheme,
        controls,
        colors: study.colors.map(entry => ({
          index: entry.index,
          hex: formatHex(entry.color),
          value: value(entry.color),
          oklch: { l: entry.oklch[0], c: entry.oklch[1], h: entry.oklch[2] },
          mapped: entry.mapped,
        })),
      };
    },
    mapColorToGamut(input, target = "srgb") {
      return selectColor(core.mapGamut(normalize(input), toCore(target), "css"));
    },
  });
}
