import {
  chmodSync,
  lstatSync,
  readFileSync,
  readdirSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";

import { type Diagnostic } from "../../src/diag/diagnostic.js";
import { compile } from "../../src/model/index.js";
import { renderSvg } from "../../src/render/index.js";
import { format } from "../../src/syntax/index.js";
import { CLI_CRATE, ROOT, kashika, tempdir } from "../support/cli.js";

const MESSY =
  'diagram  "d" size (200, 100)\nbox a at ( 10,10 ) size (40, 20)   // a box\n\n\n\nbox b at a.right+( 40 ,0) size (40, 20) pin=left\n';
const CLEAN =
  'diagram "d" size (200, 100)\nbox a at (10, 10) size (40, 20)  // a box\n\nbox b at a.right+(40, 0) size (40, 20) pin=left\n';

test("rewrites_file_in_place", () => {
  const dir = tempdir();
  const f = join(dir, "d.kash");
  writeFileSync(f, MESSY);
  const r = kashika(["fmt", f]);
  expect([r.code, r.stdout]).toEqual([0, ""]);
  expect(readFileSync(f, "utf8")).toBe(CLEAN);
  expect(readdirSync(dir).length).toBe(1);
});

test("check_reports_without_writing", () => {
  const dir = tempdir();
  const f = join(dir, "d.kash");
  writeFileSync(f, MESSY);
  const r = kashika(["fmt", f, "--check"]);
  expect(r.code).toBe(1);
  expect(r.stdout).toContain("not formatted");
  expect(readFileSync(f, "utf8")).toBe(MESSY);
  writeFileSync(f, CLEAN);
  expect(kashika(["fmt", f, "--check"]).code).toBe(0);
});

test("format_is_an_alias_of_fmt", () => {
  const dir = tempdir();
  const f = join(dir, "d.kash");
  writeFileSync(f, MESSY);
  expect(kashika(["format", f]).code).toBe(0);
  expect(readFileSync(f, "utf8")).toBe(CLEAN);
});

test("parse_errors_exit_two_and_leave_the_file", () => {
  const dir = tempdir();
  const f = join(dir, "bad.kash");
  writeFileSync(f, "box a at\n");
  const r = kashika(["fmt", f]);
  expect(r.code).toBe(2);
  expect(r.stdout).toContain("error[parse]");
  expect(readFileSync(f, "utf8")).toBe("box a at\n");
});

test("stdin_name_names_stdin_in_reports", () => {
  let r = kashika(["fmt", "-", "--stdin-name", "draft.kash"], { input: "box a at\n" });
  expect(r.code).toBe(2);
  expect(r.stdout).toContain("draft.kash:1:");
  expect(r.stdout).not.toContain("<stdin>");
  r = kashika(["fmt", "-", "--check", "--stdin-name", "draft.kash"], {
    input: 'diagram "d" size (100, 100)\nbox  a at (0, 0) size (10, 10)\n',
  });
  expect([r.code, r.stdout]).toEqual([1, "draft.kash: not formatted\n"]);
});

test("missing_file_is_a_tool_error", () => {
  expect(kashika(["fmt", "nope.kash"]).code).toBe(3);
});

/// Every .kash in the repo formats idempotently and renders the same before and after.
test("corpus_round_trips", () => {
  const files: string[] = [];
  const dirs = [join(CLI_CRATE, "tests/fixtures")];
  for (const e of readdirSync(join(ROOT, "examples"))) {
    const p = join(ROOT, "examples", e);
    if (statSync(p).isDirectory()) dirs.push(p);
  }
  for (const dir of dirs) {
    for (const e of readdirSync(dir)) {
      if (e.endsWith(".kash")) files.push(join(dir, e));
    }
  }
  expect(files.length).toBeGreaterThan(0);
  for (const f of files) {
    const src = readFileSync(f, "utf8");
    const once = format(src);
    if (!once.ok) continue;
    const twice = format(once.value);
    expect(twice.ok && twice.value, f).toBe(once.value);
    const [a, da] = compile(src);
    const [b, db] = compile(once.value);
    expect(renderSvg(a), f).toBe(renderSvg(b));
    const codes = (d: Diagnostic[]) => d.map((x) => [x.code, x.message]);
    expect(codes(da), f).toEqual(codes(db));
  }
});

test("pin_with_position_round_trips", () => {
  const src = 'diagram "d" size (200, 100)\nbox a at (10, 10) size (40, 20) pin=left@0.25\n';
  const dir = tempdir();
  const f = join(dir, "d.kash");
  writeFileSync(f, src);
  expect(kashika(["fmt", f, "--check"]).code).toBe(0);
  writeFileSync(f, src.replace("pin=left@0.25", "pin=left @ 0.25"));
  expect(kashika(["fmt", f]).code).toBe(0);
  expect(readFileSync(f, "utf8")).toBe(src);
});

test("color_names_round_trip", () => {
  const dir = tempdir();
  const f = join(dir, "d.kash");
  const src =
    'diagram "d" size (200, 100) background=Ivory\nbox a at (10, 10) size (40, 20) fill=SteelBlue color=white\n';
  writeFileSync(f, src);
  expect(kashika(["fmt", f, "--check"]).code).toBe(0);
  expect(kashika(["fmt", f]).code).toBe(0);
  expect(readFileSync(f, "utf8")).toBe(src);
});

test("writes_through_symlink", () => {
  const dir = tempdir();
  const real = join(dir, "real.kash");
  const link = join(dir, "link.kash");
  writeFileSync(real, MESSY);
  symlinkSync(real, link);
  expect(kashika(["fmt", link]).code).toBe(0);
  expect(lstatSync(link).isSymbolicLink()).toBe(true);
  expect(readFileSync(real, "utf8")).toBe(CLEAN);
});

test("preserves_file_mode", () => {
  const dir = tempdir();
  const f = join(dir, "d.kash");
  writeFileSync(f, MESSY);
  chmodSync(f, 0o640);
  expect(kashika(["fmt", f]).code).toBe(0);
  expect(readFileSync(f, "utf8")).toBe(CLEAN);
  expect(statSync(f).mode & 0o777).toBe(0o640);
});

test("render_writes_through_symlink", () => {
  const dir = tempdir();
  const src = join(dir, "d.kash");
  const real = join(dir, "real.svg");
  const link = join(dir, "link.svg");
  writeFileSync(src, CLEAN);
  writeFileSync(real, "old");
  symlinkSync(real, link);
  expect(kashika(["render", src, "-o", link]).code).toBe(0);
  expect(lstatSync(link).isSymbolicLink()).toBe(true);
  expect(readFileSync(real, "utf8").startsWith("<svg")).toBe(true);
});
