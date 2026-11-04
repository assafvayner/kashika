import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { utf8Len } from "../../src/diag/index.js";
import { format, lex, lexWithComments } from "../../src/syntax/index.js";

function fmt(src: string): string {
  const out = format(src);
  if (!out.ok) throw new Error(JSON.stringify(out.error.map((d) => d.message)));
  const again = format(out.value);
  expect(again, "not idempotent").toEqual({ ok: true, value: out.value });
  return out.value;
}

function exampleSources(): string[] {
  const out: string[] = [];
  const walk = (dir: string) => {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      if (statSync(p).isDirectory()) {
        walk(p);
      } else if (p.endsWith(".kash")) {
        out.push(readFileSync(p, "utf8"));
      }
    }
  };
  walk(join(import.meta.dirname, "../../../examples"));
  expect(out.length).toBeGreaterThan(0);
  return out;
}

describe("fmt", () => {
  it("comments_are_collected_with_spans", () => {
    const src = "box a // hi \n/* b\n c */ box d\n";
    const { toks, comments, diags } = lexWithComments(src);
    expect(diags).toEqual([]);
    expect(comments.length).toBe(2);
    expect(comments[0]!.text).toBe("// hi");
    expect([comments[0]!.span.line, comments[0]!.span.col]).toEqual([1, 7]);
    expect(comments[1]!.text).toBe("/* b\n c */");
    expect([comments[1]!.span.line, comments[1]!.span.endLine]).toEqual([2, 3]);
    expect(toks).toEqual(lex(src).toks);
    expect(toks.every((t) => !(t.tok.kind === "Ident" && t.tok.value.includes("/")))).toBe(true);
  });

  it("normalizes_token_spacing", () => {
    expect(fmt('box  a "A" at ( 10 , 20 ) size (1, 1)   fill = #fff\n')).toBe(
      'box a "A" at (10, 20) size (1, 1) fill=#fff\n',
    );
    expect(fmt('line l  db.top + ( -10 , 0 ) -> h 20 label @ 0.5 = "x"\n')).toBe(
      'line l db.top+(-10, 0) -> h 20 label@0.5="x"\n',
    );
    expect(fmt("line l db.top - ( -10 , 0 ) -> b.left@0.25 -(0, 4)\n")).toBe(
      "line l db.top-(-10, 0) -> b.left@0.25-(0, 4)\n",
    );
    expect(fmt("line l a.top -> curve  left   30 b.top allow = cross : m , near\n")).toBe(
      "line l a.top -> curve left 30 b.top allow=cross:m,near\n",
    );
    expect(fmt("junction j at (a.bottom.x, 130)\nline l (j.x , 10) -> ( 5 , 6 )\n")).toBe(
      "junction j at (a.bottom.x, 130)\nline l (j.x, 10) -> (5, 6)\n",
    );
  });

  it("keeps_point_parentheses", () => {
    expect(fmt("box b at ( a.center + ( 0 , 40 ) ) size (1, 1)\n")).toBe(
      "box b at (a.center+(0, 40)) size (1, 1)\n",
    );
    expect(
      fmt("line l (a.bottom) -> x ( c.left ) -> curve via (fan) ( b.top-(0, 4) ) (d.left)\n"),
    ).toBe("line l (a.bottom) -> x (c.left) -> curve via (fan) (b.top-(0, 4)) (d.left)\n");
    expect(fmt("box b at ( ( a.left.x ,b.y ) + ( 0 , 4 ) ) - (1, 1) size (1, 1)\n")).toBe(
      "box b at ((a.left.x, b.y)+(0, 4))-(1, 1) size (1, 1)\n",
    );
    expect(fmt("line l (10, 20) + (5, 0) -> a.c + (1, 2) + (3, 4) -> (a.c)-(1, 1)\n")).toBe(
      "line l (10, 20)+(5, 0) -> a.c+(1, 2)+(3, 4) -> (a.c)-(1, 1)\n",
    );
  });

  it("keeps_number_and_string_spelling", () => {
    expect(fmt('box a "a\\*b \\"q\\"" at (10.50, 020) size (1, 1)\n')).toBe(
      'box a "a\\*b \\"q\\"" at (10.50, 020) size (1, 1)\n',
    );
  });

  it("indents_blocks_and_continuations", () => {
    const src =
      'diagram "d" size (100, 100)\n' +
      "group g at (0, 0) size (100, 100) {\n" +
      "box a at (10, 30) size (20, 20)\n" +
      "    line l a.right\n" +
      "  -> h 20\n" +
      "        end=arrow\n" +
      "}\n";
    const want =
      'diagram "d" size (100, 100)\n' +
      "group g at (0, 0) size (100, 100) {\n" +
      "  box a at (10, 30) size (20, 20)\n" +
      "  line l a.right\n" +
      "    -> h 20\n" +
      "    end=arrow\n" +
      "}\n";
    expect(fmt(src)).toBe(want);
  });

  it("style_block_lines_are_not_continuations", () => {
    expect(fmt("style s {\nfill=#fff\n    stroke=#000\n}\n")).toBe(
      "style s {\n  fill=#fff\n  stroke=#000\n}\n",
    );
    expect(fmt("style s {fill=#fff}\n")).toBe("style s { fill=#fff }\n");
  });

  it("keeps_comments_and_collapses_blank_lines", () => {
    const src =
      "\n\n// header\n\n\nbox a at (0, 0) size (1, 1)    // trailing\n/* block\n   comment */\ngroup g at (0, 0) size (9, 9) {\n\n    // inside\nbox b at (1, 1) size (1, 1)\n\n}\n\n\n";
    const want =
      "// header\n\nbox a at (0, 0) size (1, 1)  // trailing\n/* block\n   comment */\ngroup g at (0, 0) size (9, 9) {\n  // inside\n  box b at (1, 1) size (1, 1)\n}\n";
    expect(fmt(src)).toBe(want);
  });

  it("formats_statements_the_parser_does_not_build_yet", () => {
    expect(fmt('import   "lib.kash"  as aws\n')).toBe('import "lib.kash" as aws\n');
  });

  it("formats_future_blocks_without_knowing_their_grammar", () => {
    expect(
      fmt(
        'box api at (100, 80) size (200, 120) text-align=left {\ntext "**Payments API**"\n      text "v2.4.1" size=11 muted\n}\n',
      ),
    ).toBe(
      'box api at (100, 80) size (200, 120) text-align=left {\n  text "**Payments API**"\n  text "v2.4.1" size=11 muted\n}\n',
    );
    expect(
      fmt(
        'shape lb_icon {\npath "M 0 0.5 L 1 0.5"\n    outline 1\ntext-area ( 0.2,0.3 ,0.6,0.4)\n}\n',
      ),
    ).toBe(
      'shape lb_icon {\n  path "M 0 0.5 L 1 0.5"\n  outline 1\n  text-area (0.2, 0.3, 0.6, 0.4)\n}\n',
    );
  });

  it("refuses_parse_errors", () => {
    const r = format("box a at\n");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error[0]!.code).toBe("parse");
  });

  it("drops_a_byte_order_mark", () => {
    expect(fmt("\u{feff}box a at (1, 2) size (3, 4)\n")).toBe("box a at (1, 2) size (3, 4)\n");
  });

  it("empty_input_stays_empty", () => {
    expect(fmt("")).toBe("");
    expect(fmt("\n\n")).toBe("");
  });

  it("version_attribute_round_trips", () => {
    const src = 'diagram "t" size (1, 1) kashika="0.1.0"\n';
    expect(fmt(src)).toBe(src);
  });

  it("keeps_at_size_and_attribute_order", () => {
    expect(
      fmt('box  a "A" style=service size ( 9 , 9 )  at (1, 2)\njunction j dot  at (0, 0)\n'),
    ).toBe('box a "A" style=service size (9, 9) at (1, 2)\njunction j dot at (0, 0)\n');
  });

  it("crlf_is_normalized_inside_block_comments", () => {
    const out = format(
      "box a at (1, 1) size (1, 1)\r\n/* x\r\n y */\r\nbox b at (5, 5) size (1, 1)\r\n",
    );
    expect(out.ok).toBe(true);
    if (!out.ok) return;
    expect(out.value.includes("\r")).toBe(false);
    expect(format(out.value)).toEqual({ ok: true, value: out.value });
  });

  it("examples_format_idempotently", () => {
    for (const src of exampleSources()) {
      const once = format(src);
      expect(once.ok).toBe(true);
      if (!once.ok) continue;
      expect(format(once.value)).toEqual({ ok: true, value: once.value });
    }
  });

  it("truncated_examples_never_panic_and_format_idempotently", () => {
    for (const src of exampleSources()) {
      const cs = Array.from(src);
      const ends: number[] = [];
      for (let i = 0; i <= cs.length; i++) ends.push(i);
      for (let k = 0; k < ends.length; k += 7) {
        const prefix = cs.slice(0, ends[k]).join("");
        const once = format(prefix);
        if (once.ok) {
          expect(
            format(once.value),
            `not idempotent for prefix of ${utf8Len(prefix)} bytes`,
          ).toEqual({ ok: true, value: once.value });
        }
      }
    }
  });

  it("keeps_keywords_and_attribute_names_as_written", () => {
    expect(
      fmt('arrow  ab a.right ->  b.left\n  end=none\nbox  a "A" at (0, 0) size ( auto , 40 )\n'),
    ).toBe('arrow ab a.right -> b.left\n  end=none\nbox a "A" at (0, 0) size (auto, 40)\n');
  });

  it("spaces_every_comma_inside_parentheses_once", () => {
    expect(fmt('diagram "d" size (140,auto)\n')).toBe('diagram "d" size (140, auto)\n');
    expect(
      fmt("box a at (10,20) size (140,60) padding=(8 ,4)\nbox b at a.right+(40,-4) pin=left\n"),
    ).toBe("box a at (10, 20) size (140, 60) padding=(8, 4)\nbox b at a.right+(40, -4) pin=left\n");
    expect(fmt("box c at (a.x,b.y)-( 1,2 ) size (1,1)\n")).toBe(
      "box c at (a.x, b.y)-(1, 2) size (1, 1)\n",
    );
    expect(fmt("style s {\nsize=(140,auto) padding=(6,  2)\n}\n")).toBe(
      "style s {\n  size=(140, auto) padding=(6, 2)\n}\n",
    );
    expect(fmt('shape t {\npath "M 0 0 L 1 1"\ntext-area (0.2,0.3,0.6,0.4)\n}\n')).toBe(
      'shape t {\n  path "M 0 0 L 1 1"\n  text-area (0.2, 0.3, 0.6, 0.4)\n}\n',
    );
    expect(fmt("line l (0,0) -> h 5 allow=cross:m,near\n")).toBe(
      "line l (0, 0) -> h 5 allow=cross:m,near\n",
    );
  });
});
