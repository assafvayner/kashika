import {
  entries,
  Float,
  get,
  isObject,
  type Json,
  jsonString,
  type JsonObject,
  toJsonString,
} from "../diag/json.js";
import { lines as rustLines, trim, trimStart, utf8Len } from "../diag/rust.js";
import { fmtNum } from "./num.js";

const FOLD_WIDTH = 100;
const VERSION = "0.1.0-dev0";

/// `{:#?}` text with spans written `line:col-end_line:end_col` and short leaf-only groups
/// folded onto one line.
export function foldDebug(text: string): string {
  let lines = rustLines(text);
  for (;;) {
    const folded = foldOnce(lines);
    if (folded.length === lines.length && folded.every((l, i) => l === lines[i])) {
      break;
    }
    lines = folded;
  }
  return `${lines.join("\n")}\n`;
}

function indent(s: string): number {
  return utf8Len(s) - utf8Len(trimStart(s));
}

function opener(s: string): string | null {
  const cs = Array.from(s);
  const c = cs[cs.length - 1];
  return c === "(" || c === "[" || c === "{" ? c : null;
}

function foldOnce(lines: string[]): string[] {
  const out: string[] = [];
  let i = 0;
  while (i < lines.length) {
    const open = opener(lines[i]!);
    if (open !== null) {
      const ind = indent(lines[i]!);
      let j = -1;
      for (let k = i + 1; k < lines.length; k++) {
        if (indent(lines[k]!) === ind) {
          j = k;
          break;
        }
      }
      if (j !== -1) {
        const inner = lines.slice(i + 1, j);
        const leaf = inner.every((l) => indent(l) === ind + 4 && opener(l) === null);
        if (leaf) {
          const parts = inner.map((l) => trimEndMatches(trim(l), ","));
          const body = parts.join(", ");
          const closing = trim(lines[j]!);
          let joined =
            open === "{" ? `${lines[i]} ${body} ${closing}` : `${lines[i]}${body}${closing}`;
          joined = compactSpan(joined);
          if (utf8Len(joined) <= FOLD_WIDTH) {
            out.push(joined);
            i = j + 1;
            continue;
          }
        }
      }
    }
    out.push(lines[i]!);
    i += 1;
  }
  return out;
}

function trimEndMatches(s: string, suffix: string): string {
  while (s.endsWith(suffix)) s = s.slice(0, -suffix.length);
  return s;
}

/// `Span { line: 4, col: 1, end_line: 4, end_col: 33 }` becomes `4:1-4:33`.
function compactSpan(s: string): string {
  const start = s.indexOf("Span { line: ");
  if (start === -1) {
    return s;
  }
  const len = s.slice(start).indexOf(" }");
  if (len === -1) {
    return s;
  }
  const inner = s.slice(start + "Span { ".length, start + len);
  const nums = inner
    .split(", ")
    .map((kv) => kv.split(": ")[1])
    .filter((n): n is string => n !== undefined);
  if (nums.length !== 4) {
    return s;
  }
  return `${s.slice(0, start)}${nums[0]}:${nums[1]}-${nums[2]}:${nums[3]}${s.slice(start + len + 2)}`;
}

/// One-line JSON with every float rounded by `fmtNum`.
export function compact(v: Json): string {
  if (v instanceof Float) {
    return Number.isFinite(v.v) ? fmtNum(v.v) : "null";
  }
  if (typeof v === "number") {
    return Number.isSafeInteger(v) ? String(v) : fmtNum(v);
  }
  if (Array.isArray(v)) {
    return `[${v.map(compact).join(",")}]`;
  }
  if (isObject(v)) {
    return `{${entries(v)
      .map(([k, x]) => `${jsonString(k)}:${compact(x)}`)
      .join(",")}}`;
  }
  return toJsonString(v);
}

/// Indented JSON, floats rounded by `fmtNum`; arrays and objects of scalars stay on one line.
export function pretty(v: Json): string {
  const out = { s: "" };
  writePretty(v, 0, out);
  out.s += "\n";
  return out.s;
}

function isScalar(v: Json): boolean {
  return !(Array.isArray(v) || isObject(v));
}

function values(o: JsonObject): Json[] {
  return o instanceof Map ? [...o.values()] : Object.values(o);
}

function size(o: JsonObject): number {
  return o instanceof Map ? o.size : Object.keys(o).length;
}

function flat(v: Json): boolean {
  if (Array.isArray(v)) {
    return v.every(isScalar) || (v.length <= 4 && v.every(flatPoint));
  }
  if (isObject(v)) {
    return values(v).every((x) => isScalar(x) || flatPoint(x));
  }
  return true;
}

/// `{"x":1,"y":2}` points and `[x,y]` pairs read best inline.
function flatPoint(v: Json): boolean {
  if (isObject(v)) {
    return size(v) <= 4 && values(v).every(isScalar);
  }
  if (Array.isArray(v)) {
    return v.length <= 4 && v.every(isScalar);
  }
  return false;
}

function writePretty(v: Json, depth: number, out: { s: string }): void {
  const pad = "  ".repeat(depth + 1);
  const end = "  ".repeat(depth);
  if (Array.isArray(v) && v.length > 0 && !flat(v)) {
    out.s += "[\n";
    v.forEach((x, i) => {
      out.s += pad;
      writePretty(x, depth + 1, out);
      out.s += i + 1 < v.length ? ",\n" : "\n";
    });
    out.s += end;
    out.s += "]";
  } else if (isObject(v) && size(v) > 0 && !flat(v)) {
    out.s += "{\n";
    const es = entries(v);
    es.forEach(([k, x], i) => {
      out.s += pad;
      out.s += jsonString(k);
      out.s += ": ";
      writePretty(x, depth + 1, out);
      out.s += i + 1 < es.length ? ",\n" : "\n";
    });
    out.s += end;
    out.s += "}";
  } else {
    out.s += compact(v);
  }
}

function asStr(v: Json): string | null {
  return typeof v === "string" ? v : null;
}

/// One block per diagnostic: `severity[code] L:C-L:C: message`, then help, suppression,
/// elements, facts and suggestions. `this is kashika <version>` is redacted.
export function diagnostics(diags: readonly Json[]): string {
  let out = "";
  for (const d of diags) {
    const s = get(d, "span");
    out +=
      `${asStr(get(d, "severity")) ?? "?"}[${asStr(get(d, "code")) ?? "?"}] ` +
      `${toJsonString(get(s, "line"))}:${toJsonString(get(s, "col"))}-` +
      `${toJsonString(get(s, "end_line"))}:${toJsonString(get(s, "end_col"))}: ` +
      `${asStr(get(d, "message")) ?? ""}\n`;
    const help = get(d, "help");
    if (Array.isArray(help)) {
      for (const h of help) {
        out += `  = help: ${asStr(h) ?? ""}\n`;
      }
    }
    if (get(d, "suppressed") === true) {
      const r = asStr(get(d, "reason"));
      out += r !== null ? `  = suppressed: ${r}\n` : "  = suppressed\n";
    }
    for (const key of ["elements", "facts", "suggestions"]) {
      const v = get(d, key);
      const empty =
        v === null || (Array.isArray(v) && v.length === 0) || (isObject(v) && size(v) === 0);
      if (!empty) {
        out += `  ${key}: ${compact(v)}\n`;
      }
    }
  }
  if (out === "") {
    out += "(no diagnostics)\n";
  }
  return out.split(`this is kashika ${VERSION}`).join("this is kashika <tool>");
}
