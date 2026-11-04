import { describe, expect, it } from "vitest";
import type {
  BlockText,
  File,
  Line,
  Point,
  SegKind,
  Size,
  Stmt,
} from "../../src/syntax/index.js";
import { offsetDelta, parse, pointUngrouped } from "../../src/syntax/index.js";

function parseOk(src: string): File {
  const { file, diags } = parse(src);
  expect(diags.map((d) => d.message), "unexpected diagnostics").toEqual([]);
  return file;
}

function sized(w: number, h: number): Size | null {
  return { w, h };
}

type StmtValue = { [S in Stmt as S["kind"]]: S["value"] };

function as<K extends Stmt["kind"]>(s: Stmt | undefined, kind: K): StmtValue[K] {
  if (s === undefined || s.kind !== kind) throw new Error(`expected ${kind}, got ${s?.kind}`);
  return s.value as StmtValue[K];
}

function point<K extends Point["kind"]>(p: Point, kind: K): Extract<Point, { kind: K }> {
  if (p.kind !== kind) throw new Error(`expected ${kind}, got ${p.kind}`);
  return p as Extract<Point, { kind: K }>;
}

function litNums(p: Point): [number, number] {
  const l = point(p, "Lit");
  if (l.x.kind !== "Num" || l.y.kind !== "Num") throw new Error("expected literal point");
  return [l.x.value, l.y.value];
}

function helps(src: string): [string, string][] {
  return parse(src).diags.map((d) => [d.message, d.help.join("; ")]);
}

function block(f: File, i: number): BlockText[] {
  const b = as(f.stmts[i], "Box");
  if (b.block === null) throw new Error("text block");
  return b.block;
}

function lineStmt(f: File): Line {
  return as(f.stmts[0], "Line");
}

function boxAt(src: string): Point {
  const f = parseOk(src);
  return as(f.stmts[0], "Box").at;
}

function nestedGroups(depth: number, close: boolean): string {
  let s = "";
  for (let i = 0; i < depth; i++) {
    s += `group g${i} at (0,0) size (1,1) {\n`;
  }
  if (close) {
    s += "}\n".repeat(depth);
  }
  return s;
}

describe("parse", () => {
  it("parses_diagram_and_box", () => {
    const f = parseOk(
      'diagram "T" size (400,300) grid=10\nbox api "API" at (10,20) size (120,60) style=service\n',
    );
    expect(f.stmts.length).toBe(2);
    const b = as(f.stmts[1], "Box");
    expect(b.id.name).toBe("api");
    expect(b.label).toBe("API");
    expect(b.size).toEqual(sized(120.0, 60.0));
    expect(b.attrs[0]!.key.name).toBe("style");
    expect(b.attrs[0]!.value).toEqual({ kind: "Word", value: "service" });
    expect(litNums(b.at)).toEqual([10.0, 20.0]);
  });

  it("parses_every_segment_kind", () => {
    const f = parseOk(
      "line l1 a.right -> h 10 -> v -20 -> x 50 -> y b.top -> hv c.left@0.25 -> vh (1,2) " +
        "-> vhv g.top -> hvh@0.25 (3,4) -> curve left 30 d.top -> curve via (0,0) (5,5) e.bottom -> f.left+(0,-4) end=arrow ortho\n",
    );
    const l = lineStmt(f);
    expect(l.id!.name).toBe("l1");
    const names: Record<SegKind["kind"], string> = {
      To: "to",
      H: "h",
      V: "v",
      X: "x",
      Y: "y",
      Hv: "hv",
      Vh: "vh",
      Vhv: "vhv",
      Hvh: "hvh",
      Curve: "curve",
      CurveVia: "via",
    };
    expect(l.segs.map((s) => names[s.kind.kind])).toEqual([
      "h",
      "v",
      "x",
      "y",
      "hv",
      "vh",
      "vhv",
      "hvh",
      "curve",
      "via",
      "to",
    ]);
    expect(l.segs[1]!.kind).toEqual({ kind: "V", value: -20.0 });
    const s6 = l.segs[6]!.kind;
    expect(s6.kind === "Vhv" && s6.t === null).toBe(true);
    const s7 = l.segs[7]!.kind;
    expect(s7.kind === "Hvh" && s7.t === 0.25).toBe(true);
    const s8 = l.segs[8]!.kind;
    expect(s8.kind === "Curve" && s8.side === "Left" && s8.amount === 30.0).toBe(true);
    expect(l.attrs.length).toBe(2);
    expect(l.attrs[1]!.value).toEqual({ kind: "Flag" });
  });

  it("parses_mixed_coordinates_and_offsets", () => {
    const f = parseOk('junction j at (db.left.x, lane.y)\ntext "hi" at db.top@0.5+(0,-10)\n');
    const j = as(f.stmts[0], "Junction");
    const lit = point(j.at, "Lit");
    if (lit.x.kind !== "Axis" || lit.x.axis !== "X" || lit.y.kind !== "Axis" || lit.y.axis !== "Y") {
      throw new Error(JSON.stringify(j.at));
    }
    expect(lit.x.r.anchor!.name.name).toBe("left");
    expect(lit.y.r.id.name).toBe("lane");
    expect(lit.y.r.anchor).toBeNull();
    const t = as(f.stmts[1], "Text");
    const r = point(t.at, "Ref");
    expect(r.r.anchor!.t).toBe(0.5);
    expect(r.offset).toEqual({ neg: false, dx: 0.0, dy: -10.0 });
  });

  it("continues_lines_starting_with_arrow_or_attribute", () => {
    const f = parseOk(
      'line a.top\n  -> y 40\n  -> hv b.left\n  end=arrow label="SQL"\nbox c at (0,0) size (1,1)\n',
    );
    expect(f.stmts.length).toBe(2);
    const l = lineStmt(f);
    expect(l.segs.length).toBe(2);
    expect(l.attrs.length).toBe(2);
  });

  it("parses_groups_styles_comments_and_allow_lists", () => {
    const f = parseOk(
      "// header\nstyle svc { fill=#e8f0fe\n stroke=#1a73e8 }\n" +
        'group vpc "VPC" at (0,0) size (100,100) {\n  box a at (1,1) size (10,10) // trailing\n}\n' +
        'line (0,0) -> h 5 allow=cross:l2,near label@0.3="x"\n',
    );
    expect(f.stmts.length).toBe(3);
    const s = as(f.stmts[0], "Style");
    expect(s.attrs[0]!.value).toEqual({ kind: "Color", value: "#e8f0fe" });
    const g = as(f.stmts[1], "Group");
    expect(g.children.length).toBe(1);
    const l = as(f.stmts[2], "Line");
    expect(l.attrs[0]!.value).toEqual({ kind: "Word", value: "cross:l2,near" });
    expect(l.attrs[1]!.at_t).toBe(0.3);
  });

  it("recovers_and_reports_multiple_errors", () => {
    const { file: f, diags } = parse(
      "bx a at (0,0) size (1,1)\nbox b at (0,0) size (1,1)\nline c.top ->\nbox d at (0,0) size (1,1)\n",
    );
    expect(diags.length).toBe(2);
    expect(diags[0]!.help.some((h) => h.includes("did you mean `box`?"))).toBe(true);
    expect(diags[1]!.message).toContain("must be followed by a segment");
    expect(f.stmts.length).toBe(2);
  });

  it("unclosed_group_is_reported", () => {
    const { diags } = parse("group g at (0,0) size (1,1) {\n box a at (0,0) size (1,1)\n");
    expect(diags.some((d) => d.message.includes("expected `}`"))).toBe(true);
  });

  it("failed_statements_record_their_ids", () => {
    const { file: f, diags } = parse(
      'diagram "t" size (1\nbox a at (0,0) size (1\njunction j at\ntext t "x" at\n' +
        'line l a.right ->\nimage i "x.png" at\nbox at (0,0) size (1,1)\nbox h at (0,0)\n' +
        "group g at (0,0) size (9,9) {\n  box inner at\n  box ok at (0,0) size (1,1)\n} junk\n" +
        "box fine at (0,0) size (1,1)\n",
    );
    expect(diags.length).toBeGreaterThan(0);
    expect(f.failed_ids.map((i) => i.name)).toEqual([
      "a",
      "j",
      "t",
      "l",
      "i",
      "at",
      "inner",
      "g",
      "ok",
    ]);
    expect(f.failed_diagram).toBe(true);
    expect(f.stmts.length).toBe(1);
    const { file: f2 } = parse('diagram "t" size (1,1)\nbox a at\n');
    expect(f2.failed_diagram).toBe(false);
  });

  it("block_comment_between_statements", () => {
    const f = parseOk(
      'box a "A" at (0,0) size (10,10)\n/* spans\nlines */\nbox b "B" at (20,0) size (10,10)\n',
    );
    expect(f.stmts.length).toBe(2);
  });

  it("identifiers_allow_underscores_digits_and_hyphens", () => {
    const f = parseOk(
      "box _a1 at (0,0) size (1,1)\nbox a--b at (0,0) size (1,1)\nbox c- at (0,0) size (1,1)\nline a--b.right->c-.left\n",
    );
    const ids = f.stmts.flatMap((s) => (s.kind === "Box" ? [s.value.id.name] : []));
    expect(ids).toEqual(["_a1", "a--b", "c-"]);
    const l = as(f.stmts[3], "Line");
    expect(l.segs.length).toBe(1);
  });

  it("numbers_are_integers_or_decimals_optionally_negative", () => {
    const f = parseOk("box a at (-1.5,20) size (0.25,3)\n");
    const b = as(f.stmts[0], "Box");
    expect([...litNums(b.at), b.size]).toEqual([-1.5, 20.0, sized(0.25, 3.0)]);
  });

  it("colors_take_three_six_or_eight_hex_digits", () => {
    const f = parseOk("style s { fill=#abc stroke=#a1b2c3 color=#a1b2c3d4 }\n");
    const s = as(f.stmts[0], "Style");
    expect(s.attrs[2]!.value).toEqual({ kind: "Color", value: "#a1b2c3d4" });
    for (const bad of ["#ab", "#abcd", "#ggg"]) {
      const { diags: d } = parse(`style s { fill=${bad} }\n`);
      expect(d[0]!.code, bad).toBe("parse");
    }
  });

  it("statements_end_at_a_newline", () => {
    const { diags: d } = parse("box a at (0,0) size (1,1) box b at (0,0) size (1,1)\n");
    expect(d.length).toBe(1);
    expect(d[0]!.code).toBe("parse");
    const { diags: d2 } = parse("box a at (0,0)\n  size (1,1)\n");
    expect(d2.length).toBeGreaterThan(0);
  });

  it("arrow_continuation_skips_blank_and_comment_lines", () => {
    const f = parseOk(
      "line a.top\n\n  // first hop\n  -> y 40\n  /* second\n     hop */\n  -> h 10 // trailing\n\n  end=arrow\nbox c at (0,0) size (1,1)\n",
    );
    expect(f.stmts.length).toBe(2);
    const l = lineStmt(f);
    expect([l.segs.length, l.attrs.length]).toEqual([2, 1]);
  });

  it("a_new_statement_ends_a_line_without_arrow", () => {
    const f = parseOk("line (0,0) -> h 5\nline (0,0)\n  -> v 5\n");
    expect(f.stmts.length).toBe(2);
  });

  it("block_comment_spanning_lines_never_joins_statements", () => {
    const { diags: d } = parse("box a at (0,0) /* x\n */ size (1,1)\n");
    expect(d.length).toBeGreaterThan(0);
    expect(d[0]!.code).toBe("parse");
  });

  it("unclosed_block_comment_is_a_parse_error", () => {
    const { diags: d } = parse("box a at (0,0) size (1,1)\n/* never closed\n");
    expect(d.length).toBe(1);
    expect(d[0]!.code).toBe("parse");
  });

  it("line_attributes_parse", () => {
    const f = parseOk(
      'line l (0,0) -> h 5 start=dot end=arrow stroke=#000 stroke-width=2 stroke-style=dashed style=s radius=4 label="a" label@0.25="b" ortho allow=cross reason="r"\n',
    );
    expect(lineStmt(f).attrs.length).toBe(12);
  });

  it("text_parses_with_and_without_id", () => {
    const f = parseOk('text t "a" at (0,0) size=12 align=left\ntext "b" at (0,0)\n');
    const ids = f.stmts.map((s) => {
      const t = as(s, "Text");
      return t.id === null ? null : t.id.name;
    });
    expect(ids).toEqual(["t", null]);
  });

  it("junction_with_dot_flag", () => {
    const f = parseOk("junction j at (0,0) dot\n");
    expect(as(f.stmts[0], "Junction").attrs[0]!.value).toEqual({ kind: "Flag" });
  });

  it("line_needs_a_segment", () => {
    const { diags: d } = parse("line (0,0)\n");
    expect(d[0]!.code).toBe("parse");
    expect(d[0]!.message).toContain("`->`");
  });

  it("nested_groups_parse", () => {
    const f = parseOk(
      'group a at (0,0) size (9,9) {\n  group b at (0,0) size (5,5) {\n    junction j at (0,0)\n    text "t" at (0,0)\n    line (0,0) -> h 1\n  }\n}\n',
    );
    const a = as(f.stmts[0], "Group");
    const b = as(a.children[0], "Group");
    expect(b.children.length).toBe(3);
  });

  it("parses_imports_shapes_and_images", () => {
    const f = parseOk(
      'import "lib/aws.kash" as aws\nimport "base.kash"\n' +
        'shape lb {\n  path "M 0 0 L 1 1" // diagonal\n  outline 1\n  text-area (0.2,0.3,0.6,0.4)\n}\n' +
        'shape s3 from "icons/s3.svg" outline=hull text-area=below\n' +
        'image logo "hf.png" at (20,20) size (80,80) fit=contain\n' +
        "box b at (0,0) size (1,1) shape=aws.s3 style=aws.service\n",
    );
    const i0 = as(f.stmts[0], "Import");
    expect(i0.path === "lib/aws.kash" && i0.ns!.name === "aws").toBe(true);
    expect(as(f.stmts[1], "Import").ns).toBeNull();
    const s = as(f.stmts[2], "Shape");
    expect(s.name.name).toBe("lb");
    if (s.body.kind !== "Paths") throw new Error("expected a path body");
    expect(s.body.path[0]).toBe("M 0 0 L 1 1");
    expect(s.body.outline?.[0]).toBe(1.0);
    expect(s.body.text_area?.[0]).toEqual([0.2, 0.3, 0.6, 0.4]);
    const s3 = as(f.stmts[3], "Shape");
    expect(
      s3.body.kind === "From" && s3.body.file === "icons/s3.svg" && s3.body.attrs.length === 2,
    ).toBe(true);
    const img = as(f.stmts[4], "Image");
    expect(img.id.name).toBe("logo");
    expect(img.file).toBe("hf.png");
    expect(img.size).toEqual([80.0, 80.0]);
    expect(img.attrs.length).toBe(1);
    const b = as(f.stmts[5], "Box");
    expect(b.attrs[0]!.value).toEqual({ kind: "Word", value: "aws.s3" });
    expect(b.attrs[1]!.value).toEqual({ kind: "Word", value: "aws.service" });
  });

  it("shape_body_errors_recover", () => {
    const { file: f, diags: d } = parse(
      'shape a {\n  pth "M 0 0"\n}\nshape b {\n  outline 1\n}\nshape c {\n  path "M 0 0"\n  path "M 1 1"\n}\nbox z at (0,0) size (1,1)\n',
    );
    expect(d.map((x) => [x.code, x.span.line])).toEqual([
      ["parse", 2],
      ["parse", 4],
      ["parse", 9],
    ]);
    expect(d[0]!.help).toEqual(["did you mean `path`?"]);
    expect(d[1]!.message).toContain("needs a `path`");
    expect(f.stmts[f.stmts.length - 1]!.kind).toBe("Box");
    expect(as(f.stmts[0], "Shape").name.name).toBe("c");
  });

  it("image_needs_a_file_string", () => {
    const { diags: d } = parse("image logo at (0,0) size (1,1)\n");
    expect(d[0]!.code).toBe("parse");
    expect(d[0]!.message).toContain("a file path string");
  });

  it("image_inside_a_group_parses", () => {
    const { file: f, diags } = parse(
      'group g at (0,0) size (9,9) {\n  image i "x.png" at (0,0) size (1,1)\n  box a at (0,0) size (1,1) { text "x" }\n  box b at (0,0) size (1,1)\n}\nbox c at (0,0) size (1,1)\n',
    );
    expect(diags).toEqual([]);
    const g = as(f.stmts[0], "Group");
    expect(g.children.length).toBe(3);
    expect(g.children[0]!.kind).toBe("Image");
    expect(f.stmts.length).toBe(2);
  });

  it("unclosed_shape_block_is_reported", () => {
    const { file: f, diags } = parse('shape s {\n  path "M 0 0"\n');
    expect(diags.length).toBe(1);
    expect(diags[0]!.code).toBe("parse");
    expect(diags[0]!.message).toContain("expected `}`");
    expect(f.stmts).toEqual([]);
  });

  it("dotted_words_need_an_identifier_after_the_dot", () => {
    const { diags: d } = parse("box b at (0,0) size (1,1) style=aws.\n");
    expect(d.length).toBe(1);
    expect(d[0]!.code).toBe("parse");
  });

  it("pin_with_position_parses", () => {
    const f = parseOk("box a at (0,0) size (1,1) pin=left@0.25 style=service\n");
    const n = as(f.stmts[0], "Box");
    expect(n.attrs[0]!.value).toEqual({ kind: "AnchorAt", value: ["left", 0.25] });
    expect(n.attrs.length).toBe(2);
  });

  it("flag_lines_continue_a_line", () => {
    const f = parseOk(
      'line a.top\n  -> y 40\n  ortho\nline (0,0) -> h 5\n\n  ortho end=arrow\n  label="x"\nbox c at (0,0) size (1,1)\n',
    );
    expect(f.stmts.length).toBe(3);
    expect(as(f.stmts[0], "Line").attrs[0]!.key.name).toBe("ortho");
    const l = as(f.stmts[1], "Line");
    expect(l.attrs.map((a) => a.key.name)).toEqual(["ortho", "end", "label"]);
  });

  it("flag_lines_do_not_continue_other_statements", () => {
    const { diags } = parse("box a at (0,0) size (1,1)\n  ortho\n");
    expect(diags.length).toBe(1);
    expect(diags[0]!.code).toBe("parse");
  });

  it("group_nesting_is_limited", () => {
    parseOk(nestedGroups(64, true));
    for (const close of [true, false]) {
      const { file: f, diags } = parse(nestedGroups(20_000, close));
      expect(
        diags.some((d) => d.code === "parse" && d.message.includes("nested more than 64")),
      ).toBe(true);
      expect(f.stmts.length).toBe(1);
    }
    const { file: f, diags } = parse(`${nestedGroups(70, true)}box after at (0,0) size (1,1)\n`);
    expect(diags.length).toBe(1);
    expect(f.stmts.length).toBe(2);
  });

  it("unknown_string_escapes_are_parse_errors", () => {
    const { diags: d } = parse('text "a\\qb" at (0,0)\n');
    expect(d.length).toBe(1);
    expect(d[0]!.code).toBe("parse");
    expect(d[0]!.message).toContain("`\\q`");
    parseOk('text "\\" \\\\ \\n \\* \\`" at (0,0)\n');
  });

  it("leading_byte_order_mark_is_skipped", () => {
    const f = parseOk('\u{feff}diagram "t" size (10,10)\n');
    expect(f.stmts.length).toBe(1);
  });

  it("segment_keywords_are_reserved_ids", () => {
    for (const kw of ["h", "v", "x", "y", "hv", "vh", "curve"]) {
      for (const src of [
        `box ${kw} at (0,0) size (1,1)\n`,
        `group ${kw} at (0,0) size (1,1) {\n}\n`,
        `junction ${kw} at (0,0)\n`,
        `text ${kw} "t" at (0,0)\n`,
        `line ${kw} (0,0) -> h 1\n`,
      ]) {
        const { diags: d } = parse(src);
        expect(d.length, src).toBe(1);
        expect(d[0]!.code).toBe("parse");
        expect(d[0]!.message).toContain("reserved");
        expect(d[0]!.help.length).toBeGreaterThan(0);
      }
    }
    parseOk("box hx at (0,0) size (1,1)\njunction curves at (0,0)\n");
  });

  it("box_text_block_parses_stacked_and_absolute_items", () => {
    const f = parseOk(
      'box api at (100,80) size (200,120) text-align=left {\n  text "**Payments API**"\n  text "v2.4.1" size=11 muted\n\n  text "note" at (10,90) align=right\n}\nbox b at (0,0) size (1,1)\n',
    );
    expect(f.stmts.length).toBe(2);
    const items = block(f, 0);
    expect(items.length).toBe(3);
    expect(items[0]!.content).toBe("**Payments API**");
    expect(items[0]!.at).toBeNull();
    expect(items[1]!.attrs.length).toBe(2);
    expect(items[1]!.attrs[1]!.value).toEqual({ kind: "Flag" });
    expect(items[2]!.at).not.toBeNull();
    expect(items[2]!.span.line).toBe(5);
  });

  it("one_line_and_empty_text_blocks", () => {
    const f = parseOk(
      'box a at (0,0) size (9,9) { text "x" }\nbox b at (0,0) size (9,9) {\n}\nbox c at (0,0) size (9,9)\n',
    );
    expect(block(f, 0).length).toBe(1);
    expect(block(f, 1)).toEqual([]);
    expect(as(f.stmts[2], "Box").block).toBeNull();
  });

  it("text_block_rejects_ids_and_other_statements", () => {
    const { file: f, diags } = parse(
      'box a at (0,0) size (9,9) {\n  text t "x"\n  box b at (0,0) size (1,1)\n  text "ok"\n}\nbox c at (0,0) size (1,1)\n',
    );
    expect(diags.map((d) => d.message)).toEqual([
      "text block items take no id",
      "expected `text` or `}` in a text block, found `box`",
    ]);
    expect(block(f, 0).length).toBe(1);
    expect(f.stmts.length).toBe(2);
  });

  it("unclosed_text_block_is_reported", () => {
    const { diags } = parse('box a at (0,0) size (9,9) {\n  text "x"\n');
    expect(diags.length).toBe(1);
    expect(diags[0]!.message).toBe("expected `}`, found end of file");
  });

  it("reserved_id_statements_are_dropped", () => {
    const { file: f, diags: d } = parse(
      'box h at (0,0) size (1,1)\ngroup v at (0,0) size (9,9) {\n  box inner at (1,1) size (1,1)\n}\njunction x at (0,0)\ntext y "t" at (0,0)\nline hv (0,0) -> h 1\nimage vh "a.png" at (0,0) size (1,1)\nbox ok at (0,0) size (1,1)\n',
    );
    expect(d.length).toBe(6);
    expect(f.stmts.length).toBe(1);
    expect(as(f.stmts[0], "Box").id.name).toBe("ok");
  });

  it("hash_comment_is_one_error_with_help", () => {
    for (const src of [
      "# a note\nbox a at (0,0) size (1,1)\n",
      "#note\nbox a at (0,0) size (1,1)\n",
      "  # indented\nbox a at (0,0) size (1,1)\n",
    ]) {
      const { file: f, diags: d } = parse(src);
      expect(d.length, src).toBe(1);
      expect(d[0]!.code).toBe("parse");
      expect(d[0]!.message).toContain("comment");
      expect(d[0]!.help.some((h) => h.includes("`//`"))).toBe(true);
      expect(f.stmts.length).toBe(1);
    }
    const { diags: d } = parse("box a at (0,0) size (1,1) # trailing\n");
    expect(d.length).toBe(1);
    expect(d[0]!.help.some((h) => h.includes("`//`"))).toBe(true);
  });

  it("at_size_and_attributes_come_in_any_order", () => {
    const f = parseOk(
      'diagram "t" grid=10 size (400,300) clearance=4\n' +
        'box a "A" sublabel="v2" at (20,20) size (100,60)\n' +
        "box b size (10,20) style=service at (1,2) pin=tl\n" +
        'group g "G" size (90,90) at (0,0) {\n}\n' +
        "junction j dot at (5,5)\n" +
        'text "t" size=12 muted at (3,4)\n' +
        'image i "a.png" fit=cover size (8,8) at (0,0)\n' +
        'box c at (0,0) size (9,9) {\n  text "x" muted at (1,1)\n}\n',
    );
    expect(f.stmts.length).toBe(8);
    const d = as(f.stmts[0], "Diagram");
    expect(d.size).toEqual(sized(400.0, 300.0));
    expect(d.attrs.length).toBe(2);
    const b = as(f.stmts[2], "Box");
    expect(b.size).toEqual(sized(10.0, 20.0));
    expect(b.attrs.length).toBe(2);
    const lit = point(b.at, "Lit");
    expect(lit.x).toEqual({ kind: "Num", value: 1.0 });
    expect(as(f.stmts[4], "Junction").attrs[0]!.value).toEqual({ kind: "Flag" });
    expect(as(f.stmts[5], "Text").attrs.length).toBe(2);
    expect(block(f, 7)[0]!.at).not.toBeNull();
  });

  it("duplicate_or_missing_at_and_size_are_parse_errors", () => {
    for (const [src, msg] of [
      ["box a at (0,0) size (1,1) at (2,2)\n", "duplicate `at`"],
      ["box a size (1,1) at (0,0) size (2,2)\n", "duplicate `size`"],
      ['diagram "t" size (1,1) size (2,2)\n', "duplicate `size`"],
      ["junction j at (0,0) at (1,1)\n", "duplicate `at`"],
      ["box a fill=red size (1,1)\n", "expected `at`"],
      ['image i "x.png" at (0,0) fit=cover\n', "expected `size`"],
    ] as const) {
      const { file: f, diags: d } = parse(src);
      expect(d.length, src).toBe(1);
      expect(d[0]!.code).toBe("parse");
      expect(d[0]!.message, src).toContain(msg);
      expect(f.stmts, src).toEqual([]);
    }
  });

  it("box_size_is_optional_and_takes_auto", () => {
    const f = parseOk(
      'box a "A" at (0,0)\nbox b "B" at (0,0) size (auto, 40)\nbox c "C" size (auto,auto) at (0,0)\n',
    );
    const sizes = f.stmts.map((s) => as(s, "Box").size);
    expect(sizes).toEqual([null, { w: null, h: 40.0 }, { w: null, h: null }]);
  });

  it("auto_size_is_for_boxes_groups_and_the_diagram", () => {
    for (const [src, size] of [
      ['diagram "t"\n', null],
      ['diagram "t" size (auto,100)\n', { w: null, h: 100.0 }],
    ] as const) {
      const d = as(parseOk(src).stmts[0], "Diagram");
      expect(d.size).toEqual(size);
    }
    const { file: f, diags: d } = parse('image i "x.png" at (0,0) size (auto,10)\n');
    expect(d.length).toBe(1);
    expect(d[0]!.code).toBe("parse");
    expect(f.stmts).toEqual([]);
    const g = as(parseOk("group g at (0,0) size (10,auto) {\n}\n").stmts[0], "Group");
    expect(g.node.size).toEqual({ w: 10.0, h: null });
    const g2 = as(parseOk("group g at (0,0) style=wide {\n}\n").stmts[0], "Group");
    expect(g2.node.size).toBeNull();
  });

  it("size_attribute_takes_a_size", () => {
    const f = parseOk('style s { size=(120, auto) font-size=12 }\ntext "t" at (0,0) size=12\n');
    const s = as(f.stmts[0], "Style");
    expect(s.attrs[0]!.value).toEqual({ kind: "Size", value: { w: 120.0, h: null } });
    const t = as(f.stmts[1], "Text");
    expect(t.attrs[0]!.value).toEqual({ kind: "Num", value: 12.0 });
  });

  it("arrow_is_a_line_statement", () => {
    const f = parseOk('arrow ab a.right -> b.left label="x"\narrow a.bottom -> v 20 end=none\n');
    const lines = f.stmts.map((s) => as(s, "Line"));
    expect(lines.every((l) => l.arrow)).toBe(true);
    expect(lines[0]!.id?.name).toBe("ab");
    expect(lines[1]!.id).toBeNull();
    expect(lines[1]!.attrs[0]!.key.name).toBe("end");
    expect(lineStmt(parseOk("line a.right -> b.left\n")).arrow).toBe(false);
  });

  it("arrow_is_reserved", () => {
    expect(helps("box arrow at (0,0) size (1,1)\n")).toEqual([
      [
        "`arrow` is a reserved word and cannot be an id",
        "`arrow` is a statement keyword; pick another name, e.g. `arrow_1`",
      ],
    ]);
    const h = helps("arrow ab: a.right -> b.left\n");
    expect(h[0]![1]).toBe("remove the `:`: write `arrow ab a.right -> b.left`");
  });

  it("line_id_is_an_identifier_before_a_point", () => {
    const f = parseOk(
      "line l1 a.right -> b.left\nline l2 (0,0) -> h 10\nline a -> b\nline a.top -> b\nline (0,0) -> b\n",
    );
    const ids = f.stmts.map((s) => as(s, "Line").id?.name ?? null);
    expect(ids).toEqual(["l1", "l2", null, null, null]);
  });

  it("offsets_keep_their_sign", () => {
    for (const [src, neg, dx, delta] of [
      ["line a.top+(-10,0) -> h 1\n", false, -10.0, [-10.0, 0.0]],
      ["line a.top-(10,0) -> h 1\n", true, 10.0, [-10.0, 0.0]],
      ["line a.top-(-10,0) -> h 1\n", true, -10.0, [10.0, 0.0]],
      ["line a-b.left@0.25-(0,4) -> h 1\n", true, 0.0, [0.0, -4.0]],
    ] as const) {
      const f = parseOk(src);
      const o = point(lineStmt(f).start, "Ref").offset!;
      expect([o.neg, o.dx], src).toEqual([neg, dx]);
      expect(offsetDelta(o).map((v) => v + 0), src).toEqual(delta.map((v) => v + 0));
    }
  });

  it("old_line_id_colon_is_an_error_with_help", () => {
    const { file: f, diags: d } = parse("line l1: a.right -> b.left end=arrow\n");
    expect(d.length).toBe(1);
    expect(d[0]!.message).toBe("a line id takes no `:`");
    expect(d[0]!.help).toEqual(["remove the `:`: write `line l1 a.right -> b.left end=arrow`"]);
    expect(lineStmt(f).id!.name).toBe("l1");
  });

  it("from_to_lines_are_errors_with_help", () => {
    const want = "write `line l1 a.right -> b.left`";
    for (const src of ["line l1 from a.right to b.left\n", "line l1: from a.right to b.left\n"]) {
      expect(helps(src), src).toEqual([["a line is written with `->`, not `from … to`", want]]);
    }
    expect(helps("line from a.right to (1,2)\n")[0]![1]).toBe("write `line a.right -> (1,2)`");
  });

  it("old_curve_colon_is_an_error_with_help", () => {
    const { file: f, diags: d } = parse("line a.right -> curve left:30 b.top+(0,-4) -> h 10\n");
    expect(d.length).toBe(1);
    expect(d[0]!.help).toEqual(["remove the `:`: write `curve left 30 b.top+(0,-4)`"]);
    const k = lineStmt(f).segs[0]!.kind;
    expect(k.kind === "Curve" && k.side === "Left" && k.amount === 30.0).toBe(true);
  });

  it("kashika_version_statement_is_an_error_with_help", () => {
    const { file: f, diags: d } = parse('kashika="0.1.0"\ndiagram "t" size (10,10)\n');
    expect(d.length).toBe(1);
    expect(d[0]!.help).toEqual([
      'the version is an attribute of `diagram`: write `diagram "…" size (w,h) kashika="0.1.0"`',
    ]);
    expect(f.first_stmt?.line).toBe(2);
  });

  it("bom_does_not_shift_help_text", () => {
    const { diags } = parse('\u{feff}kashika=="0.1"\n');
    expect(diags.length).toBe(1);
    const { diags: plain } = parse('kashika=="0.1"\n');
    expect(diags[0]!.help).toEqual(plain[0]!.help);
  });

  it("parenthesized_points_mean_the_bare_point", () => {
    for (const [grouped, bare] of [
      ["(a.center)", "a.center"],
      ["(a.center+(0,40))", "a.center+(0,40)"],
      ["(db.top-(10,0))", "db.top-(10,0)"],
      ["(a.left@0.25+(4,0))", "a.left@0.25+(4,0)"],
      ["(fan)", "fan"],
      ["(fan+(0,10))", "fan+(0,10)"],
    ]) {
      const g = boxAt(`box b at ${grouped} size (1,1)\n`);
      const b = boxAt(`box b at  ${bare} size (1,1)\n`);
      expect(g.kind, grouped).toBe("Group");
      const strip = (p: Point) => {
        switch (p.kind) {
          case "Lit":
            return [p.x, p.y];
          case "Ref":
            return [p.r, p.offset];
          default:
            throw new Error("still grouped");
        }
      };
      expect(strip(pointUngrouped(g)), grouped).toEqual(strip(b));
    }
  });

  it("parentheses_nest", () => {
    const g = point(boxAt("box b at (a.center) size (1,1)\n"), "Group");
    expect([g.span.col, g.span.endCol]).toEqual([10, 20]);
    expect(g.inner.kind).toBe("Ref");
    for (const src of [
      "((a.center))",
      "(((a.center)))",
      "((a.center+(0,4)))",
      "((10,20))",
      "(( 10,20 ))",
      "((a.left.x, b.y))",
      "((a.left.x, b.y)+(0,4))",
      "((a.center)+(0,4))",
    ]) {
      expect(boxAt(`box b at ${src} size (1,1)\n`).kind, src).toBe("Group");
    }
    const g2 = point(boxAt("box b at ((a.left.x, b.y)+(0,4)) size (1,1)\n"), "Group");
    const s = point(g2.inner, "Shifted");
    expect(s.base.kind).toBe("Lit");
    expect([s.offsets[0]!.dx, s.offsets[0]!.dy]).toEqual([0.0, 4.0]);
  });

  it("offsets_follow_any_point", () => {
    for (const [src, base, n] of [
      ["(10,20)+(5,0)", "lit", 1],
      ["(a.left.x, b.y)+(0,26)", "lit", 1],
      ["(a.center)+(0,4)", "group", 1],
      ["a.center+(1,2)+(3,4)", "ref", 1],
      ["(a.center+(0,4))+(2,0)", "group", 1],
      ["(a.center)-(1,2)+(3,4)", "group", 2],
    ] as const) {
      const s = point(boxAt(`box b at ${src} size (1,1)\n`), "Shifted");
      const kind = { Lit: "lit", Ref: "ref", Group: "group", Shifted: "shifted" }[s.base.kind];
      expect([kind, s.offsets.length], src).toEqual([base, n]);
    }
    const f = parseOk("line l (a.center)-(0,4) -> (10,20)+(5,0) -> b.top+(1,1)-(2,2)\n");
    const l = lineStmt(f);
    expect(point(l.start, "Shifted").offsets[0]!.neg).toBe(true);
    expect(l.segs.length).toBe(2);
  });

  it("parentheses_nest_at_most_16_deep", () => {
    const at = (n: number) => `box b at ${"(".repeat(n)}a.center${")".repeat(n)} size (1,1)\n`;
    parseOk(at(16));
    parseOk("box b at ((((((((((((((((10,20)))))))))))))))) size (1,1)\n");
    const { diags: d } = parse(at(17));
    expect(d.length).toBe(1);
    expect(d[0]!.message).toBe("a point nests at most 16 sets of parentheses");
    expect([d[0]!.span.col, d[0]!.span.endCol]).toEqual([26, 36]);
    expect(d[0]!.help).toEqual([
      "parentheses around a point change nothing; remove the extra ones",
    ]);
  });

  it("deeply_nested_point_parentheses_are_one_error", () => {
    for (const close of [false, true]) {
      const n = 5000;
      const src = `box b at ${"(".repeat(n)}a.center${close ? ")".repeat(n) : ""} size (1,1)\nbox c at (0,0) size (1,1)\n`;
      const { file: f, diags: d } = parse(src);
      expect(d.length).toBe(1);
      expect(d[0]!.message).toBe("a point nests at most 16 sets of parentheses");
      expect(f.stmts.length).toBe(1);
    }
  });

  it("parenthesized_points_in_line_endpoints_and_segments", () => {
    const f = parseOk(
      "line l (a.bottom) -> (b.top-(0,4)) -> x (c.left) -> y (lane) -> hv (d.top) " +
        "-> vh (e.left) -> curve left 20 (f.top) -> curve via (g.center) (h.center) (i.right)\n",
    );
    const l = lineStmt(f);
    expect(l.start.kind).toBe("Group");
    const points: Point[] = l.segs.flatMap((s) => {
      const k = s.kind;
      switch (k.kind) {
        case "To":
        case "Hv":
        case "Vh":
          return [k.value];
        case "Curve":
          return [k.to];
        case "X":
        case "Y":
          return k.value.kind === "Point" ? [k.value.value] : [];
        case "CurveVia":
          return [k.c1, k.c2, k.to];
        default:
          return [];
      }
    });
    expect(points.length).toBe(9);
    expect(points.every((p) => p.kind === "Group")).toBe(true);
    const r = point(pointUngrouped(points[1]!), "Ref");
    expect(r.offset).toBeNull();
    expect(r.r.anchor!.name.name).toBe("left");
  });

  it("literal_and_mixed_points_are_not_groups", () => {
    expect(boxAt("box b at (10,20) size (1,1)\n").kind).toBe("Lit");
    const l = point(boxAt("box b at (a.left.x, lane.y) size (1,1)\n"), "Lit");
    expect(l.x.kind === "Axis" && l.x.axis === "X").toBe(true);
    expect(l.y.kind === "Axis" && l.y.axis === "Y").toBe(true);
    const { diags: d } = parse("box b at (a.left, lane.y) size (1,1)\n");
    expect(d.length).toBe(1);
    expect(d[0]!.message).toBe("expected `.`, found `,`");
  });

  it("unclosed_point_paren_is_an_error_with_help", () => {
    let { diags: d } = parse("box b at (a.center+(0,40) size (1,1)\n");
    expect(d.length).toBe(1);
    expect(d[0]!.code).toBe("parse");
    expect(d[0]!.message).toBe("expected `)` to close the `(` around the point, found `size`");
    expect([d[0]!.span.line, d[0]!.span.col]).toEqual([1, 27]);
    expect(d[0]!.help).toEqual(["close it: `(a.center+(0,40))`"]);
    ({ diags: d } = parse("line l (a.center -> b.top\n"));
    expect(d.length).toBe(1);
    expect(d[0]!.message).toBe("expected `)` to close the `(` around the point, found `->`");
    ({ diags: d } = parse("box b at (a.left.x) size (1,1)\n"));
    expect(d.length).toBe(1);
    expect(d[0]!.help).toEqual([
      "`.x`/`.y` take one axis of a mixed point, which needs both: `(a.left.x, other.y)`",
    ]);
  });

  it("elbow_pair_position_is_inside_0_1", () => {
    for (const [src, msg] of [
      ["vhv@0 b.top", "`vhv@t` needs t between 0 and 1 (exclusive), found 0"],
      ["hvh@1.5 b.top", "`hvh@t` needs t between 0 and 1 (exclusive), found 1.5"],
      ["vhv@-0.2 b.top", "`vhv@t` needs t between 0 and 1 (exclusive), found -0.2"],
    ]) {
      const { file: f, diags: d } = parse(`line l a.bottom -> ${src}\n`);
      expect(d.length, src).toBe(1);
      expect([d[0]!.code, d[0]!.message]).toEqual(["parse", msg]);
      expect(f.stmts).toEqual([]);
    }
    const { diags: d } = parse("box vhv at (0,0) size (1,1)\n");
    expect(d[0]!.message).toBe("`vhv` is a reserved word and cannot be an id");
  });

  it("common_slips_get_the_fix_as_help", () => {
    for (const [src, help] of [
      ["box b at (0,0) size 100x50\n", "write `size (100,50)`"],
      ["box b at (0,0) size 100 x 50\n", "write `size (100,50)`"],
      ["box b at (0,0) size 100,50\n", "write `size (100,50)`"],
      ["line l a.right --> b.left\n", "a line hop is `->`, not `-->`"],
      ["line l a.right -> x b.left.x -> b.left\n", "`x` takes a point and uses its x: write `x b.left`"],
      ["line l a.right -> y b.top.y\n", "`y` takes a point and uses its y: write `y b.top`"],
      ["line l a.right -> to b.left\n", "write `-> b.left`"],
      ["line l a.right => b.left\n", "write `->`"],
      ["box b at 20,30 size (1,1)\n", "a literal point is in parentheses: `(20,30)`"],
      ["box b (20,30) size (1,1)\n", "a position starts with `at`: `at (x,y)`"],
    ]) {
      const { diags: d } = parse(src!);
      expect(
        d.some((x) => x.code === "parse" && x.help.length === 1 && x.help[0] === help),
        src,
      ).toBe(true);
    }
  });

  it("never_throws_on_truncated_or_garbled_input", () => {
    const src =
      'diagram "d" size (100,100)\ngroup g at (0,0) size (9,9) {\n  box a "A" at (1,1) { text "x" at (1,1) }\n}\nline l a.right -> curve left:3 (b.top) -> vhv@0.5 c.left end=arrow\nshape s { path "M 0 0" outline 1 text-area (0,0,1,1) }\nimage i "x.png" at (0,0) size (1,1)\n';
    const cs = Array.from(src);
    for (let i = 0; i <= cs.length; i++) {
      const prefix = cs.slice(0, i).join("");
      expect(() => parse(prefix)).not.toThrow();
      expect(() => parse(`${prefix})}->@=,.:#\u{e9}`)).not.toThrow();
    }
  });
});

