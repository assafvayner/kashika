import { describe, expect, test } from "vitest";
import { type Diagnostic } from "../../src/diag/diagnostic.js";
import { Rect } from "../../src/geom/index.js";
import {
  type Font,
  type Scene,
  type Shape,
  type TextRole,
  type TitlePos,
  compile,
} from "../../src/model/index.js";

const HEAD = 'diagram "t" size (800,600)\n';

function ok(body: string): Scene {
  const [scene, diags] = compile(`${HEAD}${body}`);
  expect(diags, JSON.stringify(diags.map((d) => [d.code, d.message]))).toEqual([]);
  return scene;
}

function errors(body: string): Diagnostic[] {
  return compile(`${HEAD}${body}`)[1];
}

function shape(s: Scene, id: string): Shape {
  const x = s.shapes.find((x) => x.id === id);
  if (x === undefined) throw new Error(`no shape ${id}`);
  return x;
}

function close(a: number, b: number): boolean {
  return Math.abs(a - b) < 1e-3;
}

const dump = (v: unknown): string => JSON.stringify(v);

describe("text_shapes", () => {
  test("label_is_centered_in_the_text_area", () => {
    const s = ok('box api "API" at (40,40) size (120,60)\n');
    const b = shape(s, "api");
    expect(b.textArea).toEqual(new Rect(48.0, 48.0, 104.0, 44.0));
    expect(b.text.length).toBe(1);
    const t = b.text[0]!;
    expect(t.role).toBe("label");
    expect([t.size, t.color, t.bold]).toEqual([14.0, "#0f172a", false]);
    expect(close(t.rect.w, 22.3604), dump(t.rect)).toBe(true);
    expect(close(t.rect.x, 100.0 - 22.3604 / 2.0)).toBe(true);
    expect([t.rect.y, t.rect.h]).toEqual([61.25, 17.5]);
    expect(t.lines[0]!.runs[0]!.font).toBe("regular");
    expect(b.titleBand).toBe(0.0);
  });

  test("sublabel_is_a_smaller_muted_line_below", () => {
    const s = ok(
      'box api "Payments API" at (0,0) size (200,120) sublabel="v2.4.1" style=service\n',
    );
    const b = shape(s, "api");
    const [l, sub] = [b.text[0]!, b.text[1]!];
    expect(sub.role).toBe("sublabel");
    expect(close(sub.size, 11.2)).toBe(true);
    expect(sub.color).toBe("#64748b");
    const h = 17.5 + 2.0 + 14.0;
    expect(close(l.rect.y, 8.0 + (104.0 - h) / 2.0)).toBe(true);
    expect(close(sub.rect.y, l.rect.bottom() + 2.0)).toBe(true);
  });

  test("text_align_valign_and_gap", () => {
    let s = ok(
      'box a "A" at (0,0) size (100,100) sublabel="b" text-align=left text-valign=top gap=6\n',
    );
    const b = shape(s, "a");
    expect([b.text[0]!.rect.x, b.text[0]!.rect.y]).toEqual([8.0, 8.0]);
    expect(b.text[1]!.rect.y).toBe(8.0 + 17.5 + 6.0);
    s = ok('box a "A" at (0,0) size (100,100) text-align=right text-valign=bottom\n');
    const r = shape(s, "a").text[0]!.rect;
    expect(close(r.right(), 92.0) && close(r.bottom(), 92.0), dump(r)).toBe(true);
  });

  test("bold_style_and_markup_pick_fonts", () => {
    const s = ok(
      'box a "**A** `b` c" at (0,0) size (200,60)\nbox z "Z" at (0,100) size (60,60) font-weight=bold\n',
    );
    const runs = shape(s, "a").text[0]!.lines[0]!.runs;
    const fonts: Font[] = runs.map((r) => r.font);
    expect(fonts).toEqual(["bold", "regular", "mono", "regular"]);
    const z = shape(s, "z").text[0]!;
    expect(z.bold).toBe(true);
    expect(z.lines[0]!.runs[0]!.font).toBe("bold");
  });

  test("unclosed_markup_in_labels_is_a_parse_error", () => {
    let d = errors('box a "**A" at (0,0) size (200,60)\n');
    expect(d.length, dump(d)).toBe(1);
    expect(d[0]!.code).toBe("parse");
    expect(d[0]!.message).toBe("unclosed `**` in text");
    d = errors('box a at (0,0) size (200,60) sublabel="`v"\n');
    expect(d[0]!.code).toBe("parse");
    expect(d[0]!.span.col).toBe(30);
  });

  test("label_overflow_reports_the_required_size", () => {
    const d = errors('box a "Payments API" at (0,0) size (80,30)\n');
    expect(d.length, dump(d)).toBe(1);
    expect(d[0]!.code).toBe("label-overflow");
    expect(d[0]!.message).toBe(
      "its text needs 91.64x17.5px but the text area of `a` is 64x14px (too small: 27.64px narrower, 3.5px shorter)",
    );
    expect(d[0]!.help).toEqual(["make `a` at least size (108,34), or shorten the text"]);
    expect(errors('box a "Payments API" at (0,0) size (108,34)\n')).toEqual([]);
  });

  test("padding_changes_the_text_area", () => {
    const s = ok('box a "A" at (0,0) size (40,40) padding=2\n');
    expect(shape(s, "a").textArea).toEqual(new Rect(2.0, 2.0, 36.0, 36.0));
  });

  test("padding_pair_insets_each_axis", () => {
    let s = ok('box a "A" at (0,0) size (40,40) padding=(10,3)\n');
    expect(shape(s, "a").textArea).toEqual(new Rect(10.0, 3.0, 20.0, 34.0));
    s = ok('group g "G" at (100,100) size (200,100) padding=(4,10) {\n}\n');
    const g = shape(s, "g");
    expect(g.textArea).toEqual(new Rect(104.0, 110.0, 192.0, 80.0));
    expect(g.text[0]!.rect.x).toBe(104.0);
    expect(close(g.titleBand, 27.5), String(g.titleBand)).toBe(true);
  });

  test("padding_pair_sets_the_overflow_size", () => {
    const d = errors('box a "Payments API" at (0,0) size (50,10) padding=(4,2)\n');
    expect(d[0]!.help).toEqual(["make `a` at least size (100,22), or shorten the text"]);
  });

  test("stacked_block_items", () => {
    const s = ok(
      'box api at (100,80) size (200,120) text-align=left {\n  text "**Payments API**"\n  text "v2.4.1" size=11 muted\n  text "• validates cards" size=12 align=right\n}\n',
    );
    const b = shape(s, "api");
    const roles: TextRole[] = b.text.map((t) => t.role);
    expect(roles).toEqual(["stacked", "stacked", "stacked"]);
    expect(b.text[0]!.rect.x).toBe(108.0);
    expect(b.text[1]!.color).toBe("#64748b");
    expect(b.text[1]!.size).toBe(11.0);
    expect(close(b.text[2]!.rect.right(), 292.0)).toBe(true);
    const h = 17.5 + 13.75 + 15.0 + 2.0 * 2.0;
    expect(close(b.text[0]!.rect.y, 88.0 + (104.0 - h) / 2.0)).toBe(true);
    expect(close(b.text[1]!.rect.y, b.text[0]!.rect.bottom() + 2.0)).toBe(true);
  });

  test("absolute_block_items_use_the_box_frame", () => {
    const s = ok(
      'box a at (100,50) size (200,100) {\n  text "x" at (20,30) align=left\n  text "y" at a.center\n}\n',
    );
    const b = shape(s, "a");
    expect(b.text[0]!.role).toBe("absolute");
    expect(b.text[0]!.rect.x).toBe(120.0);
    expect(b.text[0]!.rect.y).toBe(80.0 - 17.5 / 2.0);
    const c = b.text[1]!.rect.center();
    expect(close(c.x, 200.0) && close(c.y, 100.0), dump(c)).toBe(true);
  });

  test("absolute_items_may_join_a_label", () => {
    const s = ok('box a "A" at (0,0) size (200,100) sublabel="s" {\n  text "x" at (20,80)\n}\n');
    const roles: TextRole[] = shape(s, "a").text.map((t) => t.role);
    expect(roles).toEqual(["label", "sublabel", "absolute"]);
  });

  test("label_with_stacked_items_is_a_text_conflict", () => {
    let d = errors('box a "A" at (0,0) size (200,100) {\n  text "x"\n}\n');
    expect(d.length, dump(d)).toBe(1);
    expect(d[0]!.code).toBe("text-conflict");
    expect(d[0]!.span.line).toBe(3);
    d = errors('box a at (0,0) size (200,100) sublabel="s" {\n  text "x"\n}\n');
    expect(d[0]!.code).toBe("text-conflict");
  });

  test("absolute_item_larger_than_the_text_area_overflows", () => {
    const d = errors('box a at (0,0) size (40,40) {\n  text "Payments API" at (20,20)\n}\n');
    expect(d.length, dump(d)).toBe(1);
    expect(d[0]!.code).toBe("label-overflow");
    expect(d[0]!.span.line).toBe(3);
    expect(d[0]!.message.startsWith("a block item needs 91.64x17.5px"), d[0]!.message).toBe(true);
  });

  test("block_item_attributes_are_checked", () => {
    const d = errors('box a at (0,0) size (200,100) {\n  text "x" weight=bold\n}\n');
    expect(d[0]!.code).toBe("attr");
    expect(d[0]!.message).toBe("unknown attribute `weight` on text block item");
    expect(errors('box a at (0,0) size (200,100) {\n  text "x" size=0\n}\n')[0]!.code).toBe(
      "size",
    );
    expect(
      errors('box a at (0,0) size (200,100) {\n  text "x" at nope.left\n}\n')[0]!.code,
    ).toBe("unknown-id");
  });

  test("text_takes_font_size_and_text_align_or_their_aliases", () => {
    const s = ok(
      'text a "a" at (100,100) font-size=20 text-align=left\n' +
        'text b "a" at (100,200) size=20 align=left\n' +
        'box bx at (300,100) size (200,100) {\n  text "c" font-size=10 text-align=right\n  text "c" size=10 align=right\n}\n',
    );
    const [a, b] = [s.texts[0]!, s.texts[1]!];
    expect([a.size, a.align, a.rect.x, a.rect.w]).toEqual([b.size, b.align, b.rect.x, b.rect.w]);
    expect(a.size).toBe(20.0);
    const x = shape(s, "bx");
    expect([x.text[0]!.size, x.text[1]!.size]).toEqual([10.0, 10.0]);
    expect(x.text[0]!.rect.right()).toBe(x.text[1]!.rect.right());
    let d = errors('text "a" at (0,0) size=12 font-size=14 align=left text-align=right\n');
    expect(d.length, dump(d)).toBe(2);
    expect(d.every((d) => d.code === "attr")).toBe(true);
    expect(d[0]!.message).toBe("duplicate attribute: `size` is another name for `font-size`");
    expect(d[0]!.help).toEqual(["keep one of `size` and `font-size`"]);
    d = errors('box a at (0,0) size (200,100) {\n  text "x" text-align=left align=left\n}\n');
    expect(d[0]!.message).toBe("duplicate attribute: `align` is another name for `text-align`");
  });

  test("wrong_context_names_suggest_the_right_one", () => {
    const help = (src: string): string[] => errors(src)[0]!.help;
    expect(help('box a "A" at (0,0) size (90,40) align=left\n')).toEqual([
      "did you mean `text-align`?",
    ]);
    expect(help('box a "A" at (0,0) size (90,40) size=12\n')).toEqual([
      "did you mean `font-size`?",
    ]);
    expect(help('box a "A" at (0,0) size=(90,40)\n')).toEqual([
      "`size` is a clause, not an attribute: write `size (w,h)`, without `=`",
    ]);
    expect(help('box a "A" at (0,0) size (90,40) valign=top\n')).toEqual([
      "did you mean `text-valign`?",
    ]);
    expect(help("style s { text-size=12 }\n")[0]).toBe("did you mean `font-size`?");
    expect(help('text "t" at (0,0) font-color=red\n')).toEqual(["did you mean `color`?"]);
  });

  test("text_attribute_values_are_checked", () => {
    expect(errors("box a at (0,0) size (9,9) text-align=middle\n")[0]!.code).toBe("attr");
    expect(errors("box a at (0,0) size (9,9) text-valign=center\n")[0]!.code).toBe("attr");
    expect(errors("box a at (0,0) size (9,9) gap=-1\n")[0]!.code).toBe("attr");
    const d = errors(`box a at (0,0) size (9,9) gap=${"9".repeat(300)}\n`);
    expect(d[0]!.code).toBe("attr");
    expect(d[0]!.message).toBe("`gap` must be at most 1000000");
    expect(errors("box a at (0,0) size (9,9) sublabel=3\n")[0]!.code).toBe("attr");
  });

  test("group_title_sits_top_left_and_reserves_a_band", () => {
    let s = ok('group g "VPC" at (10,10) size (300,200) sublabel="10.0.0.0/16" {\n}\n');
    const g = shape(s, "g");
    expect([g.text[0]!.rect.x, g.text[0]!.rect.y]).toEqual([18.0, 18.0]);
    expect(g.titleBand).toBe(8.0 + 17.5 + 2.0 + 14.0);
    s = ok("group g at (0,0) size (300,200) {\n}\n");
    expect(shape(s, "g").titleBand).toBe(0.0);
    s = ok('group g "VPC" at (0,0) size (300,200) text-align=center {\n}\n');
    expect(close(shape(s, "g").text[0]!.rect.center().x, 150.0)).toBe(true);
  });

  test("group_title_bottom_reserves_a_band_at_the_bottom", () => {
    let s = ok(
      'group g "VPC" at (10,10) size (300,200) title=bottom icon="server" text-align=right {\n}\n',
    );
    const g = shape(s, "g");
    expect(g.title).toBe("bottom");
    expect(g.titleBand).toBe(8.0 + 17.5);
    const t = g.text[0]!.rect;
    expect(close(t.bottom(), 202.0) && close(t.right(), 302.0), dump(t)).toBe(true);
    const icon = g.icon!.rect;
    expect(icon.x).toBe(18.0);
    expect(close(icon.center().y, t.center().y)).toBe(true);
    s = ok('group g "VPC" at (0,0) size (300,200) title=top {\n}\n');
    expect(shape(s, "g").title).toBe("top");
  });

  test("title_is_a_style_key_with_inline_precedence", () => {
    const s = ok(
      "style boundary { title=bottom }\nstyle low { title=top }\n" +
        'group a "A" at (0,0) size (100,80) {\n}\n' +
        'group b "B" at (120,0) size (100,80) style=low {\n}\n' +
        'group c "C" at (240,0) size (100,80) style=low title=bottom {\n}\n',
    );
    const titles: TitlePos[] = ["a", "b", "c"].map((id) => shape(s, id).title);
    expect(titles).toEqual(["bottom", "top", "bottom"]);
    const d = errors('style low { title=top }\nbox b "B" at (0,0) size (80,40) style=low\n');
    expect(d[0]!.message).toBe("this style sets `title`, but only groups have a title");
  });

  test("title_is_a_group_attribute", () => {
    let d = errors('box b "B" at (0,0) size (80,40) title=bottom\n');
    expect(d[0]!.code).toBe("attr");
    expect(d[0]!.help).toEqual([
      "`title=` places a group's title; a box places its text with `text-valign`",
    ]);
    d = errors('group g "G" at (0,0) size (300,200) title=left {\n}\n');
    expect(d[0]!.message).toBe("`title` expects one of: top, bottom");
  });

  test("group_rejects_text_valign", () => {
    const d = errors('group g "G" at (0,0) size (300,200) text-valign=top {\n}\n');
    expect(d[0]!.code).toBe("attr");
  });

  test("group_label_overflow", () => {
    const d = errors('group g "A very long group title" at (0,0) size (60,200) {\n}\n');
    expect(d[0]!.code).toBe("label-overflow");
  });

  test("label_escapes_are_literal", () => {
    const s = ok('box a "\\*\\*a\\*\\* \\` b\\\\c" at (0,0) size (200,60) sublabel="`v` \\`"\n');
    const t = shape(s, "a").text;
    expect(t[0]!.lines[0]!.runs.length, dump(t[0]!.lines[0]!.runs)).toBe(1);
    expect(t[0]!.lines[0]!.runs[0]!.text).toBe("**a** ` b\\c");
    const fonts: Font[] = t[1]!.lines[0]!.runs.map((r) => r.font);
    expect(fonts).toEqual(["mono", "regular"]);
    expect(t[1]!.lines[0]!.runs[1]!.text).toBe(" `");
  });

  test("group_label_markup_is_checked", () => {
    const d = errors('group g "`VPC" at (0,0) size (300,200) {\n}\n');
    expect(d.length, dump(d)).toBe(1);
    expect(d[0]!.code).toBe("parse");
  });

  test("label_overflow_names_the_short_dimension", () => {
    let d = errors('box a "Payments API" at (0,0) size (200,30)\n');
    expect(d[0]!.code).toBe("label-overflow");
    expect(d[0]!.message).toBe(
      "its text needs 91.64x17.5px but the text area of `a` is 184x14px (too short by 3.5px)",
    );
    d = errors('box a "Payments API" at (0,0) size (80,60)\n');
    expect(d[0]!.message).toBe(
      "its text needs 91.64x17.5px but the text area of `a` is 64x44px (too narrow by 27.64px)",
    );
  });

  /// The size `label-overflow` asks for when the box is drawn at `size (1,1)`.
  function overflowSize(label: string, attrs: string): [number, number] {
    const d = errors(`box a ${label} at (0,0) size (1,1) ${attrs}\n`);
    const help = d.find((d) => d.code === "label-overflow")!.help[0]!;
    const size = help.split("size (")[1]!.split(")")[0]!;
    const [w, h] = size.split(",") as [string, string];
    return [Number(w), Number(h)];
  }

  test("omitted_size_is_the_size_label_overflow_reports", () => {
    for (const [label, attrs] of [
      ['"Payments API"', ""],
      ['"Payments API"', 'sublabel="v2.4.1" padding=(12,4)'],
      ['"Decision"', "shape=diamond"],
      ['"Orders"', "shape=cylinder"],
      ['"Two\\nlines\\nand three"', "shape=cylinder"],
      ['"Worker pool"', "shape=hexagon"],
      ['"Internet"', "shape=cloud"],
      ['"Server"', 'icon="server"'],
      ['"Server"', 'icon="server" icon-pos=top'],
      ['"**Bold** title"', "font-weight=bold font-size=18"],
      ["", '{\n  text "**Worker**"\n  text "retries charges" size=12\n}'],
    ] as [string, string][]) {
      const want = overflowSize(label, attrs);
      const s = ok(`box a ${label} at (0,0) ${attrs}\n`);
      const r = shape(s, "a").rect;
      expect([r.w, r.h], `${label} ${attrs}`).toEqual(want);
    }
    const s = ok('box a at (0,0) {\n  text "wide absolute item" at (8,8)\n}\n');
    expect(shape(s, "a").rect.w > 100.0).toBe(true);
  });

  test("auto_axes_fit_and_fixed_axes_stay", () => {
    const s = ok(
      'box a "Payments API" at (0,0) size (auto, 60)\n' +
        'box b "Q" at (0,100) size (200, auto) shape=diamond\n' +
        'box c "Payments API" at (0,200) size (auto,auto)\n',
    );
    const r = (id: string): Rect => shape(s, id).rect;
    expect([r("a").w, r("a").h]).toEqual([108.0, 60.0]);
    expect([r("b").w, r("b").h]).toEqual([200.0, 77.0]);
    expect([r("c").w, r("c").h]).toEqual([108.0, 34.0]);
  });

  test("fitted_sizes_round_up_to_the_grid", () => {
    const [s, d] = compile(
      'diagram "t" size (300,300) grid=10\n' +
        'box a "Payments API" at (0,0)\n' +
        'box b "Payments API" at (0,100) size (auto, 35)\n',
    );
    expect(
      d.every((d) => d.code === "off-grid"),
      dump(d),
    ).toBe(true);
    const r = (id: string): Rect => shape(s, id).rect;
    expect([r("a").w, r("a").h]).toEqual([110.0, 40.0]);
    expect([r("b").w, r("b").h]).toEqual([110.0, 35.0]);
  });

  test("anchors_and_pins_use_the_fitted_size", () => {
    const s = ok(
      'box a "Client" at (100,100) pin=center\nbox b "B" at a.right+(20,0) size (40,40) pin=left\n',
    );
    const a = shape(s, "a").rect;
    expect([a.center().x, a.center().y]).toEqual([100.0, 100.0]);
    expect(shape(s, "b").rect.x).toBe(a.right() + 20.0);
  });

  test("nothing_to_fit_is_a_size_error", () => {
    let d = errors("box a at (0,0)\nbox b at (0,0) size (auto, 40)\n");
    expect(d.length, dump(d)).toBe(2);
    expect(d.every((d) => d.code === "size")).toBe(true);
    expect(d[0]!.message).toBe("`a` has no size to fit: it has no text");
    expect(d[0]!.help).toEqual(["give it a size: `size (w,h)`"]);
    d = errors('box a "A" at (0,0) size (auto, 0)\n');
    expect(d[0]!.message).toBe("`a` must have a positive size");
  });
});
