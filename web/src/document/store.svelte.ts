import type { Engine, RenderResult } from "../engine/types.js";

export type Origin = "agent" | "user";

const HISTORY = 100;

export class Document {
  source = $state("");
  result = $state<RenderResult>({
    svg: null,
    errors: 0,
    warnings: 0,
    diagnostics: [],
    elements: [],
  });
  selection = $state<string | null>(null);
  /** Bumped on every change, for autosave. */
  version = $state(0);
  private history: string[] = [];

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
    return m?.[1] ? m[1] : "Untitled";
  }

  /** Replaces the source and re-renders. Every change goes through here. */
  set(source: string, _origin: Origin): RenderResult {
    if (source === this.source) return this.result;
    this.history.push(this.source);
    if (this.history.length > HISTORY) this.history.shift();
    this.apply(source);
    return this.result;
  }

  undo(): void {
    const prev = this.history.pop();
    if (prev !== undefined) this.apply(prev);
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
