import type { ColorInput, ColorValue } from "../engine";
import {
  createGradient,
  generateHarmony,
  generatePaletteFromAnchors,
  generateRampPalette,
  generateTonalPalette,
  sampleGradient,
  selectColor,
  type GradientDefinition,
  type GradientOptions,
  type GradientStopInput,
  type HarmonyOptions,
  type HarmonyResult,
  type HarmonyScheme,
  type PaletteResult,
  type RampPaletteOptions,
  type TonalPaletteOptions,
} from "../tools";

export interface WorkspaceState {
  color: ReturnType<typeof selectColor>;
  palette: PaletteResult | null;
  harmony: HarmonyResult | null;
  gradient: GradientDefinition | null;
}

function toInput(value: ColorValue): ColorInput {
  return {
    space: value.space,
    coordinates: value.coordinates.map((coordinate) => coordinate ?? 0),
    alpha: value.alpha,
  };
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

export class PfxColorsWorkspace {
  private state: WorkspaceState;
  private past: WorkspaceState[] = [];
  private future: WorkspaceState[] = [];
  private readonly historyLimit: number;

  constructor(initialColor: ColorInput = "#ff0014", historyLimit = 100) {
    if (!Number.isInteger(historyLimit) || historyLimit < 1 || historyLimit > 1000) {
      throw new RangeError("History limit must be between 1 and 1000.");
    }

    this.state = {
      color: selectColor(initialColor),
      palette: null,
      harmony: null,
      gradient: null,
    };
    this.historyLimit = historyLimit;
  }

  getState(): WorkspaceState {
    return clone(this.state);
  }

  canUndo(): boolean {
    return this.past.length > 0;
  }

  canRedo(): boolean {
    return this.future.length > 0;
  }

  setColor(input: ColorInput): WorkspaceState {
    return this.commit({ ...this.state, color: selectColor(input) });
  }

  generateTonalPalette(options?: TonalPaletteOptions): WorkspaceState {
    return this.commit({
      ...this.state,
      palette: generateTonalPalette(toInput(this.state.color.source), options),
    });
  }

  generateRampPalette(
    second: ColorInput,
    options?: RampPaletteOptions,
  ): WorkspaceState {
    return this.commit({
      ...this.state,
      palette: generateRampPalette(toInput(this.state.color.source), second, options),
    });
  }

  generateHarmony(
    scheme: HarmonyScheme,
    options?: HarmonyOptions,
  ): WorkspaceState {
    return this.commit({
      ...this.state,
      harmony: generateHarmony(toInput(this.state.color.source), scheme, options),
    });
  }

  generatePaletteFromHarmony(
    options?: RampPaletteOptions,
  ): WorkspaceState {
    if (!this.state.harmony) throw new Error("No harmony available.");

    return this.commit({
      ...this.state,
      palette: generatePaletteFromAnchors(
        this.state.harmony.colors.map((color) => toInput(color.value)),
        {
          ...options,
          count: options?.count ?? this.state.harmony.colors.length,
        },
      ),
    });
  }

  createGradient(
    stops: readonly GradientStopInput[],
    options?: GradientOptions,
  ): WorkspaceState {
    return this.commit({
      ...this.state,
      gradient: createGradient(stops, options),
    });
  }

  createGradientFromPalette(options?: GradientOptions): WorkspaceState {
    if (!this.state.palette) throw new Error("No palette available.");

    return this.createGradient(
      this.state.palette.colors.map((color) => ({
        color: toInput(color.value),
        position: color.position,
      })),
      options,
    );
  }

  createGradientFromHarmony(options?: GradientOptions): WorkspaceState {
    if (!this.state.harmony) throw new Error("No harmony available.");
    const last = this.state.harmony.colors.length - 1;

    return this.createGradient(
      this.state.harmony.colors.map((color, index) => ({
        color: toInput(color.value),
        position: last === 0 ? 0 : index / last,
      })),
      options,
    );
  }

  setColorFromPalette(index: number): WorkspaceState {
    const color = this.state.palette?.colors[index];
    if (!color) throw new RangeError("Palette color index is out of range.");
    return this.setColor(toInput(color.value));
  }

  setColorFromHarmony(index: number): WorkspaceState {
    const color = this.state.harmony?.colors[index];
    if (!color) throw new RangeError("Harmony color index is out of range.");
    return this.setColor(toInput(color.value));
  }

  setColorFromGradient(position: number): WorkspaceState {
    if (!this.state.gradient) throw new Error("No gradient available.");
    return this.setColor(toInput(sampleGradient(this.state.gradient, position)));
  }

  undo(): WorkspaceState {
    const previous = this.past.pop();
    if (!previous) return this.getState();

    this.future.push(clone(this.state));
    this.state = clone(previous);
    return this.getState();
  }

  redo(): WorkspaceState {
    const next = this.future.pop();
    if (!next) return this.getState();

    this.past.push(clone(this.state));
    this.trimHistory();
    this.state = clone(next);
    return this.getState();
  }

  clearHistory(): WorkspaceState {
    this.past = [];
    this.future = [];
    return this.getState();
  }

  private commit(next: WorkspaceState): WorkspaceState {
    this.past.push(clone(this.state));
    this.trimHistory();
    this.future = [];
    this.state = clone(next);
    return this.getState();
  }

  private trimHistory(): void {
    while (this.past.length > this.historyLimit) this.past.shift();
  }
}
