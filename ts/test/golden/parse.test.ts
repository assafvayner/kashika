import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { type Diagnostic, sort } from "../../src/diag/index.js";
import { diagnostics, foldDebug, pretty } from "../../src/geom/dump.js";
import { type Scene, compileLibraryWith, compileWith } from "../../src/model/index.js";
import { debugFile, format, parse } from "../../src/syntax/index.js";
import {
  Case,
  GOLDEN,
  type Output,
  caseOptions,
  checkOutputs,
  discover,
  textOutput,
} from "../support/golden.js";

function runParse(c: Case): Output[] {
  const { file } = parse(c.src);
  const opts = caseOptions(c);
  let scene: Scene | null = null;
  let diags: Diagnostic[];
  const lib = compileLibraryWith(c.src, opts);
  if (lib !== null) {
    diags = lib;
  } else {
    const [s, ds] = compileWith(c.src, opts);
    ds.push(...check(s));
    sort(ds);
    scene = s;
    diags = ds;
  }

  const once = format(c.src);
  if (once.ok) {
    const twice = format(once.value);
    if (!twice.ok) throw new Error(`${c.name}: fmt output no longer parses`);
    if (once.value !== twice.value) {
      throw new Error(`${c.name}: fmt is not idempotent on this input`);
    }
  }

  const out = [textOutput("diagnostics.txt", diagnostics(diags.map((d) => d.toJson())))];
  if (c.dumpAst) {
    out.push(textOutput("ast.txt", foldDebug(debugFile(file))));
  }
  if (c.dumpScene) {
    if (scene === null) throw new Error(`${c.name}: a library has no scene`);
    out.push(textOutput("scene.json", pretty(scene.toJson())));
  }
  return out;
}

const dirs = discover(join(GOLDEN, "parse"));

describe("golden parse cases", () => {
  it("finds the cases", () => {
    expect(dirs.length).toBeGreaterThan(200);
  });
  for (const dir of dirs) {
    const c = Case.load(GOLDEN, dir);
    it(c.name, () => {
      checkOutputs(c, runParse(c));
    });
  }
});
