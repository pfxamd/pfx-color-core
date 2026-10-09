/**
 * PFx Color Core — dependency-free JavaScript / WebAssembly bindings.
 * All color calculations occur in the original Rust engine. This module
 * only translates arguments and manages Rust-owned opaque pointers.
 * SPDX-License-Identifier: Apache-2.0
 */
export const ColorSpaces = Object.freeze({
  srgb: 0, "srgb-linear": 1, "display-p3": 2, "display-p3-linear": 3,
  rec2020: 4, "rec2020-linear": 5, "xyz-d65": 6, "xyz-d50": 7,
  lab: 8, lch: 9, oklab: 10, oklch: 11, hsl: 12, hwb: 13, hsv: 14,
});
const SpaceNames = Object.freeze(Object.keys(ColorSpaces));
const DifferenceMethods = Object.freeze({ cie76: 0, ciede2000: 1, ok: 2 });
const HueMethods = Object.freeze({ shorter: 0, longer: 1, increasing: 2, decreasing: 3 });
const GamutMethods = Object.freeze({ clip: 0, oklchChroma: 1, css: 2 });
const HarmonySchemes = Object.freeze({
  analogous: 0, complementary: 1, splitComplementary: 2,
  triadic: 3, tetradic: 4, square: 5,
});
const GradientKinds = Object.freeze({ linear: 0, radial: 1, conic: 2 });

function code(values, value, label) {
  if (typeof value !== "string" || !Object.hasOwn(values, value)) {
    throw new RangeError("Unknown " + label + ": " + String(value));
  }
  return values[value];
}
function finite(value, label) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeError(label + " must be a finite number");
  }
  return value;
}
function validCount(value) {
  if (!Number.isInteger(value) || value < 2 || value > 256) {
    throw new RangeError("count must be an integer from 2 to 256");
  }
  return value;
}
function normalized(input) {
  if (!input || typeof input !== "object" || !Array.isArray(input.channels) || input.channels.length !== 3) {
    throw new TypeError("Color requires space, channels[3], optional alpha");
  }
  return {
    space: code(ColorSpaces, input.space, "color space"),
    channels: input.channels.map((v, i) => finite(v, "channels[" + i + "]")),
    alpha: finite(input.alpha ?? 1, "alpha"),
  };
}
function status(value, label) {
  if (value !== 0) throw new Error(label + " failed (PFx ABI status " + value + ")");
}
function scalar(value, label) {
  if (!Number.isFinite(value)) throw new Error(label + " failed (invalid value/result)");
  return value;
}

/**
 * Initialize from a WebAssembly.Module, ArrayBuffer or Uint8Array.
 * Supply bytes via fetch() in a browser or readFile() in Node yourself.
 * No npm runtime dependencies, bindgen or JavaScript color algorithms.
 */
export async function createPfxColorCore(wasm) {
  const loaded = await WebAssembly.instantiate(wasm, {});
  const instance = loaded instanceof WebAssembly.Instance ? loaded : loaded.instance;
  const e = instance.exports;
  if (e.pfx_abi_version?.() !== 1 || e.pfx_color_size?.() !== 40 || !e.memory || !e.pfx_css_parse || !e.pfx_css_format) {
    throw new Error("Unsupported PFx Color Core WASM ABI; expected revision 1");
  }

  const colorNew = () => {
    const ptr = e.pfx_color_new();
    if (!ptr) throw new Error("Unable to allocate PFx Color buffer");
    return ptr;
  };
  const colorRead = (ptr) => {
    const id = e.pfx_color_get_space(ptr);
    if (id >= SpaceNames.length) throw new Error("Invalid returned color space");
    return {
      space: SpaceNames[id],
      channels: [0, 1, 2].map((i) => scalar(e.pfx_color_get_channel(ptr, i), "channel")),
      alpha: scalar(e.pfx_color_get_alpha(ptr), "alpha"),
    };
  };
  const colorWith = (value, callback) => {
    const v = normalized(value);
    const ptr = colorNew();
    try {
      status(e.pfx_color_set(ptr, v.space, ...v.channels, v.alpha), "set color");
      return callback(ptr);
    } finally {
      e.pfx_color_free(ptr);
    }
  };
  const pairWith = (a, b, callback) =>
    colorWith(a, (pa) => colorWith(b, (pb) => callback(pa, pb)));
  const outputWith = (callback) => {
    const out = colorNew();
    try {
      callback(out);
      return colorRead(out);
    } finally {
      e.pfx_color_free(out);
    }
  };
  const paletteCollect = (handle, type) => {
    if (!handle) throw new Error("PFx " + type + " could not be created");
    try {
      const count = e.pfx_palette_len(handle);
      return Array.from({ length: count }, (_, index) => {
        const color = outputWith((out) =>
          status(e.pfx_palette_get(handle, index, out), "palette item"));
        return {
          index,
          position: scalar(e.pfx_palette_position(handle, index), "palette position"),
          hueOffset: scalar(e.pfx_palette_hue_offset(handle, index), "hue offset"),
          mapped: e.pfx_palette_mapped(handle, index) === 1,
          color,
        };
      });
    } finally {
      e.pfx_palette_free(handle);
    }
  };
  const paletteDefaults = (opts) => ({
    target: code(ColorSpaces, opts.target ?? "srgb", "target space"),
    mapping: code(GamutMethods, opts.gamut ?? "oklchChroma", "gamut method"),
  });
  const mixDefaults = (opts) => ({
    space: code(ColorSpaces, opts.space ?? "oklch", "interpolation space"),
    hue: code(HueMethods, opts.hue ?? "shorter", "hue method"),
  });

  const textEncoder = new TextEncoder();
  const textDecoder = new TextDecoder("utf-8", { fatal: true });
  const utf8With = (bytes, callback) => {
    if (bytes.length === 0 || bytes.length > 1024) {
      throw new RangeError("CSS input must contain 1..1024 UTF-8 bytes");
    }
    const ptr = e.pfx_buffer_new(bytes.length);
    if (!ptr) throw new Error("Unable to allocate Rust UTF-8 buffer");
    try {
      new Uint8Array(e.memory.buffer, ptr, bytes.length).set(bytes);
      return callback(ptr, bytes.length);
    } finally {
      e.pfx_buffer_free(ptr, bytes.length);
    }
  };
  const formatText = (color, kind, mapping) => colorWith(color, (ptr) => {
    const length = 512;
    const out = e.pfx_buffer_new(length);
    if (!out) throw new Error("Unable to allocate Rust string buffer");
    try {
      const written = e.pfx_css_format(ptr, kind, mapping, out, length);
      if (written < 0) status(written, "CSS format");
      return textDecoder.decode(new Uint8Array(e.memory.buffer, out, written));
    } finally {
      e.pfx_buffer_free(out, length);
    }
  });
  return Object.freeze({
    parseCss(source) {
      if (typeof source !== "string") throw new TypeError("CSS source must be a string");
      return utf8With(textEncoder.encode(source), (ptr, length) =>
        outputWith((out) => status(e.pfx_css_parse(ptr, length, out), "CSS parse")));
    },
    formatCss(color) {
      return formatText(color, 0, 0);
    },
    formatHex(color, method = "css") {
      return formatText(color, 1, code(GamutMethods, method, "gamut method"));
    },
    convert(color, target) {
      return colorWith(color, (input) => outputWith((out) =>
        status(e.pfx_color_convert(input, code(ColorSpaces, target, "target space"), out), "conversion")));
    },
    difference(a, b, method = "ok") {
      return pairWith(a, b, (pa, pb) => scalar(
        e.pfx_color_difference(pa, pb, code(DifferenceMethods, method, "difference method")),
        "color difference"));
    },
    contrast(a, b) {
      return pairWith(a, b, (pa, pb) => scalar(e.pfx_color_contrast(pa, pb), "WCAG contrast"));
    },
    luminance(color) {
      return colorWith(color, (ptr) => scalar(e.pfx_color_luminance(ptr), "relative luminance"));
    },
    isInGamut(color, target = "srgb") {
      return colorWith(color, (ptr) => {
        const result = e.pfx_color_is_in_gamut(ptr, code(ColorSpaces, target, "target space"));
        if (result < 0) status(result, "gamut check");
        return result === 1;
      });
    },
    mapGamut(color, target = "srgb", method = "oklchChroma") {
      return colorWith(color, (ptr) => outputWith((out) =>
        status(e.pfx_color_map(
          ptr, code(ColorSpaces, target, "target space"),
          code(GamutMethods, method, "gamut method"), out), "gamut map")));
    },
    interpolate(a, b, fraction, options = {}) {
      const mix = mixDefaults(options);
      return pairWith(a, b, (pa, pb) => outputWith((out) =>
        status(e.pfx_color_interpolate(pa, pb, finite(fraction, "fraction"),
          mix.space, mix.hue, out), "interpolation")));
    },
    steps(a, b, count, options = {}) {
      const total = validCount(count);
      return Array.from({ length: total }, (_, index) =>
        this.interpolate(a, b, index / (total - 1), options));
    },
    colorStudy(seed, options = {}) {
      const target = code(ColorSpaces, options.target ?? "srgb", "target space");
      const mapping = code(GamutMethods, options.gamut ?? "css", "gamut method");
      const randomSeed = options.randomSeed ?? 0;
      if (!Number.isInteger(randomSeed) || randomSeed < 0 || randomSeed > 0xffffffff) {
        throw new RangeError("randomSeed must be an unsigned 32-bit integer");
      }
      return colorWith(seed, (ptr) => {
        const study = e.pfx_study_new(
          ptr, randomSeed,
          finite(options.lightness ?? 58, "lightness"),
          finite(options.chroma ?? 58, "chroma"),
          finite(options.hueRange ?? 58, "hueRange"),
          finite(options.toneRange ?? 58, "toneRange"),
          target, mapping,
        );
        if (!study) throw new Error("PFx Color Study could not be created");
        try {
          const schemeId = e.pfx_study_scheme(study);
          const scheme = Object.entries(HarmonySchemes).find(([, id]) => id === schemeId)?.[0];
          if (!scheme) throw new Error("Invalid returned study scheme");
          const colors = Array.from({ length: e.pfx_study_len(study) }, (_, index) => {
            const color = outputWith((out) => status(e.pfx_study_get(study, index, out), "study color"));
            return {
              index,
              color,
              mapped: e.pfx_study_mapped(study, index) === 1,
              oklch: [0, 1, 2].map((channel) =>
                scalar(e.pfx_study_oklch(study, index, channel), "study Oklch")),
            };
          });
          return { scheme, colors };
        } finally {
          e.pfx_study_free(study);
        }
      });
    },
    tonalPalette(seed, options = {}) {
      const { target, mapping } = paletteDefaults(options);
      return colorWith(seed, (ptr) => paletteCollect(e.pfx_palette_tonal_new(
        ptr, validCount(options.count ?? 9),
        finite(options.minLightness ?? 0.12, "minLightness"),
        finite(options.maxLightness ?? 0.96, "maxLightness"),
        finite(options.chromaScale ?? 1, "chromaScale"),
        target, mapping), "tonal palette"));
    },
    rampPalette(a, b, options = {}) {
      const { target, mapping } = paletteDefaults(options);
      const { space, hue } = mixDefaults(options);
      return pairWith(a, b, (pa, pb) => paletteCollect(e.pfx_palette_ramp_new(
        pa, pb, validCount(options.count ?? 9), space, target, hue, mapping),
        "ramp palette"));
    },
    /**
     * Deterministic palette of evenly spaced input anchors.
     * Color interpolation and gamut mapping are computed in Rust.
     */
    anchoredPalette(anchors, options = {}) {
      if (!Array.isArray(anchors) || anchors.length < 2 || anchors.length > 256) {
        throw new RangeError("Anchor palette requires 2..256 colors");
      }
      const count = validCount(options.count ?? Math.max(9, anchors.length));
      if (count < anchors.length) {
        throw new RangeError("count cannot be smaller than anchor count");
      }
      const { target, mapping } = paletteDefaults(options);
      const { space, hue } = mixDefaults(options);
      const builder = e.pfx_anchors_new();
      if (!builder) throw new Error("Unable to create anchor builder");
      try {
        for (const input of anchors) {
          colorWith(input, (ptr) =>
            status(e.pfx_anchors_add(builder, ptr), "add anchor"));
        }
        return paletteCollect(
          e.pfx_anchors_palette(builder, count, space, target, hue, mapping),
          "anchored palette",
        );
      } finally {
        e.pfx_anchors_free(builder);
      }
    },
    customHarmony(seed, offsets, options = {}) {
      if (!Array.isArray(offsets) || offsets.length < 2 || offsets.length > 256) {
        throw new RangeError("Custom harmony requires 2..256 hue offsets");
      }
      const { target, mapping } = paletteDefaults(options);
      return colorWith(seed, (ptr) => {
        const builder = e.pfx_custom_harmony_new(ptr, target, mapping);
        if (!builder) throw new Error("Unable to create custom harmony");
        try {
          for (const offset of offsets) {
            status(e.pfx_custom_harmony_add(builder, finite(offset, "hue offset")),
              "add hue offset");
          }
          return paletteCollect(e.pfx_custom_harmony_palette(builder), "custom harmony");
        } finally {
          e.pfx_custom_harmony_free(builder);
        }
      });
    },
    harmony(seed, scheme, options = {}) {
      const { target, mapping } = paletteDefaults(options);
      return colorWith(seed, (ptr) => paletteCollect(e.pfx_palette_harmony_new(
        ptr, code(HarmonySchemes, scheme, "harmony scheme"),
        finite(options.analogousAngle ?? 30, "analogousAngle"),
        finite(options.splitAngle ?? 30, "splitAngle"),
        finite(options.tetradicAngle ?? 60, "tetradicAngle"),
        target, mapping), "harmony"));
    },
    createGradient(stops, options = {}) {
      if (!Array.isArray(stops) || stops.length < 2 || stops.length > 256) {
        throw new RangeError("Gradient requires 2..256 stops");
      }
      const { target, mapping } = paletteDefaults(options);
      const { space, hue } = mixDefaults(options);
      const handle = e.pfx_gradient_new(
        code(GradientKinds, options.kind ?? "linear", "gradient kind"),
        finite(options.angle ?? 90, "angle"),
        finite(options.centerX ?? 0.5, "centerX"),
        finite(options.centerY ?? 0.5, "centerY"),
        space, target, hue, mapping);
      if (!handle) throw new Error("Invalid gradient configuration");
      try {
        for (const stop of stops) {
          colorWith(stop.color, (ptr) =>
            status(e.pfx_gradient_add_stop(handle, finite(stop.position, "stop position"), ptr),
              "add gradient stop"));
        }
      } catch (error) {
        e.pfx_gradient_free(handle);
        throw error;
      }
      let disposed = false;
      function active() {
        if (disposed) throw new Error("PFx gradient has been disposed");
      }
      return Object.freeze({
        sample(position) {
          active();
          return outputWith((out) =>
            status(e.pfx_gradient_sample(handle, finite(position, "position"), out), "gradient sample"));
        },
        sampleXY(x, y) {
          active();
          return outputWith((out) => status(e.pfx_gradient_sample_xy(
            handle, finite(x, "x"), finite(y, "y"), out), "gradient sampleXY"));
        },
        dispose() {
          if (!disposed) {
            disposed = true;
            e.pfx_gradient_free(handle);
          }
        },
      });
    },
  });
}
