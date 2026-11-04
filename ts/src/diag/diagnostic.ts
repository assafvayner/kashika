import { ERROR_CODES, LINT_CODES } from "./codes.js";
import { Float, type Json, Ordered, get } from "./json.js";
import { cmpStr, cmpTuple } from "./rust.js";

export class Span {
  constructor(
    readonly line: number,
    readonly col: number,
    readonly endLine: number,
    readonly endCol: number,
  ) {}

  static new(line: number, col: number, endLine: number, endCol: number): Span {
    return new Span(line, col, endLine, endCol);
  }

  static default(): Span {
    return new Span(0, 0, 0, 0);
  }

  to(other: Span): Span {
    return new Span(this.line, this.col, other.endLine, other.endCol);
  }

  eq(other: Span): boolean {
    return (
      this.line === other.line &&
      this.col === other.col &&
      this.endLine === other.endLine &&
      this.endCol === other.endCol
    );
  }

  toJson(): Json {
    return new Ordered([
      ["line", this.line],
      ["col", this.col],
      ["end_line", this.endLine],
      ["end_col", this.endCol],
    ]);
  }
}

export type Severity = "error" | "warn";

const SEVERITY_RANK: Record<Severity, number> = { error: 0, warn: 1 };

export function cmpSeverity(a: Severity, b: Severity): number {
  return SEVERITY_RANK[a] - SEVERITY_RANK[b];
}

export type Pair = [number, number];

function pairJson(p: Pair): Json {
  return [new Float(p[0]), new Float(p[1])];
}

/// An element a diagnostic is about, with enough geometry to locate it.
export class Element {
  /// 1-based index of the authored `-> ...` segment.
  segment: number | null = null;
  from: Pair | null = null;
  to: Pair | null = null;
  bbox: [Pair, Pair] | null = null;
  /// Id of the group whose frame the element's literals use.
  frame: string | null = null;
  localBbox: [Pair, Pair] | null = null;

  constructor(
    public id: string,
    /// `box`, `group`, `line`, `junction`, `text` or `label`.
    public kind: string,
    public span: Span,
  ) {}

  static new(id: string, kind: string, span: Span): Element {
    return new Element(id, kind, span);
  }

  toJson(): Json {
    const o: [string, Json][] = [
      ["id", this.id],
      ["kind", this.kind],
      ["span", this.span.toJson()],
    ];
    if (this.segment !== null) o.push(["segment", this.segment]);
    if (this.from !== null) o.push(["from", pairJson(this.from)]);
    if (this.to !== null) o.push(["to", pairJson(this.to)]);
    if (this.bbox !== null) o.push(["bbox", this.bbox.map(pairJson)]);
    if (this.frame !== null) o.push(["frame", this.frame]);
    if (this.localBbox !== null) o.push(["local_bbox", this.localBbox.map(pairJson)]);
    return new Ordered(o);
  }
}

/// A source edit the author may paste. Never applied automatically.
export class Suggestion {
  constructor(
    public span: Span,
    public replacement: string,
    /// Ids whose geometry would change.
    public ripple: string[],
  ) {}

  toJson(): Json {
    return new Ordered([
      ["span", this.span.toJson()],
      ["replacement", this.replacement],
      ["ripple", [...this.ripple]],
    ]);
  }
}

export class Diagnostic {
  elements: Element[] = [];
  facts: Map<string, Json> = new Map();
  help: string[] = [];
  suggestions: Suggestion[] = [];
  suppressed = false;
  reason: string | null = null;

  private constructor(
    public severity: Severity,
    public code: string,
    public span: Span,
    public message: string,
  ) {}

  /// An error; `code` must be in `ERROR_CODES`.
  static error(code: string, span: Span, message: string): Diagnostic {
    if (!ERROR_CODES.includes(code)) throw new Error(`\`${code}\` is not an error code`);
    return new Diagnostic("error", code, span, message);
  }

  /// A warning; `code` must be in `LINT_CODES`.
  static warn(code: string, span: Span, message: string): Diagnostic {
    if (!LINT_CODES.includes(code)) throw new Error(`\`${code}\` is not a lint code`);
    return new Diagnostic("warn", code, span, message);
  }

  withHelp(help: string): this {
    this.help.push(help);
    return this;
  }

  withElement(element: Element): this {
    this.elements.push(element);
    return this;
  }

  withFact(key: string, value: Json): this {
    this.facts.set(key, value);
    return this;
  }

  /// The `[x, y]` fact under `key`, if there is one.
  point(key: string): Pair | null {
    const v = this.facts.get(key);
    if (v === undefined) return null;
    const x = asF64(get(v, 0));
    const y = asF64(get(v, 1));
    return x === null || y === null ? null : [x, y];
  }

  toJson(): Json {
    const o: [string, Json][] = [
      ["severity", this.severity],
      ["code", this.code],
      ["message", this.message],
      ["span", this.span.toJson()],
    ];
    if (this.elements.length > 0) o.push(["elements", this.elements.map((e) => e.toJson())]);
    if (this.facts.size > 0) o.push(["facts", new Map(this.facts)]);
    if (this.help.length > 0) o.push(["help", [...this.help]]);
    o.push(["suggestions", this.suggestions.map((s) => s.toJson())]);
    if (this.suppressed) o.push(["suppressed", true]);
    if (this.reason !== null) o.push(["reason", this.reason]);
    return new Ordered(o);
  }
}

function asF64(v: Json): number | null {
  if (typeof v === "number") return v;
  if (v instanceof Float) return Number.isFinite(v.v) ? v.v : null;
  return null;
}

/// Sorts into `order`. The sort is stable, so diagnostics that `order` ties keep their order.
export function sort(diags: Diagnostic[]): void {
  diags.sort(order);
}

/// Source order: by start, then severity, code, end and message.
export function order(a: Diagnostic, b: Diagnostic): number {
  const key = (d: Diagnostic) => [
    d.span.line,
    d.span.col,
    SEVERITY_RANK[d.severity],
    d.code,
    d.span.endLine,
    d.span.endCol,
  ];
  const c = cmpTuple(key(a), key(b));
  return c !== 0 ? c : cmpStr(a.message, b.message);
}
