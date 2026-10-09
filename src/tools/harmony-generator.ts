import {
  colorEngine,
  type ColorInput,
  type ColorSpaceId,
  type ColorValue,
} from "../engine";

export const HARMONY_SCHEMES = [
  "analogous",
  "complementary",
  "split-complementary",
  "triadic",
  "tetradic",
  "square",
] as const;

export type HarmonyScheme = (typeof HARMONY_SCHEMES)[number];

export interface HarmonyColor {
  index: number;
  hueOffset: number;
  value: ColorValue;
  hex: string;
  mapped: boolean;
}

export interface HarmonyResult {
  scheme: HarmonyScheme | "custom";
  baseHue: number;
  colors: HarmonyColor[];
}

export interface HarmonyOptions {
  targetSpace?: ColorSpaceId;
  analogousAngle?: number;
  splitAngle?: number;
  tetradicAngle?: number;
}

function wrapHue(value: number): number {
  const hue = value % 360;
  return hue < 0 ? hue + 360 : hue;
}

function offsetsFor(scheme: HarmonyScheme, options: HarmonyOptions): number[] {
  switch (scheme) {
    case "analogous": {
      const angle = options.analogousAngle ?? 30;
      return [-angle, 0, angle];
    }
    case "complementary":
      return [0, 180];
    case "split-complementary": {
      const angle = options.splitAngle ?? 30;
      return [0, 180 - angle, 180 + angle];
    }
    case "triadic":
      return [0, 120, 240];
    case "tetradic": {
      const angle = options.tetradicAngle ?? 60;
      return [0, angle, 180, 180 + angle];
    }
    case "square":
      return [0, 90, 180, 270];
  }
}

function toInput(value: ColorValue): ColorInput {
  return {
    space: value.space,
    coordinates: value.coordinates.map((coordinate) => coordinate ?? 0),
    alpha: value.alpha,
  };
}

function generate(
  seed: ColorInput,
  offsets: readonly number[],
  scheme: HarmonyResult["scheme"],
  targetSpace: ColorSpaceId,
): HarmonyResult {
  const base = colorEngine.color.convert(seed, "oklch");
  const lightness = base.coordinates[0] ?? 0;
  const chroma = base.coordinates[1] ?? 0;
  const baseHue = wrapHue(base.coordinates[2] ?? 0);

  return {
    scheme,
    baseHue,
    colors: offsets.map((hueOffset, index) => {
      const raw: ColorInput = {
        space: "oklch",
        coordinates: [lightness, chroma, wrapHue(baseHue + hueOffset)],
        alpha: base.alpha,
      };
      const inGamut = colorEngine.color.isInGamut(raw, targetSpace);
      const value = inGamut
        ? colorEngine.color.convert(raw, targetSpace)
        : colorEngine.color.mapToGamut(raw, { targetSpace, method: "css" });

      return {
        index,
        hueOffset,
        value,
        hex: colorEngine.color.formatHex(toInput(value)),
        mapped: !inGamut,
      };
    }),
  };
}

export function generateHarmony(
  seed: ColorInput,
  scheme: HarmonyScheme,
  options: HarmonyOptions = {},
): HarmonyResult {
  return generate(
    seed,
    offsetsFor(scheme, options),
    scheme,
    options.targetSpace ?? "srgb",
  );
}

export function generateCustomHarmony(
  seed: ColorInput,
  offsets: readonly number[],
  targetSpace: ColorSpaceId = "srgb",
): HarmonyResult {
  if (offsets.length < 2) throw new RangeError("At least 2 hue offsets are required.");
  return generate(seed, offsets, "custom", targetSpace);
}
