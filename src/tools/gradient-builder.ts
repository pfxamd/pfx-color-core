import {
  colorEngine,
  type ColorInput,
  type ColorSpaceId,
  type ColorValue,
  type InterpolationOptions,
} from "../engine";

export type GradientType = "linear" | "radial" | "conic";

export interface GradientStopInput {
  color: ColorInput;
  position: number;
}

export interface GradientStop {
  index: number;
  position: number;
  source: ColorValue;
  value: ColorValue;
  hex: string;
}

export interface GradientDefinition {
  type: GradientType;
  angle: number;
  centerX: number;
  centerY: number;
  interpolationSpace: ColorSpaceId;
  targetSpace: ColorSpaceId;
  hue?: InterpolationOptions["hue"];
  stops: GradientStop[];
}

export interface GradientOptions {
  type?: GradientType;
  angle?: number;
  centerX?: number;
  centerY?: number;
  interpolationSpace?: ColorSpaceId;
  targetSpace?: ColorSpaceId;
  hue?: InterpolationOptions["hue"];
}

function checkedPosition(value: number): number {
  if (!Number.isFinite(value) || value < 0 || value > 1) {
    throw new RangeError("Gradient position must be between 0 and 1.");
  }
  return value;
}

function checkedUnit(value: number, label: string): number {
  if (!Number.isFinite(value) || value < 0 || value > 1) {
    throw new RangeError(label + " must be between 0 and 1.");
  }
  return value;
}

function toInput(value: ColorValue): ColorInput {
  return {
    space: value.space,
    coordinates: value.coordinates.map((coordinate) => coordinate ?? 0),
    alpha: value.alpha,
  };
}

export function createGradient(
  stops: readonly GradientStopInput[],
  options: GradientOptions = {},
): GradientDefinition {
  if (stops.length < 2) {
    throw new RangeError("A gradient requires at least 2 stops.");
  }

  const targetSpace = options.targetSpace ?? "srgb";
  const normalized = stops
    .map((stop, order) => ({
      ...stop,
      order,
      position: checkedPosition(stop.position),
    }))
    .sort((a, b) => a.position - b.position || a.order - b.order)
    .map((stop, index) => {
      const source = colorEngine.color.parse(stop.color);
      const raw = toInput(source);
      const value = colorEngine.color.isInGamut(raw, targetSpace)
        ? colorEngine.color.convert(raw, targetSpace)
        : colorEngine.color.mapToGamut(raw, { targetSpace, method: "css" });

      return {
        index,
        position: stop.position,
        source,
        value,
        hex: colorEngine.color.formatHex(toInput(value)),
      };
    });

  const rawAngle = options.angle ?? 90;
  const angle = ((rawAngle % 360) + 360) % 360;

  return {
    type: options.type ?? "linear",
    angle,
    centerX: checkedUnit(options.centerX ?? 0.5, "Gradient centerX"),
    centerY: checkedUnit(options.centerY ?? 0.5, "Gradient centerY"),
    interpolationSpace: options.interpolationSpace ?? "oklch",
    targetSpace,
    hue: options.hue,
    stops: normalized,
  };
}

export function sampleGradient(
  gradient: GradientDefinition,
  at: number,
): ColorValue {
  const target = checkedPosition(at);
  const stops = gradient.stops;

  if (target <= stops[0].position) return stops[0].value;
  if (target >= stops[stops.length - 1].position) return stops[stops.length - 1].value;

  const exact = [...stops].reverse().find((stop) => stop.position === target);
  if (exact) return exact.value;

  const rightIndex = stops.findIndex((stop) => stop.position > target);
  const left = stops[rightIndex - 1];
  const right = stops[rightIndex];
  const amount = (target - left.position) / (right.position - left.position);

  return colorEngine.color.interpolate(
    toInput(left.source),
    toInput(right.source),
    amount,
    {
      space: gradient.interpolationSpace,
      outputSpace: gradient.targetSpace,
      hue: gradient.hue,
    },
  );
}

export function gradientToCss(gradient: GradientDefinition): string {
  const interpolation =
    "in " +
    gradient.interpolationSpace +
    (gradient.hue ? " " + gradient.hue + " hue" : "");
  const stops = gradient.stops
    .map(
      (stop) =>
        stop.value.css +
        " " +
        String(Math.round(stop.position * 10000) / 100) +
        "%",
    )
    .join(", ");

  const center =
    String(Math.round(gradient.centerX * 10000) / 100) +
    "% " +
    String(Math.round(gradient.centerY * 10000) / 100) +
    "%";

  if (gradient.type === "radial") {
    return (
      "radial-gradient(circle at " +
      center +
      " " +
      interpolation +
      ", " +
      stops +
      ")"
    );
  }
  if (gradient.type === "conic") {
    return (
      "conic-gradient(from " +
      String(gradient.angle) +
      "deg at " +
      center +
      " " +
      interpolation +
      ", " +
      stops +
      ")"
    );
  }
  return (
    "linear-gradient(" +
    String(gradient.angle) +
    "deg " +
    interpolation +
    ", " +
    stops +
    ")"
  );
}
