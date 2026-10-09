import { describe, expect, it } from "vitest";
import { createGradient, gradientToCss } from "../../src/tools/gradient-builder";

const stops = [
  { color: "#ff0000", position: 0 },
  { color: "#0000ff", position: 1 },
] as const;

describe("gradient geometry", () => {
  it("positions radial gradients", () => {
    const gradient = createGradient(stops, {
      type: "radial",
      centerX: 0.25,
      centerY: 0.75,
      interpolationSpace: "oklch",
    });
    expect(gradient.centerX).toBe(0.25);
    expect(gradient.centerY).toBe(0.75);
    expect(gradientToCss(gradient)).toContain("circle at 25% 75% in oklch");
  });

  it("positions and rotates conic gradients", () => {
    const gradient = createGradient(stops, {
      type: "conic",
      angle: 45,
      centerX: 0.2,
      centerY: 0.3,
      interpolationSpace: "oklch",
    });
    expect(gradientToCss(gradient)).toContain("from 45deg at 20% 30% in oklch");
  });

  it("rejects invalid centers", () => {
    expect(() =>
      createGradient(stops, { centerX: 1.1 }),
    ).toThrow(RangeError);
  });
});
