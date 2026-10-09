import { describe, expect, it } from "vitest";
import {
  COLOR_STUDY_COUNT,
  COLOR_STUDY_SCHEMES,
  DEFAULT_COLOR_STUDY_CONTROLS,
  generateColorStudy,
} from "../../src/tools";
import { colorEngine } from "../../src/engine";

function makeRandom() {
  const sequence = [0.18, 0.72, 0.34, 0.91, 0.43, 0.62, 0.27, 0.79];
  let index = 0;
  return () => sequence[index++ % sequence.length];
}

function average(values: number[]) {
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function lightnessRange(study: ReturnType<typeof generateColorStudy>) {
  const values = study.colors.map((color) => color.oklch.l);
  return Math.max(...values) - Math.min(...values);
}

function hueDistance(a: number, b: number) {
  return Math.abs(((b - a + 540) % 360) - 180);
}

describe("generateColorStudy", () => {
  it("creates ten perceptually structured, gamut-safe colors", () => {
    const study = generateColorStudy("#ff0014", { random: makeRandom() });

    expect(study.colors).toHaveLength(COLOR_STUDY_COUNT);
    expect(COLOR_STUDY_SCHEMES).toContain(study.scheme);
    expect(study.controls).toEqual(DEFAULT_COLOR_STUDY_CONTROLS);

    for (const color of study.colors) {
      expect(color.hex).toMatch(/^#[0-9a-f]{6}$/i);
      expect(colorEngine.color.isInGamut(color.hex, "srgb")).toBe(true);
    }
  });

  it("moves the palette lightness center with LIGHTNESS", () => {
    const dark = generateColorStudy("#ff0014", {
      lightness: 18,
      random: makeRandom(),
    });
    const light = generateColorStudy("#ff0014", {
      lightness: 86,
      random: makeRandom(),
    });

    expect(
      average(light.colors.map((color) => color.oklch.l)),
    ).toBeGreaterThan(
      average(dark.colors.map((color) => color.oklch.l)) + 0.2,
    );
  });

  it("changes average chroma with CHROMA", () => {
    const muted = generateColorStudy("#ff0014", {
      chroma: 12,
      random: makeRandom(),
    });
    const vivid = generateColorStudy("#ff0014", {
      chroma: 92,
      random: makeRandom(),
    });

    expect(
      average(vivid.colors.map((color) => color.oklch.c)),
    ).toBeGreaterThan(
      average(muted.colors.map((color) => color.oklch.c)) + 0.05,
    );
  });

  it("widens hue dispersion with HUE RANGE", () => {
    const seedHue = Number(
      colorEngine.color.convert("#ff0014", "oklch").coordinates[2] ?? 0,
    );
    const tight = generateColorStudy("#ff0014", {
      hueRange: 8,
      random: makeRandom(),
    });
    const wide = generateColorStudy("#ff0014", {
      hueRange: 96,
      random: makeRandom(),
    });

    const hueSpread = (study: typeof tight) =>
      average(
        study.colors.map((color) => hueDistance(seedHue, color.oklch.h)),
      );

    expect(hueSpread(wide)).toBeGreaterThan(hueSpread(tight) + 15);
  });

  it("widens the light/dark span with TONE RANGE", () => {
    const tight = generateColorStudy("#ff0014", {
      toneRange: 8,
      random: makeRandom(),
    });
    const wide = generateColorStudy("#ff0014", {
      toneRange: 96,
      random: makeRandom(),
    });

    expect(lightnessRange(wide)).toBeGreaterThan(lightnessRange(tight) + 0.2);
  });

  it("clamps control values to the 0-100 domain", () => {
    const study = generateColorStudy("#336699", {
      lightness: -30,
      chroma: 130,
      hueRange: -1,
      toneRange: 101,
      random: () => 0.12,
    });

    expect(study.controls).toEqual({
      lightness: 0,
      chroma: 100,
      hueRange: 0,
      toneRange: 100,
    });
  });

  it("uses the seed while changing the generated study", () => {
    const first = generateColorStudy("#336699", { random: () => 0.12 });
    const second = generateColorStudy("#ffb000", { random: () => 0.12 });

    expect(first.seedHex).toBe("#336699");
    expect(second.seedHex).toBe("#ffb000");
    expect(first.colors.map((color) => color.hex)).not.toEqual(
      second.colors.map((color) => color.hex),
    );
  });
});
