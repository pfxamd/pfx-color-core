export const COLOR_SPACES = [
  "srgb", "srgb-linear", "p3", "rec2020", "a98rgb", "prophoto",
  "xyz-d50", "xyz-d65", "lab", "lab-d65", "lch", "oklab", "oklch",
  "hsl", "hsv", "hwb", "okhsl", "okhsv",
] as const;

export type ColorSpaceId = (typeof COLOR_SPACES)[number] | string;

export type ColorInput =
  | string
  | { space: ColorSpaceId; coordinates: readonly number[]; alpha?: number };

export interface ColorValue {
  space: ColorSpaceId;
  coordinates: Array<number | null>;
  alpha: number;
  css: string;
  hex: string | null;
}

export type ContrastAlgorithm = "wcag21" | "apca";
export interface ContrastResult {
  algorithm: ContrastAlgorithm;
  value: number;
  foreground: string;
  background: string;
}

export type DifferenceAlgorithm = "76" | "2000" | "ok" | "itp" | "jz" | "hct";
export interface ColorDifference {
  algorithm: DifferenceAlgorithm;
  value: number;
}

export interface InterpolationOptions {
  space?: ColorSpaceId;
  outputSpace?: ColorSpaceId;
  hue?: "shorter" | "longer" | "increasing" | "decreasing" | "raw";
}

export interface GamutMapOptions {
  targetSpace?: ColorSpaceId;
  method?: "css" | "clip";
}

export interface ColorEngine {
  parse(input: ColorInput): ColorValue;
  convert(input: ColorInput, targetSpace: ColorSpaceId): ColorValue;
  formatHex(input: ColorInput): string;
  contrast(foreground: ColorInput, background: ColorInput, algorithm?: ContrastAlgorithm): ContrastResult;
  difference(first: ColorInput, second: ColorInput, algorithm?: DifferenceAlgorithm): ColorDifference;
  interpolate(first: ColorInput, second: ColorInput, amount: number, options?: InterpolationOptions): ColorValue;
  steps(first: ColorInput, second: ColorInput, count: number, options?: InterpolationOptions): ColorValue[];
  isInGamut(input: ColorInput, targetSpace?: ColorSpaceId): boolean;
  mapToGamut(input: ColorInput, options?: GamutMapOptions): ColorValue;
}
