import { existsSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

import { type Diagnostic, sort } from "../../src/diag/index.js";
import { diagnostics, pretty } from "../../src/geom/dump.js";
import { compileLibraryWith, compileWith } from "../../src/model/index.js";
import { GOLDEN, caseDirs, caseOptions } from "../support/golden.js";

/// `diagnostics.txt` without its `warn[...]` blocks, which the lints (a later task) produce.
function withoutWarnings(text: string): string {
  const out: string[] = [];
  let skipping = false;
  for (const l of text.split("\n")) {
    if (l.startsWith("warn[")) {
      skipping = true;
      continue;
    }
    if (skipping && l.startsWith("  ")) continue;
    skipping = false;
    out.push(l);
  }
  const s = out.join("\n");
  return s.trim() === "" ? "(no diagnostics)\n" : s;
}

const cases = caseDirs(join(GOLDEN, "parse"));

describe("golden parse cases: scene.json and error diagnostics", () => {
  it("finds the cases", () => {
    expect(cases.length).toBeGreaterThan(200);
  });
  for (const dir of cases) {
    it(relative(GOLDEN, dir), () => {
      const src = readFileSync(join(dir, "input.kash"), "utf8");
      const opts = caseOptions(dir, src);
      let diags: Diagnostic[];
      const lib = compileLibraryWith(src, opts);
      if (lib !== null) {
        diags = lib;
      } else {
        const [scene, ds] = compileWith(src, opts);
        diags = ds;
        const sceneFile = join(dir, "scene.json");
        if (existsSync(sceneFile)) {
          expect(pretty(scene.toJson())).toBe(readFileSync(sceneFile, "utf8"));
        }
      }
      sort(diags);
      const got = diagnostics(diags.filter((x) => x.severity !== "warn").map((x) => x.toJson()));
      const want = withoutWarnings(readFileSync(join(dir, "diagnostics.txt"), "utf8"));
      expect(got).toBe(want);
    });
  }
});
