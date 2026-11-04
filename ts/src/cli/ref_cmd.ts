import { readFileSync } from "node:fs";
import { join } from "node:path";

import { didYouMean } from "../diag/distance.js";
import { toJsonString } from "../diag/json.js";
import { type Result, displayF64, err, lines, ok, parseF64, round } from "../diag/rust.js";
import { BUNDLED_ICONS, Marker, SHAPES, STYLES, shapeNames } from "../model/index.js";
import { TOOL_VERSION } from "../model/version.js";
import { LINE_HEIGHT, measure } from "../text/index.js";
import { type Format, print, toolError } from "./main.js";

const page = (name: string): string => readFileSync(join(import.meta.dirname, "ref", name), "utf8");

const INTRO = page("intro.md");
const RULES = page("rules.md");
const FIXES = page("fixes.md");
const SIZING = page("sizing.md");
const TEXT = page("text.md");
const SYNTAX = page("syntax.md");
const ANCHORS = page("anchors.md");
const PATHS = page("paths.md");
const LINTS = page("lints.md");
const RECIPES = page("recipes.md");
const EXAMPLE = page("example.md");
const UPLOAD = page("upload.kash");
const LIMITS = page("limits.md");
const SHAPES_MD = page("shapes.md");
const STYLES_MD = page("styles.md");
const METRICS = page("metrics.md");
const CLI = page("cli.md");

export const SCHEMA = "kashika.ref/1";

/// Every topic, with what it covers, in the order the topic list shows them. The first four
/// are also on the default page.
export const TOPICS: readonly (readonly [string, string])[] = [
  ["rules", "the rules people get wrong"],
  ["syntax", "every statement and its clauses, groups, junctions and layers"],
  ["anchors", "anchors, `@t`, offsets and mixed points"],
  ["paths", "segments, line ends, labels and `ortho`"],
  ["sizing", "fitted boxes, and how big each shape must be for its text"],
  ["text", "markup, sublabels, alignment and text blocks"],
  ["shapes", "built-in and custom shapes, SVG shapes, images, icons and imports"],
  ["styles", "style keys, colors and the built-in styles"],
  [
    "recipes",
    "routing patterns, fan-out, self-loops, sequence diagrams, ER tables, guides and graphs",
  ],
  ["examples", "a complete worked diagram"],
  ["lints", "every diagnostic code with its trigger and fix, and how to suppress one"],
  ["metrics", "font metrics and average character widths"],
  ["cli", "every command and flag: check, render, inspect, route, measure, fmt, icons and ref"],
  ["limits", "raster and file limits, and what is not supported"],
];

function topicTable(): string {
  let t = "| topic | covers |\n|---|---|\n";
  for (const [name, about] of TOPICS) {
    t += `| \`${name}\` | ${about} |\n`;
  }
  return t;
}

function examples(): string {
  return `${EXAMPLE}\n\`\`\`kash\n${UPLOAD}\`\`\`\n`;
}

function shapes(): string {
  let t = "| shape | unit path (120x80 box) | outline | text area |\n|---|---|---|---|\n";
  for (const s of SHAPES) {
    if (s.source === null) continue;
    const [d, outline, a] = s.source(120.0, 80.0);
    const name = s.name;
    const n = (v: number) => displayF64(round(v * 1000.0) / 1000.0);
    const words = d
      .split(/\s+/)
      .filter((w) => w !== "")
      .map((w) => {
        const v = parseF64(w);
        return v === null ? w : n(v);
      });
    const hidden = s.hiddenOutline ? " (not drawn)" : "";
    t += `| \`${name}\` | \`${words.join(" ")}\` | ${outline}${hidden} | (${n(a.x)},${n(a.y)},${n(a.w)},${n(a.h)}) |\n`;
  }
  return SHAPES_MD.replaceAll("{builtins}", t.trimEnd());
}

/// `` `a`, `b`, `c` `` for a list of names.
function codeList(names: Iterable<string>): string {
  return [...names].map((n) => `\`${n}\``).join(", ");
}

function stylesTable(): string {
  let t = "| built-in | fill | stroke |\n|---|---|---|";
  for (const s of STYLES) {
    const fill = s.fill === "none" ? "none" : `\`${s.fill}\``;
    let stroke = `\`${s.stroke}\``;
    if (s.strokeStyle === "dashed") stroke += ", dashed";
    else if (s.strokeStyle === "dotted") stroke += ", dotted";
    if (s.color !== null) {
      stroke += `, text \`${s.color}\``;
    }
    t += `\n| \`${s.name}\` | ${fill} | ${stroke} |`;
  }
  return t;
}

/// Fills the `{shapes}`, `{icons}`, `{styles}` and `{markers}` placeholders from the
/// built-in registries, so the pages never drift from them.
function fill(md: string): string {
  return md
    .replaceAll("{shapes}", codeList(shapeNames()))
    .replaceAll("{icons}", codeList(BUNDLED_ICONS))
    .replaceAll("{styles}", stylesTable())
    .replaceAll("{markers}", codeList(Marker.NAMES));
}

function metrics(): string {
  const SAMPLE = "The quick brown fox jumps over the lazy dog 0123456789";
  const n = Array.from(SAMPLE).length;
  const width = (size: number, bold: boolean) => {
    const b = measure(SAMPLE, size, bold);
    return (b.ok ? b.value.width() : 0.0) / n;
  };
  let s = METRICS;
  s += "\n| font-size | regular | bold | line height |\n|---|---|---|---|\n";
  for (const size of [10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0]) {
    s += `| ${displayF64(size)} | ${width(size, false).toFixed(2)} | ${width(size, true).toFixed(2)} | ${(size * LINE_HEIGHT).toFixed(2)} |\n`;
  }
  return s;
}

export function refPage(topic: string | null): Result<string, string> {
  const md = rawPage(topic);
  return md.ok ? ok(fill(md.value)) : md;
}

function rawPage(topic: string | null): Result<string, string> {
  if (topic === null) {
    const topics = `## Topics\n\n\`kashika ref <topic>\` prints one of these; \`kashika ref <code>\` explains a diagnostic.\n\n${topicTable()}`;
    return ok([INTRO, RULES, SYNTAX, ANCHORS, PATHS, FIXES, topics].join("\n"));
  }
  switch (topic) {
    case "rules":
      return ok(RULES);
    case "syntax":
      return ok(SYNTAX);
    case "sizing":
      return ok(SIZING);
    case "text":
      return ok(TEXT);
    case "recipes":
      return ok(RECIPES);
    case "limits":
      return ok(LIMITS);
    case "anchors":
      return ok(ANCHORS);
    case "paths":
      return ok(PATHS);
    case "shapes":
      return ok(shapes());
    case "styles":
      return ok(STYLES_MD);
    case "metrics":
      return ok(metrics());
    case "lints":
      return ok(LINTS);
    case "examples":
      return ok(examples());
    case "cli":
      return ok(CLI);
    default: {
      const l = lint(topic);
      return l === null ? err(unknown(topic)) : ok(l);
    }
  }
}

function trimMatches(s: string, c: string): string {
  let a = 0;
  let b = s.length;
  while (a < b && s[a] === c) a++;
  while (b > a && s[b - 1] === c) b--;
  return s.slice(a, b);
}

/// The cells of each row of the diagnostics table: code, severity, trigger, fix.
function rows(): string[][] {
  return lines(LINTS)
    .filter((l) => l.startsWith("| `"))
    .map((l) =>
      trimMatches(l, "|")
        .split("|")
        .map((c) => c.trim()),
    );
}

function lint(code: string): string | null {
  const row = rows().find((r) => trimMatches(r[0]!, "`") === code);
  if (row === undefined) return null;
  return `# ${code} (${row[1]})\n\nTrigger: ${row[2]}.\n\nFix: ${row[3]}.\n`;
}

function unknown(topic: string): string {
  const codes = rows().map((r) => trimMatches(r[0]!, "`"));
  let msg = `unknown ref topic \`${topic}\``;
  const all = [...TOPICS.map(([t]) => t), ...codes];
  const s = didYouMean(topic, all);
  let sep: string;
  if (s !== null) {
    msg += `; did you mean \`${s}\`?`;
    sep = " Pass";
  } else {
    sep = "; pass";
  }
  msg += `${sep} a diagnostic code or one of these topics:`;
  for (const [name, about] of TOPICS) {
    msg += `\n  ${name.padEnd(9)} ${about}`;
  }
  return msg;
}

export function run(topic: string | null, json: boolean): number {
  const format: Format = json ? "json" : "text";
  const md = refPage(topic);
  if (!md.ok) {
    return toolError(format, SCHEMA, md.error);
  }
  if (json) {
    print(
      `${toJsonString({
        schema: SCHEMA,
        tool: TOOL_VERSION,
        topic,
        topics: TOPICS.map(([t]) => t),
        markdown: md.value,
      })}\n`,
    );
  } else {
    print(md.value);
  }
  return 0;
}
