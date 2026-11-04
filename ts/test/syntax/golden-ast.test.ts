import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";
import { foldDebug } from "../../src/geom/dump.js";
import { debugFile, format, parse } from "../../src/syntax/index.js";

const GOLDEN = join(import.meta.dirname, "../../../tests/golden");

function caseDirs(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) {
      if (existsSync(join(p, "input.kash"))) out.push(p);
      caseDirs(p, out);
    }
  }
  return out;
}

const cases = caseDirs(GOLDEN);
const astCases = cases.filter((d) => existsSync(join(d, "ast.txt")));

describe("golden ast.txt", () => {
  it("finds the cases", () => {
    expect(astCases.length).toBe(36);
  });
  for (const dir of astCases) {
    it(relative(GOLDEN, dir), () => {
      const src = readFileSync(join(dir, "input.kash"), "utf8");
      const want = readFileSync(join(dir, "ast.txt"), "utf8");
      expect(foldDebug(debugFile(parse(src).file))).toBe(want);
    });
  }
});

describe("golden fmt idempotence", () => {
  for (const dir of cases) {
    it(relative(GOLDEN, dir), () => {
      const src = readFileSync(join(dir, "input.kash"), "utf8");
      const once = format(src);
      if (once.ok) {
        const twice = format(once.value);
        expect(twice.ok).toBe(true);
        if (twice.ok) expect(twice.value).toBe(once.value);
      }
    });
  }
});
