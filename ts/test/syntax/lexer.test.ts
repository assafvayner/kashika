import { describe, expect, it } from "vitest";
import { Tok, lex } from "../../src/syntax/index.js";

function toks(src: string): Tok[] {
  const { toks: t, diags: d } = lex(src);
  expect(d).toEqual([]);
  return t.map((t) => t.tok);
}

const u = Tok.unit;
const id = (s: string): Tok => Tok.Ident(s);

describe("lexer", () => {
  it("lexes_statement_tokens", () => {
    expect(toks('box a-b "x" at db.top@0.5+(-1,2) fill=#fff // c\n')).toEqual([
      id("box"),
      id("a-b"),
      Tok.Str("x"),
      id("at"),
      id("db"),
      u("Dot"),
      id("top"),
      u("At"),
      Tok.Num(0.5),
      u("Plus"),
      u("LParen"),
      Tok.Num(-1.0),
      u("Comma"),
      Tok.Num(2.0),
      u("RParen"),
      id("fill"),
      u("Eq"),
      Tok.Color("#fff"),
      u("Newline"),
      u("Eof"),
    ]);
  });

  it("minus_before_paren_ends_an_identifier", () => {
    expect(toks("a-b.top-(-1,2)")).toEqual([
      id("a-b"),
      u("Dot"),
      id("top"),
      u("Minus"),
      u("LParen"),
      Tok.Num(-1.0),
      u("Comma"),
      Tok.Num(2.0),
      u("RParen"),
      u("Eof"),
    ]);
  });

  it("arrow_is_not_part_of_an_identifier", () => {
    expect(toks("a->b")).toEqual([id("a"), u("Arrow"), id("b"), u("Eof")]);
  });

  it("string_escapes_keep_markup_escapes", () => {
    expect(toks('"a\\nb\\"c\\*"')[0]).toEqual(Tok.Str('a\nb"c\\*'));
    expect(toks('"a\\\\*"')[0]).toEqual(Tok.Str("a\\\\*"));
  });

  it("spans_are_one_based", () => {
    const { toks: t } = lex("  box");
    expect([t[0]!.span.line, t[0]!.span.col, t[0]!.span.endCol]).toEqual([1, 3, 6]);
  });

  it("hash_comment_is_one_error_with_help", () => {
    for (const src of ["# note\n", "#note\n", "box a # note\n"]) {
      const { toks: t, diags: d } = lex(src);
      expect(d.length, src).toBe(1);
      expect(d[0]!.message).toBe("`#` does not start a comment");
      expect(d[0]!.help[0]).toContain("`//`");
      expect(t.some((t) => t.tok.kind === "Color")).toBe(false);
    }
  });

  it("bad_color_after_eq_is_an_invalid_color", () => {
    const { diags: d } = lex("fill=# fff\n");
    expect(d.length).toBe(1);
    expect(d[0]!.message).toBe("invalid color `#`");
  });

  it("unterminated_string", () => {
    const { diags: d } = lex('"abc\nbox');
    expect(d[0]!.message).toBe("unterminated string");
  });

  it("inline_block_comment_emits_nothing", () => {
    expect(toks("box /* inline */ a")).toEqual([id("box"), id("a"), u("Eof")]);
  });

  it("multiline_block_comment_is_one_newline", () => {
    const { toks: t, diags: d } = lex("box a /* x\ny\nz */ box b");
    expect(d).toEqual([]);
    expect(t.map((t) => t.tok)).toEqual([
      id("box"),
      id("a"),
      u("Newline"),
      id("box"),
      id("b"),
      u("Eof"),
    ]);
    expect(t[3]!.span.line).toBe(3);
  });

  it("block_comments_do_not_nest", () => {
    const { toks: t } = lex("/* a /* b */ c */");
    expect(t[0]!.tok).toEqual(id("c"));
  });

  it("block_comment_marker_in_string_is_text", () => {
    expect(toks('"/* not a comment */"')).toEqual([Tok.Str("/* not a comment */"), u("Eof")]);
  });

  it("unterminated_block_comment", () => {
    const { diags: d } = lex("box /* never closed");
    expect(d.length).toBe(1);
    expect(d[0]!.message).toBe("unterminated block comment");
  });
});
