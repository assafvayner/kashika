import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

import { type Diagnostic, sort } from "../../src/diag/index.js";
import { lines } from "../../src/diag/rust.js";
import { diagnostics, pretty } from "../../src/geom/dump.js";
import { Options, compileLibraryWith, compileWith } from "../../src/model/index.js";

const GOLDEN = join(import.meta.dirname, "../../../tests/golden");

/// Every directory under `dir` holding an `input.kash`; case directories are not searched further.
function caseDirs(dir: string, out: string[] = []): string[] {
  if (existsSync(join(dir, "input.kash"))) {
    out.push(dir);
    return out;
  }
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) caseDirs(p, out);
  }
  return out;
}

interface Directives {
  allowPath: boolean;
  iconPath: string | null;
  include: string[];
}

/// The `// key: value` lines at the top of `input.kash`, as the Rust golden harness reads them.
function directives(src: string): Directives {
  const d: Directives = { allowPath: false, iconPath: null, include: [] };
  const text = src.startsWith("﻿") ? src.slice(1) : src;
  for (const line of lines(text)) {
    if (!line.startsWith("// ")) break;
    const body = line.slice(3);
    const at = body.indexOf(": ");
    if (at < 0) continue;
    const key = body.slice(0, at);
    if (!/^[a-z-]*$/.test(key)) continue;
    const values = body
      .slice(at + 2)
      .split(/[, ]/)
      .filter((v) => v !== "");
    if (key === "flags" && values.includes("allow-path")) d.allowPath = true;
    if (key === "icon-path") d.iconPath = values[0] ?? null;
    if (key === "include") d.include.push(...values);
  }
  return d;
}

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
      const d = directives(src);
      const opts = new Options({
        path: join(dir, "input.kash"),
        allowPath: d.allowPath,
        iconPath: d.iconPath === null ? [] : [join(dir, d.iconPath)],
        include: d.include.map((p) => join(dir, p)),
      });
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
