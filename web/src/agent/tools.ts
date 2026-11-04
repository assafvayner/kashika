import type { Document } from "../document/store.svelte.js";
import { type Engine, EngineError } from "../engine/types.js";
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
      "Ready-to-paste line statements, ranked by crossings. Takes from and to, or line. Points are written as in a line, e.g. a.right, b.top@0.25. Pass line to reroute an existing line.",
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
        max: { type: "integer", minimum: 1, maximum: 10 },
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

/** Runs one tool and returns its text result; errors are returned as `error: ...` text for the model. */
export function runTool(engine: Engine, doc: Document, name: string, args: Args): string {
  try {
    switch (name) {
      case "read_source":
        return doc.source;
      case "write_source": {
        const source = str(args, "source");
        if (source === undefined) return "error: write_source needs a `source` string";
        doc.set(source, "agent");
        return engine.check(source).text;
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
        const max = num(args, "max");
        return engine.route(doc.source, str(args, "from") ?? null, str(args, "to") ?? null, {
          style: style as (typeof STYLES)[number],
          line: str(args, "line"),
          max: max === undefined ? undefined : Math.min(10, Math.max(1, Math.trunc(max))),
        });
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
