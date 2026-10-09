import {
  colorEngine,
  type ColorInput,
  type ColorSpaceId,
  type ColorValue,
} from "../engine";

export const COLOR_PICKER_SPACES = ["srgb", "p3", "hsl", "oklab", "oklch"] as const;

export interface ColorPickerSelection {
  source: ColorValue;
  hex: string;
  alpha: number;
  values: Record<string, ColorValue>;
  gamut: { srgb: boolean; p3: boolean };
}

function toInput(value: ColorValue): ColorInput {
  return {
    space: value.space,
    coordinates: value.coordinates.map((coordinate) => coordinate ?? 0),
    alpha: value.alpha,
  };
}

export function selectColor(
  input: ColorInput,
  spaces: readonly ColorSpaceId[] = COLOR_PICKER_SPACES,
): ColorPickerSelection {
  const source = colorEngine.color.parse(input);
  const values = Object.fromEntries(
    spaces.map((space) => [space, colorEngine.color.convert(input, space)]),
  );

  return {
    source,
    hex: colorEngine.color.formatHex(input),
    alpha: source.alpha,
    values,
    gamut: {
      srgb: colorEngine.color.isInGamut(input, "srgb"),
      p3: colorEngine.color.isInGamut(input, "p3"),
    },
  };
}

export function setColorChannel(
  input: ColorInput,
  space: ColorSpaceId,
  channelIndex: number,
  value: number,
): ColorPickerSelection {
  if (!Number.isInteger(channelIndex) || channelIndex < 0 || channelIndex > 2) {
    throw new RangeError("Channel index must be 0, 1, or 2.");
  }
  if (!Number.isFinite(value)) throw new TypeError("Channel value must be finite.");

  const converted = colorEngine.color.convert(input, space);
  const coordinates = converted.coordinates.map((coordinate) => coordinate ?? 0);
  coordinates[channelIndex] = value;

  return selectColor({ space, coordinates, alpha: converted.alpha });
}

export function setColorAlpha(
  input: ColorInput,
  alpha: number,
): ColorPickerSelection {
  if (!Number.isFinite(alpha)) throw new TypeError("Alpha must be finite.");
  const source = colorEngine.color.parse(input);
  return selectColor({
    space: source.space,
    coordinates: source.coordinates.map((coordinate) => coordinate ?? 0),
    alpha: Math.max(0, Math.min(1, alpha)),
  });
}

export function mapColorToGamut(
  input: ColorInput,
  targetSpace: ColorSpaceId = "srgb",
): ColorPickerSelection {
  const mapped = colorEngine.color.mapToGamut(input, { targetSpace, method: "css" });
  return selectColor(toInput(mapped));
}
