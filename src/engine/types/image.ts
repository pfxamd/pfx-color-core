export interface ImageRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ImageExtractionOptions {
  colorCount?: number;
  quality?: number;
  colorSpace?: "rgb" | "oklch";
  gamut?: "srgb" | "display-p3" | "auto";
  region?: ImageRegion;
  ignoreWhite?: boolean;
  minSaturation?: number;
  signal?: AbortSignal;
}

export interface ExtractedColor {
  hex: string;
  css: string;
  rgb: { r: number; g: number; b: number };
  oklch: { l: number; c: number; h: number };
  population: number;
  proportion: number;
  gamut: "srgb" | "display-p3";
}

export interface PaletteExtractionResult {
  colors: ExtractedColor[];
}

export interface ImagePaletteExtractor {
  extractPalette(source: unknown, options?: ImageExtractionOptions): Promise<PaletteExtractionResult>;
  extractDominantColor(source: unknown, options?: ImageExtractionOptions): Promise<ExtractedColor | null>;
}
