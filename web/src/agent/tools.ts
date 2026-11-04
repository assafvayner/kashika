import type { Document } from "../document/store.svelte.js";
import { type Engine, EngineError } from "../engine/types.js";
import type { ToolDef } from "../providers/types.js";

export const TOOL_DEFS: ToolDef[] = [
  {
    name: "read_source",
    description: "The current .kash source.",
    parameters: { type: "object", properties: {} },
  },
  {
    name: "write_source",
    description:
      "Replace the whole .kash source and get the check report back. The only way to change the diagram.",
    parameters: {
      type: "object",
      properties: { source: { type: "string" } },
      required: ["source"],
    },
  },
  {
    name: "check",
    description: "The check report for the current source.",
    parameters: { type: "object", properties: {} },
  },
  {
    name: "inspect",
    description:
      "Resolved geometry: canvas size, boxes with bbox and anchors, lines with vertices. Pass ids to limit it; ports lists line endpoints per side.",
    parameters: {
      type: "object",
      properties: { ids: { type: "array", items: { type: "string" } }, ports: { type: "boolean" } },
    },
  },
  {
    name: "route",
    description:
      "Ready-to-paste line statements from one point to another, ranked by crossings. Points are written as in a line, e.g. a.right, b.top@0.25. Pass line to reroute an existing line.",
    parameters: {
      type: "object",
      properties: {
        from: { type: "string" },
        to: { type: "string" },
        style: { type: "string", enum: ["corner", "arc", "bezier"] },
        line: { type: "string" },
        max: { type: "integer" },
      },
    },
  },
  {
    name: "measure",
    description: "Exact width and height of a label with markup, at a font size (default 14).",
    parameters: {
      type: "object",
      properties: { text: { type: "string" }, size: { type: "number" }, bold: { type: "boolean" } },
      required: ["text"],
    },
  },
  {
    name: "ref",
    description:
      "A reference topic (rules, syntax, anchors, paths, sizing, text, shapes, styles, recipes, examples, lints, limits) or the page for a diagnostic code.",
    parameters: { type: "object", properties: { topic: { type: "string" } }, required: ["topic"] },
  },
  {
    name: "icons",
    description: "Search the bundled icons and built-in shapes by name or keyword.",
    parameters: { type: "object", properties: { search: { type: "string" } } },
  },
];

type Args = Record<string, unknown>;

const str = (a: Args, k: string): string | undefined =>
  typeof a[k] === "string" ? (a[k] as string) : undefined;

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
        const ids = Array.isArray(args["ids"]) ? (args["ids"] as unknown[]).map(String) : [];
        return engine.inspect(doc.source, ids, args["ports"] === true);
      }
      case "route": {
        const style = str(args, "style");
        return engine.route(doc.source, str(args, "from") ?? null, str(args, "to") ?? null, {
          style: style === "arc" || style === "bezier" ? style : "corner",
          line: str(args, "line"),
          max: typeof args["max"] === "number" ? args["max"] : undefined,
        });
      }
      case "measure": {
        const text = str(args, "text");
        if (text === undefined) return "error: measure needs a `text` string";
        const m = engine.measure(
          text,
          typeof args["size"] === "number" ? args["size"] : 14,
          args["bold"] === true,
        );
        return `width=${m.width} height=${m.height} lines=${m.lines}`;
      }
      case "ref":
        return engine.ref(str(args, "topic"));
      case "icons":
        return engine
          .icons(str(args, "search"))
          .map(
            (i) =>
              `${i.name}  ${i.path}${i.matched_keyword ? `  (matches "${i.matched_keyword}")` : ""}`,
          )
          .join("\n");
      default:
        return `error: unknown tool \`${name}\``;
    }
  } catch (e) {
    if (e instanceof EngineError) return `error: ${e.message}`;
    throw e;
  }
}
