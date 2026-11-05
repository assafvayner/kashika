import type { Document } from "../document/store.svelte.js";
import { type Engine, EngineError, type RouteOptions } from "../engine/types.js";
import type { ToolDef } from "../providers/types.js";

const NO_ARGS = { type: "object", properties: {}, additionalProperties: false };

export const TOOL_DEFS: ToolDef[] = [
  { name: "read_source", description: "The current .kash source.", parameters: NO_ARGS },
  {
    name: "write_source",
    description:
      "Replace the whole .kash source and get the check report back. The only way to change the diagram.",
    parameters: {
      type: "object",
      properties: {
        source: {
          type: "string",
          description: "The complete new .kash file, not a diff or fragment.",
        },
      },
      required: ["source"],
      additionalProperties: false,
    },
  },
  {
    name: "edit_source",
    description:
      "Edit the current .kash source in place and get the check report back. Each edit replaces one exact occurrence of `find` with `replace` (empty `replace` deletes it); `append` adds lines at the end. Nothing is applied if an edit does not match exactly once. Much cheaper than write_source for fixes and additions.",
    parameters: {
      type: "object",
      properties: {
        edits: {
          type: "array",
          items: {
            type: "object",
            properties: {
              find: { type: "string", description: "Exact text that occurs once in the source." },
              replace: { type: "string" },
            },
            required: ["find", "replace"],
            additionalProperties: false,
          },
        },
        append: { type: "string", description: "Lines to add at the end of the source." },
      },
      additionalProperties: false,
    },
  },
  { name: "check", description: "The check report for the current source.", parameters: NO_ARGS },
  {
    name: "inspect",
    description:
      "Resolved geometry: canvas size, boxes with bbox and anchors, lines with vertices. Pass ids to limit it; ports lists line endpoints per side.",
    parameters: {
      type: "object",
      properties: {
        ids: {
          type: "array",
          items: { type: "string" },
          description: "Element ids to report; omit for all.",
        },
        ports: { type: "boolean" },
      },
      additionalProperties: false,
    },
  },
  {
    name: "route",
    description:
      "Ready-to-paste line statements, ranked by crossings. With no arguments it reroutes every line the check flags (not-ortho, cross, through, near, shared-segment, group-transit) and returns one statement per line to paste over the old one. Pass lines (ids) to reroute chosen lines, pairs for new connections, or from and to for one. Points are written as in a line, e.g. a.right, b.top@0.25.",
    parameters: {
      type: "object",
      properties: {
        from: { type: "string", description: "Points like a.right or b.top@0.25." },
        to: { type: "string", description: "Points like a.right or b.top@0.25." },
        style: { type: "string", enum: ["corner", "arc", "bezier"] },
        line: {
          type: "string",
          description: "Id of an existing line to reroute instead of from/to.",
        },
        lines: {
          type: "array",
          items: { type: "string" },
          description: "Ids of existing lines to reroute, one result each.",
        },
        pairs: {
          type: "array",
          items: {
            type: "object",
            properties: { from: { type: "string" }, to: { type: "string" } },
            required: ["from", "to"],
            additionalProperties: false,
          },
          description: "New connections to route, one result each.",
        },
        max: {
          type: "integer",
          minimum: 1,
          maximum: 10,
          description: "Candidates per route; default 3 for one route, 1 for several.",
        },
      },
      additionalProperties: false,
    },
  },
  {
    name: "measure",
    description: "Exact width and height of a label with markup, at a font size (default 14).",
    parameters: {
      type: "object",
      properties: {
        text: { type: "string" },
        size: { type: "number", exclusiveMinimum: 0 },
        bold: { type: "boolean" },
      },
      required: ["text"],
      additionalProperties: false,
    },
  },
  {
    name: "ref",
    description:
      "A reference topic (rules, syntax, anchors, paths, sizing, text, shapes, styles, recipes, examples, lints, limits) or the page for a diagnostic code.",
    parameters: {
      type: "object",
      properties: { topic: { type: "string" } },
      required: ["topic"],
      additionalProperties: false,
    },
  },
  {
    name: "icons",
    description: "Search the bundled icons and built-in shapes by name or keyword.",
    parameters: {
      type: "object",
      properties: { search: { type: "string" } },
      additionalProperties: false,
    },
  },
];

type Args = Record<string, unknown>;

const str = (a: Args, k: string): string | undefined =>
  typeof a[k] === "string" ? (a[k] as string) : undefined;

const num = (a: Args, k: string): number | undefined => {
  const v = a[k];
  const n = typeof v === "string" && v.trim() !== "" ? Number(v) : v;
  return typeof n === "number" && Number.isFinite(n) ? n : undefined;
};

const STYLES = ["corner", "arc", "bezier"] as const;

/** The check report, with a nudge to fix every finding in one round. */
function report(engine: Engine, source: string): string {
  const text = engine.check(source).text;
  return /\b0 errors, 0 warnings\b/.test(text)
    ? text
    : `${text}\nFix all of these in one edit_source call, then read the new report.`;
}

/** Applies the edits in order; returns the new source or an `error:` line, applying nothing. */
function applyEdits(source: string, args: Args): string {
  const raw = args["edits"];
  const edits = Array.isArray(raw) ? (raw as unknown[]) : [];
  const append = str(args, "append");
  if (edits.length === 0 && append === undefined)
    return "error: edit_source needs `edits` (a list of {find, replace}) or `append`";
  let out = source;
  for (const [i, e] of edits.entries()) {
    const o = typeof e === "object" && e !== null ? (e as Args) : {};
    const find = str(o, "find");
    const replace = str(o, "replace") ?? "";
    if (find === undefined || find === "") return `error: edit ${i + 1} needs a non-empty \`find\``;
    const first = out.indexOf(find);
    if (first === -1) return `error: edit ${i + 1}: \`find\` is not in the source; nothing applied`;
    if (out.indexOf(find, first + find.length) !== -1)
      return `error: edit ${i + 1}: \`find\` occurs more than once; include more context; nothing applied`;
    out = out.slice(0, first) + replace + out.slice(first + find.length);
  }
  if (append !== undefined && append !== "") {
    out = `${out.replace(/\n*$/, "")}\n${append.replace(/\n*$/, "")}\n`;
  }
  return out;
}

/** Runs one tool and returns its text result; errors are returned as `error: ...` text for the model. */
/** Diagnostic codes a rerouted path fixes. */
const ROUTABLE = ["not-ortho", "cross", "through", "near", "shared-segment", "group-transit"];

/**
 * Routes several lines or pairs; with neither, every line the check flags. Each suggestion is
 * applied to a working copy before the next is computed, so the results account for each other.
 */
function routeMany(engine: Engine, source: string, args: Args, opts: RouteOptions): string {
  const lines = Array.isArray(args["lines"]) ? (args["lines"] as unknown[]).map(String) : [];
  const pairs = Array.isArray(args["pairs"])
    ? (args["pairs"] as unknown[]).flatMap((p) => {
        const o = p as Record<string, unknown>;
        return typeof o?.["from"] === "string" && typeof o?.["to"] === "string"
          ? [{ from: o["from"], to: o["to"] }]
          : [];
      })
    : [];
  const check = engine.check(source);
  if (check.diagnostics.some((d) => d.severity === "error" && d.code !== "not-ortho")) {
    return `fix these errors first, then route:\n${check.text.trimEnd()}\n`;
  }
  const rows = source.split("\n");
  const stmt = (id: string) =>
    new RegExp(`^\\s*(?:line|arrow)\\s+${id.replace(/[.*+?^${}()|[\]\\-]/g, "\\$&")}\\s`);
  const unnamed: number[] = [];
  if (lines.length === 0 && pairs.length === 0) {
    const seen = new Set<string>();
    for (const d of check.diagnostics) {
      if (d.suppressed || !ROUTABLE.includes(d.code)) continue;
      const row = rows[d.span.line - 1] ?? "";
      const id = /^\s*(?:line|arrow)\s+([A-Za-z_][\w-]*)\s/.exec(row)?.[1];
      if (id !== undefined && id !== "allow" && id !== "label") {
        if (!seen.has(id)) {
          seen.add(id);
          lines.push(id);
        }
      } else if (/^\s*(?:line|arrow)\s/.test(row)) {
        if (!unnamed.includes(d.span.line)) unnamed.push(d.span.line);
      }
    }
    if (lines.length === 0 && unnamed.length === 0) {
      return "no line has a finding that route fixes; pass lines, pairs, or from and to";
    }
  }
  let work = rows;
  const out: string[] = [];
  const one = (head: string, f: () => { text: string; statement: string | null }) => {
    try {
      const r = f();
      out.push(`## ${head}\n${r.text.trimEnd()}\n`);
      return r.statement;
    } catch (e) {
      if (e instanceof EngineError) {
        out.push(`## ${head}\nerror: ${e.message.trimEnd()}\n`);
        return null;
      }
      throw e;
    }
  };
  for (const id of lines) {
    const st = one(id, () => engine.routeFull(work.join("\n"), null, null, { ...opts, line: id }));
    const at = work.findIndex((row) => stmt(id).test(row));
    if (st !== null && at >= 0) {
      const indent = /^\s*/.exec(work[at] ?? "")?.[0] ?? "";
      work = [...work.slice(0, at), indent + st.trim(), ...work.slice(at + 1)];
    }
  }
  for (const p of pairs) {
    const st = one(`${p.from} -> ${p.to}`, () =>
      engine.routeFull(work.join("\n"), p.from, p.to, opts),
    );
    if (st !== null) work = [...work, st.trim()];
  }
  for (const n of unnamed) {
    out.push(`## line at source line ${n}\nerror: give this line an id to reroute it\n`);
  }
  return out.join("\n");
}

export function runTool(engine: Engine, doc: Document, name: string, args: Args): string {
  try {
    switch (name) {
      case "read_source":
        return doc.source;
      case "write_source": {
        const source = str(args, "source");
        if (source === undefined) return "error: write_source needs a `source` string";
        doc.set(source, "agent");
        return report(engine, source);
      }
      case "edit_source": {
        const source = applyEdits(doc.source, args);
        if (source.startsWith("error: ")) return source;
        doc.set(source, "agent");
        return report(engine, source);
      }
      case "check":
        return engine.check(doc.source).text;
      case "inspect": {
        const raw = args["ids"];
        const ids = Array.isArray(raw)
          ? (raw as unknown[]).map(String)
          : typeof raw === "string"
            ? [raw]
            : [];
        return engine.inspect(doc.source, ids, args["ports"] === true);
      }
      case "route": {
        const style = str(args, "style") ?? "corner";
        if (!(STYLES as readonly string[]).includes(style)) {
          return `error: unknown style \`${style}\`; use corner, arc or bezier`;
        }
        const raw = num(args, "max");
        const max = raw === undefined ? undefined : Math.min(10, Math.max(1, Math.trunc(raw)));
        const opts = { style: style as (typeof STYLES)[number] };
        if (str(args, "from") !== undefined || str(args, "to") !== undefined || str(args, "line")) {
          return engine.route(doc.source, str(args, "from") ?? null, str(args, "to") ?? null, {
            ...opts,
            line: str(args, "line"),
            max,
          });
        }
        return routeMany(engine, doc.source, args, { ...opts, max: max ?? 1 });
      }
      case "measure": {
        const text = str(args, "text");
        if (text === undefined) return "error: measure needs a `text` string";
        const m = engine.measure(text, num(args, "size") ?? 14, args["bold"] === true);
        return `width=${m.width} height=${m.height} lines=${m.lines}`;
      }
      case "ref": {
        const topic = str(args, "topic");
        if (!topic) return "error: ref needs a topic";
        return engine.ref(topic);
      }
      case "icons": {
        const search = str(args, "search");
        const found = engine.icons(search);
        if (found.length === 0 && search) return `no icons match \`${search}\``;
        return found
          .map(
            (i) =>
              `${i.name}  ${i.path}${i.matched_keyword ? `  (matches "${i.matched_keyword}")` : ""}`,
          )
          .join("\n");
      }
      default:
        return `error: unknown tool \`${name}\``;
    }
  } catch (e) {
    if (e instanceof EngineError) return `error: ${e.message}`;
    throw e;
  }
}
