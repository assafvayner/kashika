import { createHash } from "node:crypto";

import { type Ctx, type Deferred, type Metrics, type Omitted, free } from "../check/index.js";
import { type Diagnostic } from "../diag/diagnostic.js";
import { type Json, toJsonString, toValue } from "../diag/json.js";
import { cmpStr, lines as rustLines } from "../diag/rust.js";
import { TOOL_VERSION } from "../model/version.js";
import * as term from "./term.js";

export const SCHEMA = "kashika.check/1";

export interface Output {
  path: string;
  written: boolean;
}

export interface Opts {
  max: number;
  showSuppressed: boolean;
  summary: boolean;
  metrics: Metrics | null;
  deferred: Deferrals | null;
  /// Warnings counted but not kept, past `KEEP` (or `max`) per code.
  omitted: readonly Omitted[];
}

/// The help `check` deferred for each diagnostic, in the same order, and the context it is
/// worked out in.
export type Deferrals = [Ctx, readonly Deferred[][]];

/// How many warnings per code `check` keeps at least, so that memory stays bounded; the
/// rest are only counted.
export const KEEP = 1000;

export interface Counts {
  errors: number;
  warns: number;
  suppressed: number;
}

export function sha256(src: string): string {
  return createHash("sha256").update(src, "utf8").digest("hex");
}

export function counts(diags: readonly Diagnostic[], omitted: readonly Omitted[]): Counts {
  const errors = diags.filter((d) => d.severity === "error").length;
  const suppressed = diags.filter((d) => d.suppressed).length;
  const more = (s: boolean): number =>
    omitted.filter((o) => o.suppressed === s).reduce((n, o) => n + o.count, 0);
  return {
    errors,
    warns: diags.length - errors - suppressed + more(false),
    suppressed: suppressed + more(true),
  };
}

/// Omitted warnings that would be visible.
function omittedVisible(opts: Opts): number {
  return opts.omitted
    .filter((o) => opts.showSuppressed || !o.suppressed)
    .reduce((n, o) => n + o.count, 0);
}

/// Per code: how many findings are active and how many are suppressed.
export function byCode(
  diags: readonly Diagnostic[],
  omitted: readonly Omitted[],
): [string, [number, number]][] {
  const m = new Map<string, [number, number]>();
  const entry = (code: string): [number, number] => {
    let e = m.get(code);
    if (e === undefined) {
      e = [0, 0];
      m.set(code, e);
    }
    return e;
  };
  for (const d of diags) {
    const e = entry(d.code);
    if (d.suppressed) e[1] += 1;
    else e[0] += 1;
  }
  for (const o of omitted) {
    const e = entry(o.code);
    if (o.suppressed) e[1] += o.count;
    else e[0] += o.count;
  }
  return [...m.entries()].sort((a, b) => cmpStr(a[0], b[0]));
}

export function plural(n: number, word: string): string {
  return n === 1 ? `1 ${word}` : `${n} ${word}s`;
}

export function visible(diags: readonly Diagnostic[], showSuppressed: boolean): Diagnostic[] {
  return diags.filter((d) => showSuppressed || !d.suppressed);
}

/// What the report does with a visible diagnostic: print it, or cut it by `--max`.
type Show = "print" | "over";

/// Every error is printed; warnings fill what is left of `max`, in source order.
function plan(shown: readonly Diagnostic[], max: number): Show[] {
  const errors = shown.filter((d) => d.severity === "error").length;
  let budget = Math.max(max - errors, 0);
  return shown.map((d) => {
    if (d.severity !== "error") {
      if (budget === 0) {
        return "over";
      }
      budget -= 1;
    }
    return "print";
  });
}

interface Selection {
  /// Indices into the diagnostics of the visible ones.
  idx: number[];
  plan: Show[];
}

/// Picks what to show, errors first and each severity in source order, and fills in the
/// deferred help of the diagnostics that are printed.
function select(diags: Diagnostic[], opts: Opts): Selection {
  const idx = diags.map((_, i) => i).filter((i) => opts.showSuppressed || !diags[i]!.suppressed);
  const key = (i: number) => (diags[i]!.severity !== "error" ? 1 : 0);
  idx.sort((a, b) => key(a) - key(b));
  const shown = idx.map((i) => diags[i]!);
  const p = plan(shown, opts.max);
  if (opts.deferred !== null) {
    const [ctx, deferred] = opts.deferred;
    idx.forEach((i, k) => {
      if (p[k] === "print") {
        free.resolve(ctx, diags[i]!, [...deferred[i]!]);
      }
    });
  }
  return { idx, plan: p };
}

export function text(
  name: string,
  src: string,
  diags: Diagnostic[],
  opts: Opts,
  output: Output | null,
): string {
  const sel = select(diags, opts);
  const shownName = term.line(name);
  const lines = rustLines(src);
  let out = "";
  const source = (line: number) => {
    const l = line >= 1 ? lines[line - 1] : undefined;
    if (l !== undefined) {
      out += `${String(line).padStart(5)} | ${term.line(l)}\n`;
    }
  };
  let overflow = omittedVisible(opts);
  sel.idx.forEach((i, k) => {
    if (sel.plan[k] === "over") {
      overflow += 1;
      return;
    }
    const d = diags[i]!;
    const sev = d.severity === "error" ? "error" : "warn";
    const note = d.suppressed ? " (suppressed)" : "";
    out += `${shownName}:${d.span.line}:${d.span.col}: ${sev}[${d.code}]: ${term.line(d.message)}${note}\n`;
    const seen = [d.span.line];
    source(d.span.line);
    for (const e of d.elements) {
      if (!seen.includes(e.span.line)) {
        seen.push(e.span.line);
        source(e.span.line);
      }
    }
    for (const h of d.help) {
      out += `      = help: ${term.line(h)}\n`;
    }
    if (d.reason !== null) {
      out += `      = reason: ${term.line(d.reason)}\n`;
    }
  });
  if (overflow > 0) {
    out += `... and ${overflow} more\n`;
  }
  if (opts.summary) {
    out += "by code:\n";
    for (const [code, [active, suppressed]] of byCode(diags, opts.omitted)) {
      out += `  ${code}: ${active}`;
      if (suppressed > 0) {
        out += ` (${suppressed} suppressed)`;
      }
      out += "\n";
    }
  }
  const c = counts(diags, opts.omitted);
  out += `${plural(c.errors, "error")}, ${plural(c.warns, "warning")}`;
  if (c.suppressed > 0) {
    out += ` (${c.suppressed} suppressed)`;
  }
  if (opts.metrics !== null) {
    out += ` · crossings=${opts.metrics.crossings} wire=${opts.metrics.wirePx}px`;
  }
  if (output !== null) {
    out += output.written ? " · output: written" : " · output: not written";
  }
  out += "\n";
  return out;
}

export function json(
  name: string,
  src: string,
  diags: Diagnostic[],
  opts: Opts,
  output: Output | null,
): string {
  const sel = select(diags, opts);
  const listed = sel.idx.filter((_, k) => sel.plan[k] === "print").map((i) => diags[i]!);
  const c = counts(diags, opts.omitted);
  const v: Json = {
    schema: SCHEMA,
    tool: TOOL_VERSION,
    input: { name, sha256: sha256(src) },
    output: output === null ? null : { path: output.path, written: output.written },
    counts: { error: c.errors, warn: c.warns, suppressed: c.suppressed },
    by_code: new Map(
      byCode(diags, opts.omitted).map(([k, [active, suppressed]]) => [k, { active, suppressed }]),
    ),
    truncated: listed.length < sel.idx.length || omittedVisible(opts) > 0,
    metrics:
      opts.metrics === null
        ? null
        : { crossings: opts.metrics.crossings, wire_px: opts.metrics.wirePx },
    diagnostics: listed.map((d) => d.toJson()),
  };
  return `${toJsonString(toValue(v))}\n`;
}
