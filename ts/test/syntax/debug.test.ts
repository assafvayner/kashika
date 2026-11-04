import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { foldDebug } from "../../src/geom/dump.js";
import { debugFile, debugStr, parse } from "../../src/syntax/index.js";

const MINIMAL = join(import.meta.dirname, "../../../tests/golden/parse/smoke/minimal");

describe("debug", () => {
  it("minimal_ast_matches_the_golden", () => {
    const src = readFileSync(join(MINIMAL, "input.kash"), "utf8");
    const want = readFileSync(join(MINIMAL, "ast.txt"), "utf8");
    expect(foldDebug(debugFile(parse(src).file))).toBe(want);
  });

  it("prints_unfolded_pretty_debug", () => {
    const got = debugFile(parse("").file);
    expect(got).toBe("File {\n    stmts: [],\n    first_stmt: None,\n    failed_ids: [],\n    failed_diagram: false,\n}");
    const one = debugFile(parse("junction j at (1,2) dot\n").file);
    expect(one).toContain(
      "        Junction(\n            Junction {\n                id: Ident {\n                    name: \"j\",\n",
    );
    expect(one).toContain("value: Flag,");
    expect(one).toContain("x: Num(\n");
  });

  it("escapes_strings_like_rust_debug", () => {
    expect(
      debugStr("e\u{301}x\u{200b}\u{a0}\u{7f}\u{1}\0\t\r\n\\\"'é\u{e000}\u{10ffff}\u{300} •"),
    ).toBe(
      '"e\\u{301}x\\u{200b}\\u{a0}\\u{7f}\\u{1}\\0\\t\\r\\n\\\\\\"\'é\\u{e000}\\u{10ffff}\\u{300} •"',
    );
  });
});
