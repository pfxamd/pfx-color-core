import {
  colorEngine,
  type ColorInput,
  type ColorSpaceId,
  type ColorValue,
  type InterpolationOptions,
} from "../engine";

export interface PaletteColor {
  index: number;
  position: number;
  value: ColorValue;
  hex: string;
  mapped: boolean;
}

export interface PaletteResult {
  mode: "tonal" | "ramp" | "anchors";
  colors: PaletteColor[];
}

export interface TonalPaletteOptions {
  count?: number;
  minLightness?: number;
  maxLightness?: number;
  chromaScale?: number;
  targetSpace?: ColorSpaceId;
}

export interface RampPaletteOptions {
  count?: number;
  interpolationSpace?: ColorSpaceId;
  targetSpace?: ColorSpaceId;
  hue?: InterpolationOptions["hue"];
}

function checkedCount(value = 9): number {
  if (!Number.isInteger(value) || value < 2 || value > 256) {
    throw new RangeError("Palette count must be between 2 and 256.");
  }
  return value;
}

function toInput(value: ColorValue): ColorInput {
  return {
    space: value.space,
    coordinates: value.coordinates.map((coordinate) => coordinate ?? 0),
    alpha: value.alpha,
  };
}

function finish(
  mode: PaletteResult["mode"],
  entries: Array<{ input: ColorInput; position: number }>,
  targetSpace: ColorSpaceId,
): PaletteResult {
  return {
    mode,
    colors: entries.map(({ input, position }, index) => {
      const inGamut = colorEngine.color.isInGamut(input, targetSpace);
      const value = inGamut
        ? colorEngine.color.convert(input, targetSpace)
        : colorEngine.color.mapToGamut(input, { targetSpace, method: "css" });

      return {
        index,
        position,
        value,
        hex: colorEngine.color.formatHex(toInput(value)),
        mapped: !inGamut,
      };
    }),
  };
}

export function generateTonalPalette(
  seed: ColorInput,
  options: TonalPaletteOptions = {},
): PaletteResult {
  const total = checkedCount(options.count);
  const min = options.minLightness ?? 0.12;
  const max = options.maxLightness ?? 0.96;
  if (!(min >= 0 && max <= 1 && min < max)) {
    throw new RangeError("Invalid lightness range.");
  }

  const seedOklch = colorEngine.color.convert(seed, "oklch");
  const chroma = (seedOklch.coordinates[1] ?? 0) * (options.chromaScale ?? 1);
  const hue = seedOklch.coordinates[2] ?? 0;
  const targetSpace = options.targetSpace ?? "srgb";

  const entries = Array.from({ length: total }, (_, index) => {
    const position = index / (total - 1);
    return {
      position,
      input: {
        space: "oklch",
        coordinates: [min + (max - min) * position, chroma, hue],
        alpha: seedOklch.alpha,
      } satisfies ColorInput,
    };
  });

  return finish("tonal", entries, targetSpace);
}

export function generateRampPalette(
  first: ColorInput,
  second: ColorInput,
  options: RampPaletteOptions = {},
): PaletteResult {
  const total = checkedCount(options.count);
  const targetSpace = options.targetSpace ?? "srgb";
  const interpolationSpace = options.interpolationSpace ?? "oklch";
  const entries = Array.from({ length: total }, (_, index) => {
    const position = index / (total - 1);
    const value = colorEngine.color.interpolate(first, second, position, {
      space: interpolationSpace,
      outputSpace: targetSpace,
      hue: options.hue,
    });
    return { input: toInput(value), position };
  });

  return finish("ramp", entries, targetSpace);
}

export function generatePaletteFromAnchors(
  anchors: readonly ColorInput[],
  options: RampPaletteOptions = {},
): PaletteResult {
  if (anchors.length < 2) throw new RangeError("At least 2 anchors are required.");

  const total = checkedCount(options.count ?? Math.max(9, anchors.length));
  if (total < anchors.length) {
    throw new RangeError("Palette count cannot be smaller than anchor count.");
  }

  const targetSpace = options.targetSpace ?? "srgb";
  const interpolationSpace = options.interpolationSpace ?? "oklch";
  const segments = anchors.length - 1;

  const entries = Array.from({ length: total }, (_, index) => {
    const position = index / (total - 1);
    const scaled = position * segments;
    const segment = Math.min(segments - 1, Math.floor(scaled));
    const local = position === 1 ? 1 : scaled - segment;
    const value = colorEngine.color.interpolate(
      anchors[segment],
      anchors[segment + 1],
      local,
      {
        space: interpolationSpace,
        outputSpace: targetSpace,
        hue: options.hue,
      },
    );
    return { input: toInput(value), position };
  });

  return finish("anchors", entries, targetSpace);
}
