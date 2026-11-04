import { describe, expect, test } from "vitest";
import { type Diagnostic } from "../../src/diag/diagnostic.js";
import { Pt } from "../../src/geom/index.js";
import { type Font, type Scene, compile } from "../../src/model/index.js";

const HEAD = 'diagram "t" size (800,600)\n';

function ok(body: string): Scene {
  const [scene, diags] = compile(`${HEAD}${body}`);
  expect(diags, JSON.stringify(diags.map((d) => [d.code, d.message]))).toEqual([]);
  return scene;
}

function errors(body: string): Diagnostic[] {
  return compile(`${HEAD}${body}`)[1];
}

function close(a: number, b: number): boolean {
  return Math.abs(a - b) < 1e-3;
}

describe("text_free", () => {
  test("free_text_is_measured_around_its_anchor", () => {
    const s = ok('text t "hi" at (50,40) align=left\ntext u "note" at (0,0) muted\n');
    const t = s.texts[0]!;
    expect([t.rect.x, t.rect.y, t.rect.h]).toEqual([50.0, 31.25, 17.5]);
    expect(close(t.rect.w, 11.6689)).toBe(true);
    expect(t.lines.length).toBe(1);
    expect(s.texts[1]!.color).toBe("#64748b");
  });

  test("color_wins_over_muted", () => {
    const s = ok('text "x" at (0,0) muted color=#ff0000\n');
    expect(s.texts[0]!.color).toBe("#ff0000");
  });

  test("free_text_markup_picks_fonts", () => {
    const s = ok('text "a `b`" at (100,100)\n');
    const fonts: Font[] = s.texts[0]!.lines[0]!.runs.map((r) => r.font);
    expect(fonts).toEqual(["regular", "mono"]);
  });

  test("line_labels_are_placed_on_the_path", () => {
    const s = ok('line l (0,100) -> h 200 label="SQL" label@0.25="a"\n');
    const l = s.lines[0]!;
    expect(l.labels[0]!.at).toEqual(new Pt(100.0, 100.0));
    expect(close(l.labels[0]!.rect.center().y, 100.0)).toBe(true);
    expect(close(l.labels[0]!.rect.w, 23.6602)).toBe(true);
    expect(l.labels[1]!.at).toEqual(new Pt(50.0, 100.0));
    expect(l.labels[0]!.span.line).toBe(2);
  });

  test("unclosed_markup_in_free_text_and_labels_is_a_parse_error", () => {
    let d = errors('text "`x" at (0,0)\n');
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("parse");
    expect(d[0]!.message).toBe("unclosed ``` in text");
    d = errors('line l (0,0) -> h 50 label="**x"\n');
    expect(d.length, JSON.stringify(d)).toBe(1);
    expect(d[0]!.code).toBe("parse");
    expect(d[0]!.message).toBe("unclosed `**` in text");
  });

  test("escapes_are_literal_and_raw_strings_are_kept", () => {
    const s = ok('text "\\*\\*x\\*\\* \\\\ y" at (0,0)\nline l (0,0) -> h 5 label="\\`"\n');
    expect(s.texts[0]!.content).toBe("\\*\\*x\\*\\* \\\\ y");
    const runs = [...s.texts[0]!.lines[0]!.runs];
    expect(runs.length).toBe(1);
    expect(runs[0]!.text).toBe("**x** \\ y");
    expect(s.lines[0]!.labels[0]!.lines[0]!.runs[0]!.text).toBe("`");
  });

  test("escaped_backslash_does_not_escape_the_next_marker", () => {
    const s = ok('text "\\\\**b**" at (0,0)\n');
    const runs = s.texts[0]!.lines[0]!.runs;
    expect([runs[0]!.text, runs[0]!.font]).toEqual(["\\", "regular"]);
    expect([runs[1]!.text, runs[1]!.font]).toEqual(["b", "bold"]);
  });
});
