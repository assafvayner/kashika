import { readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { type Diagnostic, sort } from "../../src/diag/index.js";
import { diagnostics } from "../../src/geom/dump.js";
import { compileLibraryWith, compileWith } from "../../src/model/index.js";
import { GOLDEN, caseDirs, caseOptions } from "../support/golden.js";

const cases = caseDirs(join(GOLDEN, "parse"));

describe("golden parse cases: diagnostics.txt with warnings", () => {
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
        diags.push(...check(scene));
        sort(diags);
      }
      const got = diagnostics(diags.map((d) => d.toJson()));
      expect(got).toBe(readFileSync(join(dir, "diagnostics.txt"), "utf8"));
    });
  }
});
