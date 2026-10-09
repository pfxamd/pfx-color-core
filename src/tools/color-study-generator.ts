import {
  colorEngine,
  type ColorInput,
  type ColorValue,
} from "../engine";
import {
  generateHarmony,
  type HarmonyScheme,
} from "./harmony-generator";

export const COLOR_STUDY_COUNT = 10;

export const COLOR_STUDY_SCHEMES = [
  "analogous",
  "split-complementary",
  "triadic",
  "tetradic",
  "square",
] as const satisfies readonly HarmonyScheme[];

export interface ColorStudyColor {
  index: number;
  hex: string;
  value: ColorValue;
  oklch: {
    l: number;
    c: number;
    h: number;
  };
  mapped: boolean;
}

export interface ColorStudyControls {
  lightness: number;
  chroma: number;
  hueRange: number;
  toneRange: number;
}

export interface ColorStudyResult {
  seedHex: string;
  scheme: HarmonyScheme;
  controls: ColorStudyControls;
  colors: ColorStudyColor[];
}

export interface ColorStudyOptions extends Partial<ColorStudyControls> {
  random?: () => number;
  targetSpace?: "srgb" | "p3";
}

export const DEFAULT_COLOR_STUDY_CONTROLS: Readonly<ColorStudyControls> = Object.freeze({
  lightness: 58,
  chroma: 58,
  hueRange: 58,
  toneRange: 58,
});

const LIGHTNESS_TRACK = [-0.48, -0.30, 0.34, -0.14, 0.20, -0.58, 0.48, -0.22, 0.10, -0.04];
const CHROMA_TRACK = [1.0, 0.72, 0.58, 1.18, 0.88, 0.52, 0.4, 1.08, 0.78, 0.94];

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}

function normalizedControl(
  value: number | undefined,
  fallback: number,
) {
  return clamp(value ?? fallback, 0, 100);
}

function wrapHue(value: number) {
  const hue = value % 360;
  return hue < 0 ? hue + 360 : hue;
}

function hueDelta(from: number, to: number) {
  return ((to - from + 540) % 360) - 180;
}

function asInput(value: ColorValue): ColorInput {
  return {
    space: value.space,
    coordinates: value.coordinates.map((coordinate) => coordinate ?? 0),
    alpha: value.alpha,
  };
}

function distanceFromExisting(candidate: ColorInput, colors: ColorStudyColor[]) {
  if (colors.length === 0) return Number.POSITIVE_INFINITY;
  return Math.min(
    ...colors.map((color) =>
      colorEngine.color.difference(candidate, asInput(color.value), "ok").value,
    ),
  );
}

export function generateColorStudy(
  seed: ColorInput,
  options: ColorStudyOptions = {},
): ColorStudyResult {
  const random = options.random ?? Math.random;
  const targetSpace = options.targetSpace ?? "srgb";

  const controls: ColorStudyControls = {
    lightness: normalizedControl(
      options.lightness,
      DEFAULT_COLOR_STUDY_CONTROLS.lightness,
    ),
    chroma: normalizedControl(
      options.chroma,
      DEFAULT_COLOR_STUDY_CONTROLS.chroma,
    ),
    hueRange: normalizedControl(
      options.hueRange,
      DEFAULT_COLOR_STUDY_CONTROLS.hueRange,
    ),
    toneRange: normalizedControl(
      options.toneRange,
      DEFAULT_COLOR_STUDY_CONTROLS.toneRange,
    ),
  };

  const lightnessAmount = controls.lightness / 100;
  const chromaAmount = controls.chroma / 100;
  const hueRangeAmount = controls.hueRange / 100;
  const toneRangeAmount = controls.toneRange / 100;

  const scheme =
    COLOR_STUDY_SCHEMES[
      Math.min(
        COLOR_STUDY_SCHEMES.length - 1,
        Math.floor(random() * COLOR_STUDY_SCHEMES.length),
      )
    ];

  const seedValue = colorEngine.color.parse(seed);
  const seedOklch = colorEngine.color.convert(seed, "oklch");
  const seedHue =
    seedOklch.coordinates[2] == null
      ? random() * 360
      : Number(seedOklch.coordinates[2]);
  const seedChroma = Number(seedOklch.coordinates[1] ?? 0);

  const lightnessCenter = 0.18 + lightnessAmount * 0.68;

  const requestedChroma =
    0.018 +
    chromaAmount * 0.282 +
    Math.min(seedChroma, 0.18) * 0.16;
  const workingChroma = clamp(requestedChroma, 0.018, 0.32);

  const harmony = generateHarmony(
    {
      space: "oklch",
      coordinates: [
        lightnessCenter,
        workingChroma,
        seedHue,
      ],
      alpha: seedValue.alpha,
    },
    scheme,
    { targetSpace: "p3" },
  );

  const hueSpread = 0.06 + hueRangeAmount * 1.08;
  const anchorHues = harmony.colors.map((color) => {
    const converted = colorEngine.color.convert(asInput(color.value), "oklch");
    const rawHue = Number(converted.coordinates[2] ?? seedHue);
    return wrapHue(seedHue + hueDelta(seedHue, rawHue) * hueSpread);
  });

  const toneAmplitude = 0.055 + toneRangeAmount * 0.47;
  const chromaSpread = 0.24 + chromaAmount * 0.82;
  const minimumDistance =
    0.014 +
    hueRangeAmount * 0.024 +
    toneRangeAmount * 0.022;
  const colors: ColorStudyColor[] = [];

  for (let index = 0; index < COLOR_STUDY_COUNT; index += 1) {
    const anchorHue = anchorHues[(index * 2 + 1) % anchorHues.length];
    let candidate: ColorInput = seed;
    let mapped = false;

    for (let attempt = 0; attempt < 8; attempt += 1) {
      const hueJitter =
        (random() - 0.5) *
        ((3 + hueRangeAmount * 22) + attempt * (1.5 + hueRangeAmount * 4));
      const lightnessJitter =
        (random() - 0.5) * (0.008 + toneRangeAmount * 0.042);
      const chromaJitter =
        (random() - 0.5) * (0.006 + chromaAmount * 0.028);

      const lightness = clamp(
        lightnessCenter +
          LIGHTNESS_TRACK[index] * toneAmplitude +
          lightnessJitter +
          attempt * 0.002,
        0.06,
        0.98,
      );

      const trackedChroma =
        1 + (CHROMA_TRACK[index] - 1) * chromaSpread;
      const chroma = clamp(
        workingChroma * trackedChroma + chromaJitter,
        index === 6 ? 0.008 : 0.012,
        0.36,
      );

      const hue = wrapHue(
        seedHue +
          hueDelta(seedHue, anchorHue) +
          hueJitter +
          attempt * (1.5 + hueRangeAmount * 4),
      );

      candidate = {
        space: "oklch",
        coordinates: [lightness, chroma, hue],
        alpha: seedValue.alpha,
      };

      if (
        distanceFromExisting(candidate, colors) >= minimumDistance ||
        attempt === 7
      ) {
        break;
      }
    }

    mapped = !colorEngine.color.isInGamut(candidate, targetSpace);
    const value = mapped
      ? colorEngine.color.mapToGamut(candidate, {
          targetSpace,
          method: "css",
        })
      : colorEngine.color.convert(candidate, targetSpace);
    const perceptual = colorEngine.color.convert(asInput(value), "oklch");

    colors.push({
      index,
      hex: colorEngine.color.formatHex(asInput(value)),
      value,
      oklch: {
        l: Number(perceptual.coordinates[0] ?? 0),
        c: Number(perceptual.coordinates[1] ?? 0),
        h: Number(perceptual.coordinates[2] ?? 0),
      },
      mapped,
    });
  }

  return {
    seedHex: colorEngine.color.formatHex(seed),
    scheme,
    controls,
    colors,
  };
}
