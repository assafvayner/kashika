import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterEach, describe, expect, test } from "vitest";

import { type Diagnostic } from "../../src/diag/diagnostic.js";
import { cmpTuple } from "../../src/diag/rust.js";
import { Pt, Rect } from "../../src/geom/index.js";
import {
  Options,
  type Scene,
  type Shape,
  compileLibraryWith,
  compileWith,
} from "../../src/model/index.js";

const HEAD = 'diagram "t" size (800,600)\n';

const SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><path d="M 10 0 L 30 0 L 40 20 L 0 20 Z" fill="#123456"/></svg>';

const TRI = 'shape lb {\n  path "M 0.5 0 L 1 1 L 0 1 Z"\n}\n';

const temps: string[] = [];

afterEach(() => {
  for (const t of temps.splice(0)) rmSync(t, { recursive: true, force: true });
});

function tempdir(): string {
  const d = mkdtempSync(join(tmpdir(), "kashika-"));
  temps.push(d);
  return d;
}

/// A PNG header with the given size; enough for `imagesize`.
function png(w: number, h: number): Uint8Array {
  const b = Buffer.alloc(33);
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52]).copy(b);
  b.writeUInt32BE(w, 16);
  b.writeUInt32BE(h, 20);
  Buffer.from([8, 6, 0, 0, 0, 0, 0, 0, 0]).copy(b, 24);
  return b;
}

function shape(s: Scene, id: string): Shape {
  const x = s.shapes.find((x) => x.id === id);
  if (x === undefined) throw new Error(`no shape ${id}`);
  return x;
}

function start(s: Scene, id: string): Pt {
  const l = s.lines.find((l) => l.id === id);
  if (l === undefined) throw new Error(`no line ${id}`);
  return l.path[0]!.from;
}

function close(a: Pt, b: Pt): boolean {
  return a.dist(b) <= 0.3;
}

function write(dir: string, name: string, body: Uint8Array | string): void {
  const p = join(dir, name);
  mkdirSync(dirname(p), { recursive: true });
  writeFileSync(p, body);
}

function opts(dir: string, extra: Partial<Options> = {}): Options {
  return new Options({ path: join(dir, "main.kash"), ...extra });
}

function runWith(src: string, o: Options): [Scene, Diagnostic[]] {
  return compileWith(src, o);
}

/// Writes `src` as `main.kash` in `dir` (so the main file's path exists) and compiles it.
function run(dir: string, src: string): [Scene, Diagnostic[]] {
  write(dir, "main.kash", src);
  return runWith(src, opts(dir));
}

function codes(d: Diagnostic[]): string[] {
  return d.map((x) => x.code);
}

/// `n0.kash` .. `n{depth-1}.kash` each import the next twice, `as p` and `as q`; the last
/// defines `s`.
function fanOut(dir: string, depth: number): void {
  for (let i = 0; i < depth; i++) {
    const next = i + 1;
    write(dir, `n${i}.kash`, `import "n${next}.kash" as p\nimport "n${next}.kash" as q\n`);
  }
  write(dir, `n${depth}.kash`, TRI.replace("lb", "s"));
}

describe("files", () => {
  test("icons_dir_must_exist", () => {
    const dir = tempdir();
    const [, d] = run(dir, 'diagram "t" size (800,600) icons="nope"\n');
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("file");
    expect(d[0]!.message.includes("`nope`"), d[0]!.message).toBe(true);
  });

  test("icons_dir_inside_the_project_is_accepted", () => {
    const dir = tempdir();
    mkdirSync(join(dir, "assets/icons"), { recursive: true });
    const [, d] = run(dir, 'diagram "t" size (800,600) icons="assets/icons"\n');
    expect(d, JSON.stringify(d)).toEqual([]);
  });

  test("icons_dir_outside_the_project_needs_allow_path", () => {
    const tmp = tempdir();
    const proj = join(tmp, "proj");
    mkdirSync(join(tmp, "shared"), { recursive: true });
    mkdirSync(proj, { recursive: true });
    const src = 'diagram "t" size (800,600) icons="../shared"\n';
    let [, d] = run(proj, src);
    expect(d[0]!.code).toBe("file");
    expect(d[0]!.message.includes("outside the project root"), d[0]!.message).toBe(true);
    [, d] = runWith(src, opts(proj, { allowPath: true }));
    expect(d, JSON.stringify(d)).toEqual([]);
  });

  test("icons_attr_must_be_a_string", () => {
    const dir = tempdir();
    const [, d] = run(dir, 'diagram "t" size (800,600) icons=12\n');
    expect(d[0]!.code).toBe("attr");
  });

  test("svg_shapes_use_bbox_or_hull", () => {
    const dir = tempdir();
    write(dir, "t.svg", SVG);
    const [s, d] = run(
      dir,
      `${HEAD}shape tb from "t.svg"\nshape th from "t.svg" outline=hull text-area=below\n` +
        `box a "A" at (100,100) size (80,40) shape=tb\nbox b "B" at (300,100) size (80,40) shape=th\n` +
        `line la a.top@0.1 -> v -20\nline lb b.top@0.1 -> v -20\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    expect(start(s, "la")).toEqual(new Pt(108.0, 100.0));
    expect(close(start(s, "lb"), new Pt(308.0, 124.0)), JSON.stringify(start(s, "lb"))).toBe(true);
    const a = shape(s, "a");
    expect(a.image!.fit).toBe("contain");
    expect(a.image!.rect).toEqual(new Rect(100.0, 100.0, 80.0, 40.0));
    expect(a.textArea).toEqual(new Rect(108.0, 108.0, 64.0, 24.0));
    expect(shape(s, "b").textArea).toEqual(new Rect(300.0, 148.0, 80.0, 17.5));
    expect(s.assets.length, "both shapes share one asset").toBe(1);
    expect(s.assets[0]!.natural).toEqual([40.0, 20.0]);
    expect(s.assets[0]!.path).toBe("t.svg");
  });

  test("svg_shapes_contain_their_drawing_by_default", () => {
    const dir = tempdir();
    write(dir, "t.svg", SVG);
    const [s, d] = run(
      dir,
      `${HEAD}shape tb from "t.svg"\nshape tl from "t.svg" text-area=below\n` +
        `shape th from "t.svg" outline=hull\n` +
        `box a "A" at (100,100) size (80,80) shape=tb\n` +
        `box b "B" at (300,100) size (80,80) shape=tl\n` +
        `box c at (100,300) size (120,40) shape=th\n` +
        `box d at (500,100) size (80,80) shape=tb pin=top\n` +
        `line la a.top -> v -20\nline lc c.left -> h -20\nline lk a.top-left -> v -20\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    const a = shape(s, "a");
    expect(a.image!.fit).toBe("contain");
    expect(a.rect).toEqual(new Rect(100.0, 100.0, 80.0, 80.0));
    expect(a.anchorFrame()).toEqual(new Rect(100.0, 120.0, 80.0, 40.0));
    expect(start(s, "la")).toEqual(new Pt(140.0, 120.0));
    expect(start(s, "lk")).toEqual(new Pt(100.0, 120.0));
    expect(a.textArea).toEqual(new Rect(108.0, 128.0, 64.0, 24.0));
    expect(shape(s, "d").rect).toEqual(new Rect(460.0, 80.0, 80.0, 80.0));
    expect(shape(s, "b").textArea).toEqual(new Rect(300.0, 168.0, 80.0, 17.5));
    expect(close(start(s, "lc"), new Pt(130.0, 320.0)), JSON.stringify(start(s, "lc"))).toBe(true);
  });

  test("svg_shapes_take_cover_and_stretch", () => {
    const dir = tempdir();
    write(dir, "t.svg", SVG);
    const [s, d] = run(
      dir,
      `${HEAD}shape tc from "t.svg" fit=cover outline=hull\nshape ts from "t.svg" fit=stretch\n` +
        `box a at (400,300) size (40,40) shape=tc\nbox b at (100,100) size (80,80) shape=ts\n` +
        `line lb b.top -> v -20\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    const a = shape(s, "a");
    expect(a.image!.fit).toBe("cover");
    expect(a.anchorFrame()).toEqual(a.rect);
    const hull = a.outlinePolygon();
    expect(
      hull.every((p) => p.x >= 400.0 && p.x <= 440.0 && p.y >= 300.0 && p.y <= 340.0),
      JSON.stringify(hull),
    ).toBe(true);
    expect(a.imageDistortion(s.assets)).toBe(null);
    const b = shape(s, "b");
    expect(b.image!.fit).toBe("stretch");
    expect(b.imageDistortion(s.assets)).toBe(0.5);
    expect(start(s, "lb")).toEqual(new Pt(140.0, 100.0));
  });

  test("fitted_svg_shapes_keep_the_drawing_ratio", () => {
    const dir = tempdir();
    write(dir, "t.svg", SVG);
    const [s, d] = run(
      dir,
      `${HEAD}shape tb from "t.svg"\n` +
        `box a "Payments" at (100,100) shape=tb\n` +
        `box b "B" at (300,100) size (200, auto) shape=tb\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    const a = shape(s, "a").rect;
    expect(Math.abs(a.w / a.h - 2.0) < 0.1, JSON.stringify(a)).toBe(true);
    expect(shape(s, "b").rect).toEqual(new Rect(300.0, 100.0, 200.0, 100.0));
  });

  test("text_below_uses_the_vertical_padding", () => {
    const dir = tempdir();
    write(dir, "t.svg", SVG);
    const [s, d] = run(
      dir,
      `${HEAD}shape th from "t.svg" text-area=below\n` +
        `box b "B" at (300,100) size (80,40) shape=th padding=(20,6)\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    expect(shape(s, "b").textArea).toEqual(new Rect(300.0, 146.0, 80.0, 17.5));
  });

  test("svg_shape_file_errors", () => {
    const dir = tempdir();
    write(dir, "icons/s3.svg", SVG);
    write(dir, "x.png", png(4, 4));
    let [, d] = run(dir, `${HEAD}shape a from "icons/s4.svg"\n`);
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("file");
    expect(d[0]!.span.line).toBe(2);
    expect(d[0]!.help).toEqual(["did you mean `icons/s3.svg`?"]);
    [, d] = run(dir, `${HEAD}shape b from "x.png"\n`);
    expect(d[0]!.code).toBe("file");
    expect(d[0]!.message.includes("needs an SVG"), d[0]!.message).toBe(true);
    [, d] = run(dir, `${HEAD}shape c from "icons/s3.svg" outline=circle\n`);
    expect(d[0]!.code).toBe("attr");
  });

  test("bundled_icons_work_as_shapes", () => {
    const dir = tempdir();
    const [s, d] = run(
      dir,
      `${HEAD}shape srv from "server"\nbox a at (0,0) size (48,48) shape=srv\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    expect(s.assets[0]!.path).toBe("builtin:server");
  });

  test("imports_bring_in_shapes_and_styles", () => {
    const dir = tempdir();
    write(dir, "lib/net.kash", `style edge { fill=#ff0000 }\n${TRI}import "inner.kash" as in\n`);
    write(dir, "lib/inner.kash", 'shape dot {\n  path "M 0 0 L 1 0 L 1 1 Z"\n}\n');
    write(dir, "plain.kash", "style svc2 { stroke=#00ff00 }\nstyle service { fill=#000000 }\n");
    const [s, d] = run(
      dir,
      `${HEAD}import "lib/net.kash" as net\nimport "plain.kash"\nimport "plain.kash"\n` +
        `box a at (0,0) size (10,10) shape=net.lb style=net.edge\n` +
        `box b at (20,0) size (10,10) shape=net.in.dot style=svc2\n` +
        `box c at (40,0) size (10,10) style=service\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    expect(shape(s, "a").shape).toBe("net.lb");
    expect(shape(s, "a").style.fill).toBe("#ff0000");
    expect(shape(s, "b").shape).toBe("net.in.dot");
    expect(shape(s, "b").style.stroke).toBe("#00ff00");
    expect(shape(s, "c").style.fill, "imported style overrides the built-in").toBe("#000000");
    expect(shape(s, "c").style.stroke, "key by key").toBe("#1a73e8");
  });

  test("library_styles_set_sizes", () => {
    const dir = tempdir();
    write(dir, "lib.kash", "style card { size=(100,50) }\nstyle boundary { size=(10,10) }\n");
    const [s, d] = run(
      dir,
      `${HEAD}import "lib.kash" as ui\nimport "lib.kash"\n` +
        `box a "A" at (0,0) style=ui.card\nbox b "B" at (0,100) style=card\n`,
    );
    expect(codes(d), JSON.stringify(d)).toEqual(["attr"]);
    expect(d[0]!.message.includes("the `boundary` style can't set `size`")).toBe(true);
    expect(d[0]!.span.line, "reported at the unnamespaced import").toBe(3);
    for (const id of ["a", "b"]) {
      const r = shape(s, id).rect;
      expect([r.w, r.h]).toEqual([100.0, 50.0]);
    }
  });

  test("library_errors_point_at_the_import", () => {
    const dir = tempdir();
    write(dir, "lib/bad.kash", "box n at (0,0) size (1,1)\nstyle s { fil=#fff }\n");
    const [s, d] = run(dir, `${HEAD}import "lib/bad.kash"\n`);
    const got = d
      .map((x): [string, number, string] => [x.code, x.span.line, x.message])
      .sort(cmpTuple);
    expect(got.length, JSON.stringify(d)).toBe(2);
    expect(got[0]![0]).toBe("attr");
    expect(got[0]![1]).toBe(2);
    expect(got[0]![2].startsWith("in `lib/bad.kash` at 2:11: "), got[0]![2]).toBe(true);
    expect(got[1]![0]).toBe("parse");
    expect(
      got[1]![2].startsWith(
        "in `lib/bad.kash` at 1:1: `box` is not allowed in a library; library files may contain only",
      ),
      got[1]![2],
    ).toBe(true);
    expect(s.shapes).toEqual([]);
    expect(s.literals, JSON.stringify(s.literals)).toEqual([]);
  });

  test("import_cycles_are_errors", () => {
    const dir = tempdir();
    write(dir, "a.kash", 'import "b.kash"\n');
    write(dir, "b.kash", 'import "a.kash"\n');
    const [, d] = run(dir, `${HEAD}import "a.kash"\n`);
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("file");
    expect([d[0]!.span.line, d[0]!.span.col]).toEqual([2, 8]);
    expect(d[0]!.message).toBe(
      "in `a.kash` at 1:8: in `b.kash` at 1:8: import cycle: a.kash -> b.kash -> a.kash",
    );
  });

  test("importing_the_main_file_is_a_cycle", () => {
    const dir = tempdir();
    const [, d] = run(dir, `${HEAD}import "main.kash"\n`);
    expect(d[0]!.code).toBe("file");
    expect(d[0]!.message.includes("import cycle: main.kash -> main.kash"), d[0]!.message).toBe(
      true,
    );
  });

  test("import_rules", () => {
    const dir = tempdir();
    write(dir, "plain.kash", "style s { fill=#000000 }\n");
    write(dir, "x.svg", SVG);
    const cases: [string, string, string][] = [
      [
        'box a at (0,0) size (1,1)\nimport "plain.kash"\n',
        "parse",
        "must come right after `diagram`",
      ],
      ['import "plan.kash"\n', "file", "not found"],
      ['import "x.svg"\n', "file", "`.kash`"],
      ['import "server"\n', "file", "bundled icon"],
      ['import "https://x.dev/a.kash"\n', "file", "not a local file"],
      [
        'group g at (0,0) size (9,9) {\n  import "plain.kash"\n}\n',
        "parse",
        "only allowed at the top level",
      ],
      [
        'import "plain.kash"\nstyle s { fill=#ffffff }\n',
        "duplicate-style",
        "already defined in `plain.kash` (imported at 2:8)",
      ],
    ];
    for (const [body, code, needle] of cases) {
      const [, d] = run(dir, `${HEAD}${body}`);
      expect(d.length, `${body}: ${JSON.stringify(d)}`).toBe(1);
      expect(d[0]!.code, body).toBe(code);
      expect(d[0]!.message.includes(needle), `${body}: ${d[0]!.message}`).toBe(true);
    }
    const [, d] = run(dir, `${HEAD}import "plan.kash"\n`);
    expect(d[0]!.help).toEqual(["did you mean `plain.kash`?"]);
  });

  test("styles_from_two_imports_clash", () => {
    const dir = tempdir();
    write(dir, "one.kash", "style s { fill=#000000 }\n");
    write(dir, "two.kash", "style s { fill=#ffffff }\n");
    let [, d] = run(dir, `${HEAD}import "one.kash"\nimport "two.kash"\n`);
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("duplicate-style");
    expect(d[0]!.span.line).toBe(3);
    expect(d[0]!.message).toBe(
      "in `two.kash` at 1:1: style `s` is already defined in `one.kash` (imported at 2:8)",
    );
    [, d] = run(dir, `${HEAD}import "one.kash" as a\nimport "two.kash" as b\n`);
    expect(d, JSON.stringify(d)).toEqual([]);
  });

  test("imported_shapes_clash_by_qualified_name", () => {
    const dir = tempdir();
    write(dir, "tri.kash", TRI);
    let [, d] = run(dir, `${HEAD}import "tri.kash"\n${TRI}`);
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("duplicate-style");
    [, d] = run(
      dir,
      `${HEAD}import "tri.kash" as t\n${TRI}box a at (0,0) size (9,9) shape=t.lb\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
  });

  test("library_style_shapes_resolve_in_the_library_namespace", () => {
    const dir = tempdir();
    write(dir, "lib/icons/t.svg", SVG);
    write(
      dir,
      "lib/aws.kash",
      `style service { shape=lb fill=#ff0000 }\nstyle deep { shape=in.dot }\n` +
        `style icon { shape=t }\nstyle db { shape=cylinder }\n` +
        `${TRI}shape t from "icons/t.svg"\nimport "inner.kash" as in\n`,
    );
    write(dir, "lib/inner.kash", 'shape dot {\n  path "M 0 0 L 1 0 L 1 1 Z"\n}\n');
    const [s, d] = run(
      dir,
      `${HEAD}import "lib/aws.kash" as aws\nstyle mine { shape=aws.lb }\n` +
        `box a at (0,0) size (10,10) style=aws.service\n` +
        `box b at (20,0) size (10,10) style=aws.deep\n` +
        `box c at (40,0) size (10,10) style=aws.icon\n` +
        `box d at (60,0) size (10,10) style=aws.db\n` +
        `box e at (80,0) size (10,10) style=mine\n` +
        `box f at (100,0) size (10,10) style=aws.service shape=rect\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    expect(shape(s, "a").shape).toBe("aws.lb");
    expect(shape(s, "a").style.fill).toBe("#ff0000");
    expect(shape(s, "b").shape).toBe("aws.in.dot");
    expect(shape(s, "c").shape).toBe("aws.t");
    expect(shape(s, "c").image !== null).toBe(true);
    expect(shape(s, "d").shape).toBe("cylinder");
    expect(shape(s, "e").shape).toBe("aws.lb");
    expect(shape(s, "f").shape).toBe("rect");
  });

  test("unknown_shape_in_a_library_style_points_at_the_import", () => {
    const dir = tempdir();
    write(dir, "lib/aws.kash", `style s { shape=lc }\nstyle boundary { shape=lb }\n${TRI}`);
    let [s, d] = run(
      dir,
      `${HEAD}import "lib/aws.kash" as aws\nbox a at (0,0) size (10,10) style=aws.s\n` +
        `box b at (20,0) size (10,10) style=aws.boundary\n`,
    );
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("unknown-shape");
    expect(d[0]!.span.line).toBe(2);
    expect(d[0]!.message).toBe("in `lib/aws.kash` at 1:11: unknown shape `lc`");
    expect(d[0]!.help[0]).toBe("did you mean `lb`?");
    expect(shape(s, "a").shape).toBe("rect");
    expect(shape(s, "b").shape).toBe("aws.lb");

    [s, d] = run(dir, `${HEAD}import "lib/aws.kash"\ngroup g at (0,0) size (50,50) {\n}\n`);
    const got = d.map((x): [string, string] => [x.code, x.message]).sort(cmpTuple);
    expect(got.length, JSON.stringify(d)).toBe(2);
    expect(got[0]![0]).toBe("attr");
    expect(
      got[0]![1].startsWith("in `lib/aws.kash` at 2:18: the `boundary` style can't set `shape`"),
      got[0]![1],
    ).toBe(true);
    expect(got[1]![0]).toBe("unknown-shape");
    expect(shape(s, "g").shape).toBe("rect");
  });

  test("standalone_library_resolves_style_shapes", () => {
    const dir = tempdir();
    const src = `style a { shape=lb }\nstyle b { shape=lc }\n${TRI}`;
    write(dir, "main.kash", src);
    const d = compileLibraryWith(src, opts(dir));
    expect(d, "a library").not.toBe(null);
    expect(d!.length, JSON.stringify(d)).toBe(1);
    expect(d![0]!.code).toBe("unknown-shape");
    expect(d![0]!.span.line).toBe(2);
    expect(d![0]!.help[0]).toBe("did you mean `lb`?");
  });

  test("imports_outside_the_root_need_allow_path", () => {
    const tmp = tempdir();
    const proj = join(tmp, "proj");
    mkdirSync(proj, { recursive: true });
    write(tmp, "shared.kash", TRI);
    const src = `${HEAD}import "../shared.kash"\nbox a at (0,0) size (9,9) shape=lb\n`;
    let [, d] = run(proj, src);
    expect(
      d.some((x) => x.code === "file"),
      JSON.stringify(d),
    ).toBe(true);
    [, d] = runWith(src, opts(proj, { allowPath: true }));
    expect(d, JSON.stringify(d)).toEqual([]);
  });

  test("deep_import_chains_stop", () => {
    const dir = tempdir();
    for (let i = 0; i < 40; i++) {
      write(dir, `l${i}.kash`, `import "l${i + 1}.kash"\n`);
    }
    write(dir, "l40.kash", "");
    const [, d] = run(dir, `${HEAD}import "l0.kash"\n`);
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("file");
    expect(d[0]!.message).toBe(
      "in `l0.kash` at 1:8: in `l1.kash` at 1:8: … 29 more imports …: in `l31.kash` at 1:8: imports are nested more than 32 deep",
    );
  });

  test("skipped_entries_load_no_assets", () => {
    const dir = tempdir();
    write(dir, "a.svg", SVG);
    write(dir, "b.svg", SVG);
    const [s, d] = run(
      dir,
      `${HEAD}image i "a.svg" at (0,0) size (40,20)\nimage i "b.svg" at (0,40) size (40,20)\n` +
        `group g at (100,0) size (100,100) {\n}\ngroup g at (300,0) size (100,100) {\n  box c at (10,10) size (40,40) icon="server"\n}\n`,
    );
    expect(codes(d), JSON.stringify(d)).toEqual(["duplicate-id", "duplicate-id"]);
    expect(s.assets.length, JSON.stringify(s.assets)).toBe(1);
    expect(s.assets[0]!.path.endsWith("a.svg")).toBe(true);
  });

  test("import_depth_is_the_same_from_stdin", () => {
    const dir = tempdir();
    for (let i = 0; i < 40; i++) {
      write(dir, `l${i}.kash`, `import "l${i + 1}.kash"\n`);
    }
    const stdin = new Options({ iconPath: [dir] });
    const [, d] = runWith(`${HEAD}import "l0.kash"\n`, stdin);
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(
      d[0]!.message.endsWith("in `l31.kash` at 1:8: imports are nested more than 32 deep"),
      d[0]!.message,
    ).toBe(true);
  });

  test("fan_out_imports_register_every_namespace", () => {
    const dir = tempdir();
    fanOut(dir, 4);
    const [s, d] = run(
      dir,
      `${HEAD}import "n0.kash" as a\nbox b at (0,0) size (9,9) shape=a.q.p.q.p.s\nbox c at (20,0) size (9,9) shape=a.p.p.p.p.s\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    expect(s.shapes.length).toBe(2);
  });

  test("fan_out_imports_are_capped", () => {
    const dir = tempdir();
    fanOut(dir, 24);
    const started = Date.now();
    const [, d] = run(dir, `${HEAD}import "n0.kash" as a\n`);
    const elapsed = Date.now() - started;
    expect(elapsed < 10_000, `${elapsed}ms`).toBe(true);
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("file");
    expect(
      d[0]!.message.endsWith(
        "more than 1000 library imports in total, counting each namespace a library is imported under",
      ),
      d[0]!.message,
    ).toBe(true);
  });

  test("oversized_imports_are_refused", () => {
    const dir = tempdir();
    write(dir, "big.kash", "// padding\n".repeat(100_000));
    const [, d] = run(dir, `${HEAD}import "big.kash"\n`);
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.message).toBe("`big.kash` is larger than 1 MiB");
  });

  test("library_files_check_on_their_own", () => {
    const dir = tempdir();
    write(dir, "x.svg", SVG);
    write(dir, "inner.kash", "style s { fil=#fff }\n");
    const src =
      'import "inner.kash"\nimport "gone.kash"\nshape t from "y.svg"\nshape u from "x.svg"\nstyle a { fill=#fff stroke=nope }\nstyle a { fill=#000 }\n';
    write(dir, "main.kash", src);
    const d = compileLibraryWith(src, opts(dir));
    expect(d, "a library").not.toBe(null);
    const got = d!.map((x): [string, number, string] => [x.code, x.span.line, x.message]);
    expect(got, JSON.stringify(d)).toEqual([
      ["attr", 1, "in `inner.kash` at 1:11: unknown attribute `fil` on style"],
      ["file", 2, "file `gone.kash` not found"],
      ["file", 3, "file `y.svg` not found"],
      ["attr", 5, "`stroke` expects a color like `#1a73e8`, a CSS color name or `none`"],
      ["duplicate-style", 6, "style `a` is defined twice"],
    ]);
    for (const s of [
      "",
      "box a at (0,0) size (1,1)\n",
      "style a { fill=#fff }\nbox a at (0,0) size (1,1)\n",
      'diagram "t" size (10,10)\nstyle a { fill=#fff }\n',
    ]) {
      expect(compileLibraryWith(s, opts(dir)), JSON.stringify(s)).toBe(null);
    }
  });

  test("svg_shapes_in_libraries_resolve_from_the_library", () => {
    const dir = tempdir();
    write(dir, "lib/icons/t.svg", SVG);
    write(dir, "lib/aws.kash", 'shape t from "icons/t.svg"\n');
    const [s, d] = run(
      dir,
      `${HEAD}import "lib/aws.kash" as aws\nbox a at (0,0) size (40,20) shape=aws.t\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    expect(s.assets.length).toBe(1);
    expect(shape(s, "a").image !== null).toBe(true);
  });

  test("images_resolve_with_their_natural_size", () => {
    const dir = tempdir();
    write(dir, "assets/logo.png", png(200, 100));
    const [s, d] = run(
      dir,
      `${HEAD}image logo "assets/logo.png" at (20,20) size (80,80) fit=cover\n` +
        `image mark "assets/logo.png" at logo.right+(20,0) size (40,40) pin=left\n` +
        `line l logo.right -> mark.left\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    const logo = shape(s, "logo");
    expect(logo.kind).toBe("image");
    expect(logo.shape).toBe("image");
    expect(logo.rect).toEqual(new Rect(20.0, 20.0, 80.0, 80.0));
    const img = logo.image!;
    expect([img.fit, img.rect]).toEqual(["cover", logo.rect]);
    expect(shape(s, "mark").image!.fit).toBe("contain");
    expect(shape(s, "mark").rect).toEqual(new Rect(120.0, 40.0, 40.0, 40.0));
    expect(s.assets.length).toBe(1);
    expect([s.assets[0]!.mime, s.assets[0]!.natural]).toEqual(["image/png", [200.0, 100.0]]);
    expect(start(s, "l")).toEqual(new Pt(100.0, 60.0));
  });

  test("stretched_images_report_their_distortion", () => {
    const dir = tempdir();
    write(dir, "logo.png", png(200, 100));
    const [s] = run(
      dir,
      `${HEAD}image a "logo.png" at (0,0) size (80,80) fit=stretch\n` +
        `image b "logo.png" at (100,0) size (80,40) fit=stretch\n` +
        `image c "logo.png" at (200,0) size (80,80)\n`,
    );
    expect(shape(s, "a").imageDistortion(s.assets)).toBe(0.5);
    expect(shape(s, "b").imageDistortion(s.assets)).toBe(0.0);
    expect(shape(s, "c").imageDistortion(s.assets)).toBe(null);
  });

  test("image_errors", () => {
    const dir = tempdir();
    write(dir, "logo.png", png(2, 2));
    const cases: [string, string][] = [
      ['image a "lgo.png" at (0,0) size (8,8)\n', "file"],
      ['image a "logo.png" at (0,0) size (0,8)\n', "size"],
      ['image a "logo.png" at (0,0) size (8,8) fit=fill\n', "attr"],
      ['image a "logo.png" at (0,0) size (8,8) label="x"\n', "attr"],
      ['group g at (0,0) size (9,9) {\n  image a "logo.png" at (0,0) size (8,8)\n}\n', "parse"],
    ];
    for (const [body, code] of cases) {
      const [, d] = run(dir, `${HEAD}${body}`);
      expect(codes(d), `${body}: ${JSON.stringify(d)}`).toEqual([code]);
    }
    const [s] = run(dir, `${HEAD}image a "lgo.png" at (0,0) size (8,8)\n`);
    expect(shape(s, "a").image, "geometry survives a missing file").toBe(null);
  });

  test("icons_shrink_the_text_area", () => {
    const dir = tempdir();
    const [s, d] = run(
      dir,
      `${HEAD}box a "A" at (0,0) size (200,60) icon="server"\n` +
        `box b "B" at (0,100) size (100,100) icon="server" icon-pos=top icon-size=32\n` +
        `group g "G" at (300,0) size (200,100) icon="server" {\n}\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    const a = shape(s, "a");
    expect(a.icon!.rect).toEqual(new Rect(8.0, 18.0, 24.0, 24.0));
    expect(a.icon!.fit).toBe("contain");
    expect(a.textArea).toEqual(new Rect(40.0, 8.0, 152.0, 44.0));
    const b = shape(s, "b");
    expect(b.icon!.rect).toEqual(new Rect(34.0, 108.0, 32.0, 32.0));
    expect(b.textArea).toEqual(new Rect(8.0, 148.0, 84.0, 44.0));
    const g = shape(s, "g");
    expect(g.icon!.rect).toEqual(new Rect(308.0, 8.0, 17.5, 17.5));
    expect(g.textArea).toEqual(new Rect(333.5, 8.0, 158.5, 84.0));
    expect(s.assets.length, "one bundled icon, loaded once").toBe(1);
  });

  test("icons_use_the_padding_axis_of_their_side", () => {
    const dir = tempdir();
    const [s, d] = run(
      dir,
      `${HEAD}box a "A" at (0,0) size (200,60) icon="server" padding=(4,2)\n` +
        `box b "B" at (0,100) size (100,100) icon="server" icon-pos=top icon-size=32 padding=(4,2)\n` +
        `group g "G" at (300,0) size (200,100) icon="server" padding=(4,2) {\n}\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    const a = shape(s, "a");
    expect(a.icon!.rect).toEqual(new Rect(4.0, 18.0, 24.0, 24.0));
    expect(a.textArea).toEqual(new Rect(32.0, 2.0, 164.0, 56.0));
    const b = shape(s, "b");
    expect(b.icon!.rect).toEqual(new Rect(34.0, 102.0, 32.0, 32.0));
    expect(b.textArea).toEqual(new Rect(4.0, 136.0, 92.0, 62.0));
    const g = shape(s, "g");
    expect(g.icon!.rect).toEqual(new Rect(304.0, 2.0, 17.5, 17.5));
    expect(g.textArea).toEqual(new Rect(325.5, 2.0, 170.5, 96.0));
  });

  test("icon_attribute_errors", () => {
    const dir = tempdir();
    const cases: [string, string][] = [
      ["box a at (0,0) size (9,9) icon-pos=top\n", "attr"],
      ['box a at (0,0) size (9,9) icon="server" icon-size=0\n', "attr"],
      ['box a at (0,0) size (9,9) icon="server" icon-pos=right\n', "attr"],
      ['group g at (0,0) size (9,9) icon="server" icon-pos=top {\n}\n', "attr"],
      ['box a at (0,0) size (9,9) icon="servr"\n', "file"],
      [`box a at (0,0) size (9,9) icon="server" icon-size=${"9".repeat(300)}\n`, "attr"],
    ];
    for (const [body, code] of cases) {
      const [, d] = run(dir, `${HEAD}${body}`);
      expect(codes(d), `${body}: ${JSON.stringify(d)}`).toEqual([code]);
    }
  });

  test("icons_are_found_on_the_search_path", () => {
    const tmp = tempdir();
    const proj = join(tmp, "proj");
    write(proj, "assets/icons/db.svg", SVG);
    write(tmp, "vendor/s3.svg", SVG);
    const src =
      'diagram "t" size (800,600) icons="assets/icons"\n' +
      'box a at (0,0) size (90,40) icon="db.svg"\nbox b at (0,50) size (90,40) icon="s3.svg"\n';
    let [, d] = run(proj, src);
    expect(codes(d), JSON.stringify(d)).toEqual(["file"]);
    let s: Scene;
    [s, d] = runWith(src, opts(proj, { iconPath: [join(tmp, "vendor")] }));
    expect(d, JSON.stringify(d)).toEqual([]);
    expect(s.assets.map((a) => a.path)).toEqual(["db.svg", "s3.svg"]);
  });

  test("image_geometry_is_recorded_like_a_box", () => {
    const dir = tempdir();
    write(dir, "logo.png", png(10, 10));
    const [s, d] = run(
      dir,
      `${HEAD}box a at (0,0) size (10,10)\n` +
        `image i "logo.png" at a.right+(5,0) size (30,40) pin=left\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    const lits = s.literals.filter((l) => l.owner === "i").map((l) => [l.what, l.value]);
    expect(lits).toEqual([
      ["dx", 5.0],
      ["dy", 0.0],
      ["width", 30.0],
      ["height", 40.0],
    ]);
    expect(s.placement.some((e) => e.from === "i" && e.to === "a" && e.kind === "at")).toBe(true);
  });

  test("images_paint_with_boxes_in_file_order", () => {
    const dir = tempdir();
    write(dir, "logo.png", png(10, 10));
    const [s, d] = run(
      dir,
      `${HEAD}box a at (0,0) size (10,10)\n` +
        `image i "logo.png" at (20,0) size (10,10)\n` +
        `box b at (40,0) size (10,10)\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    expect(s.shapes.map((x) => x.id)).toEqual(["a", "i", "b"]);
  });

  test("duplicate_and_unknown_image_references", () => {
    const dir = tempdir();
    write(dir, "logo.png", png(10, 10));
    const [, d] = run(
      dir,
      `${HEAD}box i at (0,0) size (10,10)\n` + `image i "logo.png" at (20,0) size (10,10)\n`,
    );
    expect(codes(d)).toEqual(["duplicate-id"]);
  });

  test("icon_overflow_hint_accounts_for_the_icon", () => {
    const dir = tempdir();
    const src = (size: string) =>
      `${HEAD}box a "A rather long label" at (0,0) size (${size}) icon="server"\n`;
    const [, d] = run(dir, src("100,40"));
    expect(codes(d)).toEqual(["label-overflow"]);
    const help = d[0]!.help[0]!;
    const size = help.split("size (")[1]!.split(")")[0]!;
    const [, fixed] = run(dir, src(size));
    expect(fixed, `(${size}): ${JSON.stringify(fixed)}`).toEqual([]);
    const [, d2] = run(dir, `${HEAD}box a "A rather long label" at (0,0) size (100,40)\n`);
    const plain = d2[0]!.help[0]!.split("size (")[1]!.split(",")[0]!;
    const withIcon = parseFloat(size.split(",")[0]!);
    expect(withIcon).toBe(parseFloat(plain) + 32.0);
  });

  test("images_are_anchorable_and_pinnable", () => {
    const dir = tempdir();
    write(dir, "logo.png", png(10, 10));
    const [s, d] = run(
      dir,
      `${HEAD}image i "logo.png" at (100,100) size (40,20) pin=center\n` + `line l i.top -> v -5\n`,
    );
    expect(d, JSON.stringify(d)).toEqual([]);
    expect(shape(s, "i").rect).toEqual(new Rect(80.0, 90.0, 40.0, 20.0));
    expect(start(s, "l")).toEqual(new Pt(100.0, 90.0));
  });
});
