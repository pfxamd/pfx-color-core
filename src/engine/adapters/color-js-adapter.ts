import Color from "colorjs.io";
import type {
  ColorDifference,
  ColorEngine,
  ColorInput,
  ColorSpaceId,
  ColorValue,
  ContrastAlgorithm,
  ContrastResult,
  DifferenceAlgorithm,
  GamutMapOptions,
  InterpolationOptions,
} from "../types/color";

function toColor(input: ColorInput): Color {
  if (typeof input === "string") return new Color(input);

  const coordinates = [
    input.coordinates[0] ?? null,
    input.coordinates[1] ?? null,
    input.coordinates[2] ?? null,
  ] as [number | null, number | null, number | null];

  return new Color(input.space, coordinates, input.alpha ?? 1);
}

function normalizeHexOutput(value: string): string {
  const hex = value.toLowerCase();
  if (/^#[0-9a-f]{3}$/.test(hex)) {
    return "#" + [...hex.slice(1)].map((character) => character + character).join("");
  }
  return hex;
}

function normalizeAmount(amount: number): number {
  return Math.max(0, Math.min(1, amount));
}

function toValue(color: Color, targetSpace?: ColorSpaceId): ColorValue {
  const resolved = targetSpace ? color.to(targetSpace) : color;
  const srgb = resolved.to("srgb");
  const mappedSrgb = srgb.inGamut("srgb")
    ? srgb
    : srgb.clone().toGamut({ space: "srgb", method: "css" });

  return {
    space: resolved.spaceId,
    coordinates: [...resolved.coords],
    alpha: resolved.alpha ?? 1,
    css: resolved.toString(),
    hex: normalizeHexOutput(mappedSrgb.toString({ format: "hex" })),
  };
}

export class ColorJsAdapter implements ColorEngine {
  parse(input: ColorInput): ColorValue {
    return toValue(toColor(input));
  }

  convert(input: ColorInput, targetSpace: ColorSpaceId): ColorValue {
    return toValue(toColor(input), targetSpace);
  }

  formatHex(input: ColorInput): string {
    return toValue(toColor(input)).hex ?? "#000000";
  }

  contrast(
    foreground: ColorInput,
    background: ColorInput,
    algorithm: ContrastAlgorithm = "wcag21",
  ): ContrastResult {
    const foregroundColor = toColor(foreground);
    const backgroundColor = toColor(background);
    const value =
      algorithm === "apca"
        ? backgroundColor.contrast(foregroundColor, "APCA")
        : backgroundColor.contrast(foregroundColor, "WCAG21");

    return {
      algorithm,
      value,
      foreground: this.formatHex(foreground),
      background: this.formatHex(background),
    };
  }

  difference(
    first: ColorInput,
    second: ColorInput,
    algorithm: DifferenceAlgorithm = "ok",
  ): ColorDifference {
    const firstColor = toColor(first);
    const secondColor = toColor(second);

    const value = {
      "76": () => firstColor.deltaE76(secondColor),
      "2000": () => firstColor.deltaE2000(secondColor),
      ok: () => firstColor.deltaEOK(secondColor),
      itp: () => firstColor.deltaEITP(secondColor),
      jz: () => firstColor.deltaEJz(secondColor),
      hct: () => firstColor.deltaE(secondColor, "HCT"),
    }[algorithm]();

    return { algorithm, value };
  }

  interpolate(
    first: ColorInput,
    second: ColorInput,
    amount: number,
    options: InterpolationOptions = {},
  ): ColorValue {
    const space = options.space ?? "oklch";
    const outputSpace = options.outputSpace ?? "srgb";
    const mixed = toColor(first).mix(toColor(second), normalizeAmount(amount), {
      space,
      outputSpace,
      hue: options.hue,
    });

    return toValue(mixed, outputSpace);
  }

  steps(
    first: ColorInput,
    second: ColorInput,
    count: number,
    options: InterpolationOptions = {},
  ): ColorValue[] {
    const total = Math.max(2, Math.floor(count));
    return Array.from({ length: total }, (_, index) =>
      this.interpolate(first, second, index / (total - 1), options),
    );
  }

  isInGamut(input: ColorInput, targetSpace: ColorSpaceId = "srgb"): boolean {
    return toColor(input).inGamut(targetSpace);
  }

  mapToGamut(input: ColorInput, options: GamutMapOptions = {}): ColorValue {
    const targetSpace = options.targetSpace ?? "srgb";
    const color = toColor(input).to(targetSpace);

    color.toGamut({
      space: targetSpace,
      method: options.method ?? "css",
    });

    return toValue(color, targetSpace);
  }
}
