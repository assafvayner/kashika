import { describe, expect, it } from "vitest";

import { systemPrompt } from "../src/agent/prompt.js";
import { TOOL_DEFS, runTool } from "../src/agent/tools.js";
import { Document } from "../src/document/store.svelte.js";
import { engine } from "./wasm.js";

const SRC = `diagram "t" size (300,200)
box a "A" at (20,20) size (80,40)
box b "B" at (200,20) size (80,40)
`;

describe("systemPrompt", () => {
  it("carries the skill, the core guide, and no CLI commands", () => {
    const p = systemPrompt(engine);
    expect(p).toContain("no auto-layout");
    expect(p).toContain("## Topics");
    expect(p).not.toMatch(/`kashika (check|render|route|inspect|ref|measure|icons)/);
    expect(p).toContain("write_source");
    expect(p).not.toMatch(/file\.kash|`[^`]*--[a-z]|\bCLI\b|^---$|render and look/im);
    expect(p).not.toMatch(/\| `cli` \|/);
    expect(p).toMatch(/\| `limits` \|/);
  });
});

describe("runTool", () => {
  const doc = () => new Document(engine, SRC);
  it("write_source replaces the document and reports", () => {
    const d = doc();
    const out = runTool(engine, d, "write_source", { source: SRC + "line ab a.right -> b.left\n" });
    expect(d.source).toContain("line ab");
    expect(out).toContain("0 errors, 0 warnings");
  });
  it("edit_source applies exact edits and appends, or applies nothing", () => {
    const d = doc();
    const out = runTool(engine, d, "edit_source", {
      edits: [{ find: 'box b "B" at (200,20)', replace: 'box b "Bee" at (200,20)' }],
      append: "line ab a.right -> b.left",
    });
    expect(d.source).toContain('"Bee"');
    expect(d.source).toMatch(/line ab a\.right -> b\.left\n$/);
    expect(out).toContain("0 errors, 0 warnings");
    expect(out).not.toContain("Fix all of these");
    const before = d.source;
    expect(runTool(engine, d, "edit_source", { edits: [{ find: "zzz", replace: "" }] })).toMatch(
      /^error: edit 1: `find` is not in the source/,
    );
    expect(
      runTool(engine, d, "edit_source", {
        edits: [
          { find: "box a", replace: "box a" },
          { find: "size (80,40)", replace: "" },
        ],
      }),
    ).toMatch(/^error: edit 2: `find` occurs more than once/);
    expect(runTool(engine, d, "edit_source", {})).toMatch(/^error: edit_source needs/);
    expect(d.source).toBe(before);
    expect(
      runTool(engine, d, "edit_source", { edits: [{ find: "line ab", replace: "line ab zz" }] }),
    ).toMatch(/Fix all of these in one edit_source call/);
  });
  it("read_source, check, inspect, route, measure, ref, icons", () => {
    const d = doc();
    expect(runTool(engine, d, "read_source", {})).toBe(SRC);
    expect(runTool(engine, d, "check", {})).toContain("0 errors");
    expect(runTool(engine, d, "inspect", { ids: ["a"] })).toContain("a");
    expect(runTool(engine, d, "route", { from: "a.right", to: "b.left" })).toMatch(/^#1/);
    expect(runTool(engine, d, "measure", { text: "Hello" })).toMatch(/width=/);
    expect(runTool(engine, d, "ref", { topic: "through" })).toMatch(/^# through/);
    expect(runTool(engine, d, "icons", { search: "database" })).toContain("cylinder");
  });
  it("route batches lines, pairs and flagged lines", () => {
    const d = new Document(
      engine,
      `diagram "r" size (400,300) ortho
box a "A" at (20,20) size (80,40)
box b "B" at (300,20) size (80,40)
box c "C" at (300,200) size (80,40)
line ac a.bottom -> c.left
line bc b.bottom -> c.top
`,
    );
    const auto = runTool(engine, d, "route", {});
    expect(auto).toMatch(/^## ac\n#1 {2}crossings=/m);
    expect(auto).toMatch(/^ {4}line ac /m);
    expect(auto).not.toContain("## bc");
    expect(auto).not.toContain("#2");
    const picked = runTool(engine, d, "route", { lines: ["ac", "bc"], max: 2 });
    expect(picked).toMatch(/^## ac\n#1 /m);
    expect(picked).toMatch(/^## bc\n#1 /m);
    const pairs = runTool(engine, d, "route", { pairs: [{ from: "a.right", to: "b.left" }] });
    expect(pairs).toMatch(/^## a\.right -> b\.left\n#1 /m);
    expect(runTool(engine, d, "route", { lines: ["zz"] })).toMatch(/^## zz\nerror: /m);
    const broken = new Document(engine, d.source + "line zz a.rigth -> c.top\n");
    const blocked = runTool(engine, broken, "route", {});
    expect(blocked).toMatch(/^fix these errors first, then route:\n/);
    expect(blocked.match(/unknown-anchor/g)).toHaveLength(1);
    expect(runTool(engine, doc(), "route", {})).toMatch(/^no line has a finding/);
    d.set(d.source + "line a.top -> c.top\n", "agent");
    expect(runTool(engine, d, "route", {})).toContain(
      "## line at source line 7\nerror: give this line an id",
    );
  });
  it("reports engine errors and unknown tools as text", () => {
    const d = doc();
    expect(runTool(engine, d, "inspect", { ids: ["zz"] })).toMatch(/^error: unknown id/);
    expect(runTool(engine, d, "nope", {})).toMatch(/^error: unknown tool/);
    expect(runTool(engine, d, "write_source", {})).toMatch(/^error: /);
  });
  it("coerces loose arguments and reports bad ones", () => {
    const d = doc();
    expect(runTool(engine, d, "inspect", { ids: "a" })).toContain("a");
    expect(runTool(engine, d, "measure", { text: "Hello", size: "20" })).toMatch(/width=/);
    expect(runTool(engine, d, "route", { from: "a.right", to: "b.left", max: "99" })).toMatch(
      /^#1/,
    );
    expect(runTool(engine, d, "route", { from: "a.right", to: "b.left", style: "wavy" })).toMatch(
      /^error: unknown style `wavy`/,
    );
    expect(runTool(engine, d, "icons", { search: "zzzqqq" })).toBe("no icons match `zzzqqq`");
    expect(runTool(engine, d, "ref", {})).toBe("error: ref needs a topic");
  });
  it("every tool has a definition", () => {
    expect(TOOL_DEFS.map((t) => t.name).sort()).toEqual([
      "check",
      "edit_source",
      "icons",
      "inspect",
      "measure",
      "read_source",
      "ref",
      "route",
      "write_source",
    ]);
  });
});
