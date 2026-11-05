import type { Engine, RenderResult } from "../engine/types.js";

export type Origin = "agent" | "user";

const HISTORY = 100;

export class Document {
  source = $state("");
  result = $state.raw<RenderResult>({
    svg: null,
    errors: 0,
    warnings: 0,
    diagnostics: [],
    elements: [],
  });
  selection = $state<string | null>(null);
  /** Bumped on every change, for autosave. */
  version = $state(0);
  private history = $state.raw<{ source: string; origin: Origin }[]>([]);

  constructor(
    private readonly engine: Engine,
    source: string,
  ) {
    this.source = source;
    this.result = engine.render(source);
  }

  get canUndo(): boolean {
    return this.history.length > 0;
  }

  get title(): string {
    const m = /^\s*diagram\s+"((?:[^"\\]|\\.)*)"/m.exec(this.source);
    const title = m?.[1]?.replace(/\\n/g, " ").replace(/\\(.)/g, "$1");
    return title ? title : "Untitled";
  }

  /** Replaces the source and re-renders. Every change goes through here. */
  set(source: string, origin: Origin): RenderResult {
    if (source === this.source) return this.result;
    this.history = [...this.history.slice(-(HISTORY - 1)), { source: this.source, origin }];
    this.apply(source);
    return this.result;
  }

  undo(): void {
    const prev = this.history.at(-1);
    if (prev === undefined) return;
    this.history = this.history.slice(0, -1);
    this.apply(prev.source);
  }

  private apply(source: string): void {
    this.source = source;
    this.result = this.engine.render(source);
    if (this.selection !== null && !this.result.elements.some((e) => e.id === this.selection)) {
      this.selection = null;
    }
    this.version += 1;
  }
}
