import { describe, expect, it } from "vitest";
import {
  createGradient,
  generateHarmony,
  generatePaletteFromAnchors,
  generateTonalPalette,
  gradientToCss,
  sampleGradient,
  selectColor,
  setColorChannel,
} from "../../src/tools";

describe("color tools", () => {
  it("selects and edits colors", () => {
    expect(selectColor("#7c3aed").hex).toBe("#7c3aed");
    expect(setColorChannel("#ff0000", "hsl", 0, 120).hex).toBe("#00ff00");
  });

  it("builds tonal and anchored palettes", () => {
    const tonal = generateTonalPalette("#7c3aed", { count: 7 });
    expect(tonal.colors).toHaveLength(7);

    const anchored = generatePaletteFromAnchors(
      ["#ff0000", "#00ff00", "#0000ff"],
      { count: 9 },
    );
    expect(anchored.colors).toHaveLength(9);
    expect(anchored.colors[0].hex).toBe("#ff0000");
    expect(anchored.colors[8].hex).toBe("#0000ff");
  });

  it("builds standard harmonies", () => {
    expect(generateHarmony("#ff0000", "complementary").colors).toHaveLength(2);
    expect(generateHarmony("#ff0000", "triadic").colors).toHaveLength(3);
    expect(generateHarmony("#ff0000", "square").colors).toHaveLength(4);
  });

  it("builds and samples multi-stop gradients", () => {
    const gradient = createGradient([
      { color: "#ff0000", position: 0 },
      { color: "#00ff00", position: 0.5 },
      { color: "#0000ff", position: 1 },
    ]);

    expect(gradient.stops).toHaveLength(3);
    expect(sampleGradient(gradient, 0.5).hex).toBe("#00ff00");
    expect(gradientToCss(gradient).startsWith("linear-gradient(")).toBe(true);
  });
});
