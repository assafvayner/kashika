import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, test } from "vitest";
import { sum } from "../../src/diag/rust.js";
import { round2 } from "../../src/geom/index.js";
import { runWidth } from "../../src/text/font.js";
import { type Font, Run } from "../../src/text/index.js";

const PARSE = join(import.meta.dirname, "../../../tests/golden/parse");

function scenes(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) scenes(p, out);
    else if (name === "scene.json") out.push(p);
  }
  return out;
}

interface Line {
  runs: { font: Font; text: string }[];
  width: number;
}

/// Every text block in a scene: an object with a `size` and `lines` of runs.
function blocks(
  v: unknown,
  out: { size: number; lines: Line[] }[] = [],
): { size: number; lines: Line[] }[] {
  if (Array.isArray(v)) {
    for (const x of v) blocks(x, out);
  } else if (v !== null && typeof v === "object") {
    const o = v as Record<string, unknown>;
    if (typeof o.size === "number" && Array.isArray(o.lines) && o.lines.every((l) => "runs" in l)) {
      out.push({ size: o.size, lines: o.lines as Line[] });
    }
    for (const x of Object.values(o)) blocks(x, out);
  }
  return out;
}

describe("golden widths", () => {
  const files = scenes(PARSE);

  test("every_parse_golden_has_text", () => {
    expect(files.length).toBeGreaterThan(50);
    expect(
      files.reduce((n, f) => n + blocks(JSON.parse(readFileSync(f, "utf8"))).length, 0),
    ).toBeGreaterThan(100);
  });

  for (const file of files) {
    test(relative(PARSE, file), () => {
      for (const b of blocks(JSON.parse(readFileSync(file, "utf8")))) {
        for (const l of b.lines) {
          const w = sum(l.runs.map((r) => runWidth(new Run(r.text, r.font), b.size)));
          expect(round2(w), JSON.stringify(l.runs)).toBe(l.width);
        }
      }
    });
  }
});
