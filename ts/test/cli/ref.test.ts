import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";

import { check } from "../../src/check/index.js";
import { ERROR_CODES, LINT_CODES } from "../../src/diag/codes.js";
import { BUNDLED_ICONS, Marker, STYLES, compile, shapeNames } from "../../src/model/index.js";
import { format } from "../../src/syntax/index.js";
import { CLI_CRATE, ROOT, kashika, kashikaAsync } from "../support/cli.js";

const TOPICS = [
  "rules",
  "syntax",
  "anchors",
  "paths",
  "sizing",
  "text",
  "shapes",
  "styles",
  "recipes",
  "examples",
  "lints",
  "metrics",
  "cli",
  "limits",
];

function page(args: string[]): string {
  const r = kashika(["ref", ...args]);
  expect(r.code, args.join(" ")).toBe(0);
  return r.stdout;
}

async function pages(topics: string[]): Promise<string[]> {
  const runs = await Promise.all(topics.map((t) => kashikaAsync(["ref", t])));
  runs.forEach((r, i) => expect(r.code, topics[i]).toBe(0));
  return runs.map((r) => r.stdout);
}

const skill = () => readFileSync(join(ROOT, "skills/kashika/SKILL.md"), "utf8");

test("ref pages are copies of the Rust crate's", () => {
  const ours = join(import.meta.dirname, "../../src/cli/ref");
  const theirs = join(CLI_CRATE, "ref");
  expect(readdirSync(ours).sort()).toEqual(readdirSync(theirs).sort());
  for (const f of readdirSync(theirs)) {
    expect(readFileSync(join(ours, f)).equals(readFileSync(join(theirs, f))), f).toBe(true);
  }
});

test("references_and_docs_are_aliases_of_ref", () => {
  const p = page(["cli"]);
  for (const alias of ["references", "docs"]) {
    const r = kashika([alias, "cli"]);
    expect(r.code, alias).toBe(0);
    expect(r.stdout, alias).toBe(p);
  }
});

test("default_page_has_the_required_parts_and_length", () => {
  const p = page([]);
  for (const heading of [
    "## The loop",
    "## Rules people get wrong",
    "## Syntax",
    "## Anchors and points",
    "## Line paths",
    "## Fixing diagnostics",
    "## Topics",
  ]) {
    expect(p, `missing ${heading}`).toContain(heading);
  }
  for (const t of TOPICS) {
    expect(p, `topic list misses ${t}`).toContain(`| \`${t}\` |`);
  }
  const words = p
    .split(/\s+/)
    .filter((w) => w !== "" && [...w].some((c) => /[\p{L}\p{N}]/u.test(c))).length;
  expect(words >= 1500 && words <= 2500, `default page has ${words} words`).toBe(true);
});

test("recipes_page_has_every_recipe", () => {
  const p = page(["recipes"]);
  for (const recipe of [
    "Staircase",
    "Wrap over the top",
    "Gap corridor",
    "Outside margin",
    "The comb",
    "Unavoidable crossings",
    "Fan-out via a junction",
    "Two lines on one side",
    "Self-loop",
    "Sequence diagrams",
    "Table cards",
    "Guides for rows and columns",
    "Graphs and timelines",
  ]) {
    expect(p, `missing recipe ${recipe}`).toContain(recipe);
  }
});

test("every_topic_has_a_page", async () => {
  const all = await pages(TOPICS);
  all.forEach((p, i) => expect(p.trim(), TOPICS[i]).not.toBe(""));
  expect(all[TOPICS.indexOf("examples")]).toContain(
    readFileSync(join(ROOT, "examples/upload/upload.kash"), "utf8"),
  );
});

test("bundled_upload_example_matches_examples", () => {
  expect(
    readFileSync(join(CLI_CRATE, "ref/upload.kash"), "utf8"),
    "copy examples/upload/upload.kash to crates/kashika-cli/ref/upload.kash",
  ).toBe(readFileSync(join(ROOT, "examples/upload/upload.kash"), "utf8"));
});

test("every_diagnostic_code_has_a_page", async () => {
  for (const [codes, severity] of [
    [ERROR_CODES, "error"],
    [LINT_CODES, "warn"],
  ] as const) {
    const all = await pages([...codes]);
    codes.forEach((code, i) => {
      const p = all[i]!;
      expect(p.startsWith(`# ${code} (${severity})`), `${code}: ${p}`).toBe(true);
      expect(p, code).toContain("Fix: ");
    });
  }
});

test("json_wraps_the_page", () => {
  let r = kashika(["ref", "anchors", "--json"]);
  expect(r.code).toBe(0);
  let v = JSON.parse(r.stdout);
  expect(v.schema).toBe("kashika.ref/1");
  expect(v.topic).toBe("anchors");
  expect(v.topics.length).toBe(TOPICS.length);
  expect(v.markdown).toContain("## Anchors and points");
  r = kashika(["ref", "--json"]);
  v = JSON.parse(r.stdout);
  expect(v.topic).toBeNull();
});

test("unknown_topic_is_a_usage_error_with_a_suggestion", () => {
  let r = kashika(["ref", "anchor"]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain("did you mean `anchors`?");
  r = kashika(["ref", "nope"]);
  expect(r.code).toBe(3);
  for (const t of TOPICS) {
    expect(r.stderr).toContain(`\n  ${t} `);
  }
});

test("metrics_are_generated_from_the_bundled_fonts", () => {
  const p = page(["metrics"]);
  expect(p).toContain("| 14 |");
  expect(p).toContain("kashika measure");
  const row = p.split("\n").find((l) => l.startsWith("| 14 |"))!;
  const cells = row
    .replace(/^\|+|\|+$/g, "")
    .split("|")
    .map((c) => Number(c.trim()));
  expect(cells[1]! > 5.0 && cells[1]! < 10.0, row).toBe(true);
  expect(cells[2]! >= cells[1]!, `bold is not narrower: ${row}`).toBe(true);
  expect(cells[3]).toBe(17.5);
});

function kashBlocks(md: string): string[] {
  const out: string[] = [];
  let cur: string | null = null;
  for (const l of md.replace(/\n$/, "").split("\n")) {
    if (cur === null) {
      if (l === "```kash") cur = "";
    } else if (l === "```") {
      out.push(cur);
      cur = null;
    } else {
      cur += `${l}\n`;
    }
  }
  return out;
}

function assertBlocksCleanAndFormatted(blocks: string[]): void {
  for (const b of blocks) {
    const [scene, diags] = compile(b);
    diags.push(...check(scene).filter((d) => !d.suppressed));
    expect(diags.map((d) => `${d.code}: ${d.message}`), b).toEqual([]);
    const f = format(b);
    expect(f.ok && f.value, `not in fmt form:\n${b}`).toBe(b);
  }
}

test("ref_examples_check_clean_and_are_formatted", async () => {
  const blocks = kashBlocks(page([]));
  for (const p of await pages(TOPICS)) blocks.push(...kashBlocks(p));
  expect(blocks.length, `found ${blocks.length} blocks`).toBeGreaterThanOrEqual(9);
  assertBlocksCleanAndFormatted(blocks);
});

test("skill_examples_check_clean_and_are_formatted", () => {
  const blocks = kashBlocks(skill());
  expect(blocks.length, "SKILL.md should have an example").toBeGreaterThan(0);
  assertBlocksCleanAndFormatted(blocks);
});

test("shapes_page_lists_every_builtin_shape", () => {
  const p = page(["shapes"]);
  for (const name of shapeNames()) {
    expect(p, `missing ${name}`).toContain(`\n| \`${name}\``);
  }
});

test("skill_points_at_real_ref_topics", async () => {
  const s = skill();
  const topics: string[] = [];
  let i = s.indexOf("kashika ref ");
  while (i >= 0) {
    const m = /^[A-Za-z0-9-]*/.exec(s.slice(i + "kashika ref ".length))![0];
    if (m !== "") topics.push(m);
    i = s.indexOf("kashika ref ", i + 1);
  }
  expect(topics.length, "SKILL.md should point at `kashika ref <topic>` pages").toBeGreaterThan(0);
  await pages(topics);
});

/// Every token inside the inline code spans and code blocks of `md`.
function codeTokens(md: string): Set<string> {
  const ticks = (s: string) => s.length - s.replace(/^`+/, "").length;
  const code: string[] = [];
  let rest = md;
  for (;;) {
    const i = rest.indexOf("`");
    if (i < 0) break;
    const n = ticks(rest.slice(i));
    const body = rest.slice(i + n);
    let close: number | null = null;
    let from = 0;
    for (;;) {
      const j = body.slice(from).indexOf("`");
      if (j < 0) break;
      const k = ticks(body.slice(from + j));
      if (k === n) {
        close = from + j;
        break;
      }
      from += j + k;
    }
    if (close !== null) {
      code.push(body.slice(0, close));
      rest = body.slice(close + n);
    } else {
      rest = body;
    }
  }
  return new Set(code.flatMap((c) => c.split(/[^A-Za-z0-9_-]/)).filter((t) => t !== ""));
}

/// The spec and the `ref` pages name every built-in asset, so an agent that reads the topics
/// finds each one. The skill is a short entry point that sends agents to `kashika ref`, so it
/// lists none of them.
test("spec_and_ref_name_every_builtin_asset", async () => {
  const all: [string, string][] = [
    ...shapeNames().map((n): [string, string] => ["shape", n]),
    ...BUNDLED_ICONS.map((n): [string, string] => ["icon", n]),
    ...STYLES.map((s): [string, string] => ["style", s.name]),
    ...Marker.NAMES.map((n): [string, string] => ["line end", n]),
  ];
  const spec = readFileSync(join(ROOT, "docs/spec/language.md"), "utf8");
  const refPages = (await pages(TOPICS)).join("");
  for (const [doc, text] of [
    ["docs/spec/language.md", spec],
    ["`kashika ref`", refPages],
  ] as const) {
    const tokens = codeTokens(text);
    for (const [kind, name] of all) {
      expect(tokens.has(name), `${doc} never names the built-in ${kind} \`${name}\` in code`).toBe(
        true,
      );
    }
  }
});
