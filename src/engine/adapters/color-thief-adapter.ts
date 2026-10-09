import {
  getColor,
  getPalette,
  type Color as ColorThiefColor,
  type ExtractionOptions,
  type ImageSource,
} from "colorthief";
import type {
  ExtractedColor,
  ImageExtractionOptions,
  ImagePaletteExtractor,
  PaletteExtractionResult,
} from "../types/image";

function toExtractionOptions(options: ImageExtractionOptions = {}): ExtractionOptions {
  return {
    colorCount: options.colorCount,
    quality: options.quality,
    colorSpace: options.colorSpace ?? "oklch",
    gamut: options.gamut ?? "srgb",
    region: options.region,
    ignoreWhite: options.ignoreWhite,
    minSaturation: options.minSaturation,
    signal: options.signal,
  };
}

function toExtractedColor(color: ColorThiefColor): ExtractedColor {
  return {
    hex: color.hex(),
    css: color.css("oklch"),
    rgb: color.rgb(),
    oklch: color.oklch(),
    population: color.population,
    proportion: color.proportion,
    gamut: color.gamut,
  };
}

export class ColorThiefAdapter implements ImagePaletteExtractor {
  async extractPalette(
    source: unknown,
    options: ImageExtractionOptions = {},
  ): Promise<PaletteExtractionResult> {
    const colors =
      (await getPalette(source as ImageSource, toExtractionOptions(options))) ?? [];

    return { colors: colors.map(toExtractedColor) };
  }

  async extractDominantColor(
    source: unknown,
    options: ImageExtractionOptions = {},
  ): Promise<ExtractedColor | null> {
    const color = await getColor(source as ImageSource, toExtractionOptions(options));
    return color ? toExtractedColor(color) : null;
  }
}
