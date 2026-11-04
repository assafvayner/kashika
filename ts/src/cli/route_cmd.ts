import { Axis } from "../check/free.js";
import { toJsonString, toValue } from "../diag/json.js";
import { fmtNum as num } from "../geom/index.js";
import { TOOL_VERSION } from "../model/version.js";
import { type Request, type Style, type Suggestion, suggest } from "../route/index.js";
import { type Format, options, print, readInput, toolError } from "./main.js";
import * as term from "./term.js";

export const SCHEMA = "kashika.route/1";

export const NO_ARROW =
  "`route` takes the two points as separate arguments, with no `->`: `kashika route <file> a.right b.left`";

export interface RouteArgs {
  file: string;
  /// `<from> <to>` points, then an optional style.
  rest: string[];
  line: string | null;
  radius: number;
  max: number;
  format: Format;
  allowPath: boolean;
  include: string[];
  stdinName: string | null;
}

export function run(a: RouteArgs): number {
  const input = readInput(a.file, a.stdinName);
  if (!input.ok) {
    return toolError(a.format, SCHEMA, input.error);
  }
  const [name, src] = input.value;
  const rest = [...a.rest];
  const last = rest.length > 0 ? rest[rest.length - 1] : undefined;
  const picked: Style | null =
    last === "corner" || last === "arc" || last === "bezier" ? last : null;
  if (picked !== null) {
    rest.pop();
  }
  const style: Style = picked ?? "corner";
  let from: string | null;
  let to: string | null;
  if (rest.length === 0) {
    [from, to] = [null, null];
  } else if (rest.length === 2) {
    [from, to] = [rest[0]!, rest[1]!];
  } else {
    let msg = "expected `<from> <to>` and an optional style (corner, arc, bezier)";
    if (rest.some((r) => r.includes("->"))) {
      msg += "; ";
      msg += NO_ARROW;
    }
    return toolError(a.format, SCHEMA, msg);
  }
  const opts = options(a.file, a.allowPath, a.include);
  if (!opts.ok) {
    return toolError(a.format, SCHEMA, opts.error);
  }
  const req: Request = {
    src,
    from,
    to,
    style,
    line: a.line,
    radius: a.radius,
    max: a.max,
    opts: opts.value,
  };
  const r = suggest(req);
  if (!r.ok) {
    const e = r.error;
    if (e.kind === "Usage") {
      return toolError(a.format, SCHEMA, e.message);
    }
    if (a.format === "json") {
      const v = {
        schema: SCHEMA,
        tool: TOOL_VERSION,
        input: { name },
        diagnostics: e.diags.map((d) => d.toJson()),
      };
      print(`${toJsonString(toValue(v))}\n`);
    } else {
      const shown = term.line(name);
      for (const d of e.diags) {
        print(`${shown}: error[${d.code}]: ${term.line(d.message)}\n`);
        for (const h of d.help) {
          print(`      = help: ${term.line(h)}\n`);
        }
      }
    }
    return 2;
  }
  const s = r.value;
  if (a.format === "json") {
    const v = {
      schema: SCHEMA,
      tool: TOOL_VERSION,
      input: { name },
      from: s.from.toJson(),
      to: s.to.toJson(),
      style: s.style,
      candidates: s.candidates.map((c) => c.toJson()),
      existing_lines: [...s.existingLines],
    };
    print(`${toJsonString(toValue(v))}\n`);
  } else {
    print(term.text(text(s)));
  }
  return s.candidates.length === 0 ? 1 : 0;
}

function text(s: Suggestion): string {
  let out = candidatesText(s);
  const id = s.existingLines[0];
  if (id !== undefined) {
    out += `hint: line \`${id}\` already connects these; pass --line ${id} to reroute it instead of avoiding it\n`;
  }
  return out;
}

function candidatesText(s: Suggestion): string {
  if (s.candidates.length === 0) {
    return "no candidate path avoids every box with at most 4 elbows (or 1 curve); try other anchors, or move a box\n";
  }
  let out = "";
  s.candidates.forEach((c, i) => {
    let crossings = `crossings=${c.crossings}`;
    if (c.crossed.length > 0) {
      const list = c.crossed.map((x) => `${x.line} at (${num(x.at.x)},${num(x.at.y)})`);
      crossings += ` (${list.join(", ")})`;
    }
    let transits = `transits=${c.transits}`;
    if (c.transited.length > 0) {
      const list = c.transited.map((t) => `${t.group} at (${num(t.at.x)},${num(t.at.y)})`);
      transits += ` (${list.join(", ")})`;
    }
    out += `#${i + 1}  ${crossings} ${transits} near=${c.near} segments=${c.segments} length=${num(c.length)}px`;
    if (c.corridor !== null) {
      const k = c.corridor;
      out += `  via corridor ${Axis.name(k.axis)} ${num(k.band[0])}..${num(k.band[1])}`;
    }
    out += `\n    ${c.statement}\n`;
    if (c.droppedAllow.length > 0) {
      out += `    note: dropped allow=${c.droppedAllow.join(",")} (no longer needed)\n`;
    }
  });
  return out;
}
