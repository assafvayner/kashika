import { LINT_CODES } from "../../diag/codes.js";
import { Diagnostic, Span } from "../../diag/diagnostic.js";
import { didYouMean } from "../../diag/distance.js";
import { chars, displayF64 } from "../../diag/rust.js";
import { type Attr, RESERVED_IDS } from "../../syntax/index.js";
import { LIMIT } from "../attrs.js";
import * as attrs from "../attrs.js";
import { type Resolver } from "../resolve.js";
import { Allow } from "../scene.js";

export function allowList(r: Resolver, a: Attr, checkIds: boolean): Allow[] {
  if (a.value.kind !== "Word") {
    r.diags.push(attrs.typeErr(a, "lint codes like `cross` or `cross:other_line`"));
    return [];
  }
  const w = a.value.value;
  const out: Allow[] = [];
  let col = entriesStart(a, w);
  for (const item of w.split(",")) {
    const len = chars(item).length;
    const span = col === null ? a.span : Span.new(a.span.line, col, a.span.line, col + len);
    col = col === null ? null : col + len + 1;
    const colon = item.indexOf(":");
    const [code, other] =
      colon >= 0 ? [item.slice(0, colon), item.slice(colon + 1)] : [item, null];
    if (!LINT_CODES.includes(code)) {
      const d = Diagnostic.error("attr", a.span, `unknown lint \`${code}\``);
      const s = didYouMean(code, LINT_CODES);
      if (s !== null) {
        d.withHelp(`did you mean \`${s}\`?`);
      }
      r.diags.push(d);
      continue;
    }
    if (other !== null) {
      if (!checkIds) {
        r.diags.push(
          Diagnostic.error("attr", a.span, "diagram-level `allow` cannot name another line"),
        );
        continue;
      }
      if (!r.ids.has(other)) {
        if (r.skippedIds.has(other)) {
          continue;
        }
        const d = unknownId(r, other, a.span);
        r.diags.push(d);
        continue;
      }
    }
    out.push(new Allow(code, other, span));
  }
  return out;
}

/// Checks that every value is a finite number within `LIMIT` of zero.
export function finite(r: Resolver, vals: readonly number[], span: Span): boolean {
  if (vals.some((v) => !Number.isFinite(v))) {
    r.diags.push(Diagnostic.error("range", span, "value is not a finite number"));
    return false;
  } else if (vals.some((v) => Math.abs(v) > LIMIT)) {
    r.diags.push(
      Diagnostic.error(
        "range",
        span,
        `value is out of range (more than ${displayF64(LIMIT)} from the origin)`,
      ).withHelp(`keep coordinates within -${displayF64(LIMIT)}..${displayF64(LIMIT)}`),
    );
    return false;
  } else {
    return true;
  }
}

export function sizeOk(r: Resolver, [w, h]: [number, number], span: Span, what: string): boolean {
  if (!finiteOnly(r, [w, h], span)) {
    return false;
  }
  let msg: string;
  if (w <= 0.0 || h <= 0.0) {
    msg = `${what} must have a positive size`;
  } else if (w > LIMIT || h > LIMIT) {
    msg = `${what} is larger than ${displayF64(LIMIT)}`;
  } else {
    return true;
  }
  r.diags.push(Diagnostic.error("size", span, msg));
  return false;
}

/// Like `finite`, but only rejects non-finite values.
export function finiteOnly(r: Resolver, vals: readonly number[], span: Span): boolean {
  return vals.every((v) => Number.isFinite(v)) || finite(r, vals, span);
}

export function unknownId(r: Resolver, name: string, span: Span): Diagnostic {
  const d = Diagnostic.error("unknown-id", span, `unknown id \`${name}\``);
  if (RESERVED_IDS.includes(name)) {
    d.withHelp(
      `\`${name}\` is a reserved word, so nothing can have that id; rename the shape it refers to`,
    );
  } else {
    const s = didYouMean(name, r.ids.keys());
    if (s !== null) {
      d.withHelp(`did you mean \`${s}\`?`);
    }
  }
  return d;
}

/// The column of the first entry of `allow=<w>`, when the attribute is written on one line
/// with no space around `=` or the entries, so each entry's column follows from its length.
function entriesStart(a: Attr, w: string): number | null {
  const s = a.span;
  const start = a.key.span.endCol + 1;
  return s.line === s.endLine &&
    a.key.span.endLine === s.line &&
    s.endCol === start + chars(w).length
    ? start
    : null;
}
