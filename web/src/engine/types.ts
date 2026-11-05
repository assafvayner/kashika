export interface Span {
  line: number;
  col: number;
  end_line: number;
  end_col: number;
}

export interface DiagElement {
  id: string;
  kind: string;
  span: Span;
  bbox?: [[number, number], [number, number]];
}

export interface Diagnostic {
  severity: "error" | "warn";
  code: string;
  message: string;
  span: Span;
  elements?: DiagElement[];
  help?: string[];
  suppressed?: boolean;
}

export interface Elem {
  id: string;
  kind: "box" | "group" | "image" | "text";
  parent: string | null;
  depth: number;
  bbox: [[number, number], [number, number]];
}

export interface RenderResult {
  /** Present unless the source is a library or does not parse to a scene. */
  svg: string | null;
  errors: number;
  warnings: number;
  diagnostics: Diagnostic[];
  elements: Elem[];
}

export interface CheckResult {
  errors: number;
  warnings: number;
  diagnostics: Diagnostic[];
  text: string;
}

export interface RouteOptions {
  style?: "corner" | "arc" | "bezier";
  line?: string;
  radius?: number;
  max?: number;
}

export interface Measurement {
  width: number;
  height: number;
  lines: number;
}

export interface IconRow {
  name: string;
  path: string;
  size: [number, number] | null;
  library: string;
  matched_keyword?: string;
}

export class EngineError extends Error {
  constructor(
    message: string,
    public readonly kind: string = "error",
  ) {
    super(message);
    this.name = "EngineError";
  }
}

export interface Engine {
  render(source: string): RenderResult;
  check(source: string): CheckResult;
  inspect(source: string, ids?: string[], ports?: boolean): string;
  route(source: string, from: string | null, to: string | null, opts?: RouteOptions): string;
  /** `route` with the first candidate's statement, for applying suggestions in sequence. */
  routeFull(
    source: string,
    from: string | null,
    to: string | null,
    opts?: RouteOptions,
  ): { text: string; statement: string | null };
  measure(text: string, size?: number, bold?: boolean): Measurement;
  format(source: string): string;
  ref(topic?: string): string;
  icons(search?: string): IconRow[];
  version(): string;
}
