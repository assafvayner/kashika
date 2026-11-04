import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { expect, test } from "vitest";

import { kashika, tempdir } from "../support/cli.js";

const SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><path d="M 10 0 L 30 0 L 40 20 L 0 20 Z"/></svg>';

function write(dir: string, name: string, body: string): void {
  const p = join(dir, name);
  mkdirSync(dirname(p), { recursive: true });
  writeFileSync(p, body);
}

test("render_resolves_files_next_to_the_input_and_embeds_them", () => {
  const dir = tempdir();
  write(dir, "lib/net.kash", 'shape lb from "../icons/lb.svg"\n');
  write(dir, "icons/lb.svg", SVG);
  write(
    dir,
    "arch.kash",
    'diagram "a" size (300,200)\nimport "lib/net.kash" as net\nbox lb "LB" at (20,20) size (80,40) shape=net.lb\n',
  );
  const out = join(dir, "arch.svg");
  const r = kashika(["render", join(dir, "arch.kash"), "-o", out], { cwd: tmpdir() });
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("output: written");
  expect(readFileSync(out, "utf8")).toContain('href="data:image/svg+xml;base64,');
});

test("escaping_paths_need_allow_path", () => {
  const tmp = tempdir();
  write(tmp, "shared/x.svg", SVG);
  write(
    tmp,
    "proj/a.kash",
    'diagram "a" size (300,200)\nbox a at (0,0) size (40,40) icon="../shared/x.svg"\n',
  );
  const file = join(tmp, "proj/a.kash");
  const r = kashika(["check", file]);
  expect(r.code).toBe(2);
  expect(r.stdout).toContain("error[file]");
  expect(r.stdout).toContain("--allow-path");
  expect(kashika(["check", file, "--allow-path"]).code).toBe(0);
});

test("include_grants_single_files_outside_the_root", () => {
  const tmp = tempdir();
  write(tmp, "lib/pieces.kash", 'import "base.kash"\nshape king from "king.svg"\n');
  write(tmp, "lib/base.kash", "style piece { fill=#eeeeee }\n");
  write(tmp, "lib/king.svg", SVG);
  write(
    tmp,
    "diagrams/board.kash",
    'diagram "b" size (300,200)\nimport "../lib/pieces.kash"\nbox k at (20,20) size (40,40) shape=king style=piece\n',
  );
  const cwd = join(tmp, "diagrams");
  const run = (extra: string[]) => kashika(["check", "board.kash", ...extra], { cwd });
  let r = run([]);
  expect(r.code).toBe(2);
  expect(r.stdout).toContain("`../lib/pieces.kash` is outside the project root");
  r = run(["--include", "../lib/pieces.kash"]);
  expect(r.code).toBe(2);
  expect(r.stdout).toContain("`base.kash` is outside the project root");
  expect(r.stdout).toContain("`king.svg` is outside the project root");
  expect(
    run([
      "--include",
      "../lib/pieces.kash",
      "--include",
      "../lib/base.kash",
      "--include",
      "../lib/king.svg",
    ]).code,
  ).toBe(0);
  for (const cmd of [
    ["inspect", "board.kash"],
    ["route", "board.kash", "k.right", "(200,40)"],
    ["render", "board.kash", "-o", "b.svg"],
  ]) {
    r = kashika([...cmd, "--include", "../lib/nope.kash"], { cwd });
    expect(r.code, cmd.join(" ")).toBe(3);
    expect(r.stderr).toContain("--include `../lib/nope.kash`");
  }
  r = run(["--include", "../lib"]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain("is not a file");
  r = run(["--include", "../lib/nope.kash", "--format", "json"]);
  expect(r.code).toBe(3);
  expect(r.stdout).toContain('"tool_error"');
});

test("icon_path_comes_from_the_environment", () => {
  const tmp = tempdir();
  write(tmp, "vendor/aws/s3.svg", SVG);
  write(
    tmp,
    "proj/a.kash",
    'diagram "a" size (300,200)\nbox a at (0,0) size (40,40) icon="aws/s3.svg"\n',
  );
  const file = join(tmp, "proj/a.kash");
  expect(kashika(["check", file], { env: { KASHIKA_ICON_PATH: null } }).code).toBe(2);
  expect(kashika(["check", file], { env: { KASHIKA_ICON_PATH: join(tmp, "vendor") } }).code).toBe(
    0,
  );
});

test("stdin_resolves_from_the_current_directory", () => {
  const dir = tempdir();
  write(dir, "x.svg", SVG);
  const r = kashika(["check", "-"], {
    cwd: dir,
    input: 'diagram "a" size (100,100)\nbox a at (0,0) size (40,40) icon="x.svg"\n',
  });
  expect(r.code).toBe(0);
});

test("icons_lists_bundled_icons_and_directory_contents", () => {
  const dir = tempdir();
  write(dir, "aws/s3.svg", SVG);
  write(dir, "lib.kash", 'shape lb {\n  path "M 0 0 L 1 1"\n}\n');
  write(dir, "notes.txt", "x");
  let r = kashika(["icons", dir]);
  expect(r.code).toBe(0);
  for (const s of ["server", "builtin:server", "aws/s3.svg", "40x20", "lb "]) {
    expect(r.stdout).toContain(s);
  }
  expect(r.stdout).not.toContain("notes");
  r = kashika(["icons", dir, "--search", "S3"]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("s3");
  expect(r.stdout).not.toContain("server");
  r = kashika(["icons", dir, "--format", "json"]);
  const v = JSON.parse(r.stdout);
  expect(v.schema).toBe("kashika.icons/1");
  const s3 = v.icons.find((e: { name: string }) => e.name === "s3");
  expect(s3.size).toEqual([40.0, 20.0]);
  expect(s3.path).toBe("aws/s3.svg");
});

test("icons_search_matches_keywords", () => {
  let r = kashika(["icons", "--search", "user"]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("builtin:person");
  expect(r.stdout).toContain("builtin:shape:person");
  expect(r.stdout).toContain('(matches "user")');
  r = kashika(["icons", "--search", "database"]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("cylinder");
  expect(r.stdout).toContain('(matches "database")');
  r = kashika(["icons", "--search", "cylinder"]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("cylinder");
  expect(r.stdout).not.toContain("matches");
  const v = JSON.parse(kashika(["icons", "--search", "db", "--format", "json"]).stdout);
  expect(v.icons[0].name).toBe("database");
  expect(v.icons[0].matched_keyword).toBe("db");
  expect(v.icons[1].name).toBe("cylinder");
  expect(v.icons[1].matched_keyword).toBe("db");
  r = kashika(["icons", "--search", "zzznomatch"]);
  expect(r.code).toBe(1);
  expect(r.stderr).toContain("no icons or shapes match `zzznomatch`");
});

test("icons_defaults_to_the_icon_path", () => {
  const dir = tempdir();
  write(dir, "db.svg", SVG);
  const r = kashika(["icons"], { env: { KASHIKA_ICON_PATH: dir } });
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("db.svg");
});

test("inspect_reports_image_natural_size_and_fit", () => {
  const dir = tempdir();
  write(dir, "logo.svg", SVG);
  write(
    dir,
    "a.kash",
    'diagram "a" size (300,200)\nimage logo "logo.svg" at (20,20) size (80,80) fit=stretch\n',
  );
  const file = join(dir, "a.kash");
  const r = kashika(["inspect", file]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("image logo (line 2)");
  expect(r.stdout).toContain('  image "logo.svg" natural 40x20 aspect 2 fit=stretch');
  const v = JSON.parse(kashika(["inspect", file, "--format", "json"]).stdout);
  const img = v.shapes[0].image;
  expect(img.natural).toEqual([40.0, 20.0]);
  expect(img.aspect).toBe(2.0);
  expect(img.fit).toBe("stretch");
});
