import { Float, type Json, toJsonString } from "../diag/json.js";
import { type Result, displayF64 } from "../diag/rust.js";
import { round2 } from "../geom/index.js";
import { TOOL_VERSION } from "../model/version.js";
import { type Block, type MarkupError, measure } from "../text/index.js";
import { type Format, print, toolError } from "./main.js";
import * as term from "./term.js";

export const SCHEMA = "kashika.measure/1";

export type Weight = "normal" | "bold";

export interface MeasureArgs {
  text: string[];
  size: number;
  weight: Weight;
  format: Format;
}

/// Applies the string escapes the lexer applies: `\n`, `\"` and `\\`. Markup escapes
/// such as `\*` stay for the markup parser.
function unescape(s: string): string {
  let out = "";
  const chars = Array.from(s);
  let i = 0;
  while (i < chars.length) {
    const c = chars[i++]!;
    if (c !== "\\") {
      out += c;
      continue;
    }
    const next = chars[i++];
    switch (next) {
      case "n":
        out += "\n";
        break;
      case '"':
        out += '"';
        break;
      case "\\":
        out += "\\";
        break;
      case undefined:
        out += "\\";
        break;
      default:
        out += "\\";
        out += next;
    }
  }
  return out;
}

export function run(a: MeasureArgs): number {
  if (!(Number.isFinite(a.size) && a.size > 0.0)) {
    return toolError(a.format, SCHEMA, "--size must be a positive number");
  }
  const bold = a.weight === "bold";
  const weight = bold ? "bold" : "normal";
  const results = a.text.map((t) => measure(unescape(t), a.size, bold));
  const failed = results.some((r) => !r.ok);
  const entry = (r: Result<Block, MarkupError>): Map<string, Json> =>
    r.ok
      ? new Map<string, Json>([
          ["width", new Float(round2(r.value.width()))],
          ["height", new Float(round2(r.value.height()))],
          ["lines", r.value.lines().length],
        ])
      : new Map<string, Json>([["error", { code: "parse", message: r.error.toString() }]]);
  if (a.format === "text") {
    for (const r of results) {
      if (r.ok) {
        print(
          `width=${displayF64(round2(r.value.width()))} height=${displayF64(round2(r.value.height()))} lines=${r.value.lines().length}\n`,
        );
      } else {
        print(`error[parse]: ${term.line(r.error.toString())}\n`);
      }
    }
  } else {
    const fields = new Map<string, Json>([
      ["schema", SCHEMA],
      ["tool", TOOL_VERSION],
      ["size", new Float(a.size)],
      ["weight", weight],
    ]);
    if (results.length === 1) {
      for (const [k, v] of entry(results[0]!)) fields.set(k, v);
    } else {
      fields.set(
        "results",
        a.text.map((t, i) => {
          const e = entry(results[i]!);
          e.set("text", t);
          return e;
        }),
      );
    }
    print(`${toJsonString(fields)}\n`);
  }
  return failed ? 2 : 0;
}
