import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

import { type Diagnostic } from "../../src/diag/index.js";
import { lines } from "../../src/diag/rust.js";
import { Options, type Scene, compileFile } from "../../src/model/index.js";
import { renderSvgWith } from "../../src/render/index.js";
import { DIVERGENT } from "../golden/divergent.js";

const GOLDEN = join(import.meta.dirname, "../../../tests/golden");

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
  force: boolean;
  debug: boolean;
  allowPath: boolean;
  iconPath: string | null;
  include: string[];
}

/// The `// key: value` lines at the top of `input.kash`, as `case.rs` reads them.
function directives(src: string): Directives {
  const d: Directives = { force: false, debug: false, allowPath: false, iconPath: null, include: [] };
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
    if (key === "flags") {
      d.force ||= values.includes("force");
      d.debug ||= values.includes("debug");
      d.allowPath ||= values.includes("allow-path");
    }
    if (key === "icon-path") d.iconPath = values[0] ?? null;
    if (key === "include") d.include.push(...values);
  }
  return d;
}

/// The diagnostics `kashika render` reports for `scene`: the compile diagnostics, and the
/// lints once the check module is ported.
function collectDiagnostics(_scene: Scene, diags: Diagnostic[]): Diagnostic[] {
  return diags;
}

/// Renders like `kashika render input.kash -o out.svg --show-suppressed` in the case directory.
function render(dir: string): string {
  const src = readFileSync(join(dir, "input.kash"), "utf8");
  const d = directives(src);
  const opts = new Options({
    path: join(dir, "input.kash"),
    allowPath: d.allowPath,
    iconPath: d.iconPath === null ? [] : [join(dir, d.iconPath)],
    include: d.include.map((p) => join(dir, p)),
  });
  const { scene, diags } = compileFile(src, opts);
  if (scene === null) throw new Error("a library has no scene");
  const all = collectDiagnostics(scene, diags);
  const errors = all.filter((x) => x.severity === "error").length;
  if (errors > 0 && !d.force) throw new Error(`${errors} errors without --force: nothing is written`);
  return renderSvgWith(scene, all, { errorMarkers: errors > 0, debug: d.debug });
}

const cases = caseDirs(join(GOLDEN, "render"));

/// Cases whose `out.svg` marks lint warnings (`--debug`), which `collectDiagnostics` does not
/// report until the check module is ported. They are expected to fail until then.
const PENDING_CHECK: ReadonlySet<string> = new Set(["render/cli/debug-overlay"]);

describe("golden render cases: out.svg", () => {
  it("finds the cases", () => {
    expect(cases.length).toBeGreaterThan(50);
  });
  it("lists only existing cases as divergent", () => {
    const names = new Set(cases.map((dir) => relative(GOLDEN, dir)));
    expect([...DIVERGENT.keys()].filter((n) => !names.has(n))).toEqual([]);
  });
  for (const dir of cases) {
    const name = relative(GOLDEN, dir);
    const reason = DIVERGENT.get(name);
    const test = PENDING_CHECK.has(name) ? it.fails : it;
    test(reason === undefined ? name : `${name} (divergent: ${reason})`, () => {
      const got = render(dir);
      const want = readFileSync(join(dir, "out.svg"), "utf8");
      if (reason === undefined) {
        expect(got).toBe(want);
      } else if (got === want) {
        throw new Error(`${name} matches out.svg: remove it from divergent.ts`);
      }
    });
  }
});
