import { describe, expect, it } from "vitest";
import { PfxColorsWorkspace } from "../../src/workspace";

describe("PfxColorsWorkspace", () => {
  it("shares one current color across tools", () => {
    const workspace = new PfxColorsWorkspace("#ff0000");
    workspace.generateHarmony("triadic");
    workspace.setColorFromHarmony(1);
    workspace.generateTonalPalette({ count: 5 });

    expect(workspace.getState().palette?.colors).toHaveLength(5);
  });

  it("transfers harmony and palette output into gradients", () => {
    const workspace = new PfxColorsWorkspace("#7c3aed");
    workspace.generateHarmony("square");
    workspace.createGradientFromHarmony({ type: "conic" });
    expect(workspace.getState().gradient?.stops).toHaveLength(4);

    workspace.generateTonalPalette({ count: 6 });
    workspace.createGradientFromPalette();
    expect(workspace.getState().gradient?.stops).toHaveLength(6);
  });

  it("undoes and redoes cross-tool operations", () => {
    const workspace = new PfxColorsWorkspace("#ff0000");
    workspace.generateHarmony("triadic");
    workspace.generatePaletteFromHarmony();
    expect(workspace.getState().palette).not.toBeNull();

    workspace.undo();
    expect(workspace.getState().palette).toBeNull();
    workspace.redo();
    expect(workspace.getState().palette).not.toBeNull();
  });

  it("returns isolated snapshots", () => {
    const workspace = new PfxColorsWorkspace("#7c3aed");
    const state = workspace.getState();
    state.color.hex = "#ffffff";
    expect(workspace.getState().color.hex).toBe("#7c3aed");
  });
});
