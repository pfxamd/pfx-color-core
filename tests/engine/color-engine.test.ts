import { describe, expect, it } from "vitest";
import { PfxColorEngine } from "../../src/engine";

describe("PfxColorEngine", () => {
  const engine = new PfxColorEngine();

  it("converts sRGB to OKLab with reference precision", () => {
    const result = engine.color.convert("#336699", "oklab");
    expect(result.coordinates[0]).toBeCloseTo(0.49931445584520834, 12);
    expect(result.coordinates[1]).toBeCloseTo(-0.03304348760594705, 12);
    expect(result.coordinates[2]).toBeCloseTo(-0.09296659206477714, 12);
  });

  it("round-trips a supported color without visible drift", () => {
    const oklab = engine.color.convert("#7c3aed", "oklab");
    const back = engine.color.convert(
      {
        space: "oklab",
        coordinates: oklab.coordinates.map((value) => value ?? 0),
      },
      "srgb",
    );
    expect(back.hex).toBe("#7c3aed");
  });

  it("returns WCAG and APCA contrast", () => {
    expect(engine.color.contrast("#000", "#fff", "wcag21").value).toBeCloseTo(21, 10);
    expect(engine.color.contrast("#000", "#fff", "apca").value).toBeGreaterThan(100);
  });

  it("matches a CIEDE2000 reference pair", () => {
    const result = engine.color.difference(
      { space: "lab", coordinates: [50, 2.6772, -79.7751] },
      { space: "lab", coordinates: [50, 0, -82.7485] },
      "2000",
    );
    expect(result.value).toBeCloseTo(2.0425, 3);
  });

  it("maps out-of-gamut OKLCH to sRGB", () => {
    const mapped = engine.color.mapToGamut({
      space: "oklch",
      coordinates: [0.7, 0.4, 40],
    });
    expect(mapped.hex).toMatch(/^#[0-9a-f]{6}$/i);
    expect(engine.color.isInGamut(mapped.hex ?? "#000000", "srgb")).toBe(true);
  });
});
