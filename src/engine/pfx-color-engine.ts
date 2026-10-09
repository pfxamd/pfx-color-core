import { ColorJsAdapter } from "./adapters/color-js-adapter";
import type { ColorEngine } from "./types/color";
import type { ImagePaletteExtractor } from "./types/image";

export class PfxColorEngine {
  readonly color: ColorEngine;
  readonly images: ImagePaletteExtractor | null;

  constructor(
    colorEngine: ColorEngine = new ColorJsAdapter(),
    imageExtractor: ImagePaletteExtractor | null = null,
  ) {
    this.color = colorEngine;
    this.images = imageExtractor;
  }
}

export const colorEngine = new PfxColorEngine();
