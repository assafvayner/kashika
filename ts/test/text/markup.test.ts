import { describe, expect, test } from "vitest";
import { type Font, MarkupError, Run, TAB_WIDTH, parseMarkup } from "../../src/text/index.js";

function run(text: string, font: Font): Run {
  return new Run(text, font);
}

function parse(s: string, bold: boolean): Run[][] {
  const r = parseMarkup(s, bold);
  if (!r.ok) throw new Error(r.error.toString());
  return r.value;
}

function parseErr(s: string, bold: boolean): MarkupError {
  const r = parseMarkup(s, bold);
  if (r.ok) throw new Error("expected an error");
  return r.error;
}

describe("markup", () => {
  test("plain_text_is_one_regular_run", () => {
    expect(parse("API", false)).toEqual([[run("API", "regular")]]);
  });

  test("bold_and_code_runs", () => {
    expect(parse("a **b** `c`", false)).toEqual([
      [run("a ", "regular"), run("b", "bold"), run(" ", "regular"), run("c", "mono")],
    ]);
  });

  test("bold_base_makes_plain_runs_bold", () => {
    expect(parse("a `c`", true)).toEqual([[run("a ", "bold"), run("c", "mono")]]);
  });

  test("markers_do_not_nest", () => {
    expect(parse("`**x**`", false)).toEqual([[run("**x**", "mono")]]);
    expect(parse("**`x`**", false)).toEqual([[run("`x`", "bold")]]);
  });

  test("escapes_and_single_stars_are_literal", () => {
    expect(parse("\\*\\*x\\*\\* a*b \\`", false)).toEqual([[run("**x** a*b `", "regular")]]);
  });

  test("escaped_backslash_is_literal_and_does_not_escape_a_marker", () => {
    expect(parse("a\\\\b \\\\**c**", false)).toEqual([
      [run("a\\b \\", "regular"), run("c", "bold")],
    ]);
  });

  test("newlines_split_lines_and_markers_carry_across", () => {
    expect(parse("**a\nb**", false)).toEqual([[run("a", "bold")], [run("b", "bold")]]);
    expect(parse("", false)).toEqual([[]]);
  });

  test("unclosed_markers_are_errors", () => {
    expect(parseErr("**a", false).marker).toBe("**");
    expect(parseErr("`a", false).marker).toBe("`");
    expect(parseErr("**a", false).toString()).toBe("unclosed `**` in text");
  });

  test("tabs_become_four_spaces", () => {
    expect(parse("a\tb", false)).toEqual([[run("a    b", "regular")]]);
    expect(TAB_WIDTH).toBe(4);
  });

  test("trailing_backslash_and_lone_escapes_stay_literal", () => {
    expect(parse("a\\", false)).toEqual([[run("a\\", "regular")]]);
    expect(parse("\\n \\x", false)).toEqual([[run("\\n \\x", "regular")]]);
  });

  test("empty_markers_make_no_runs", () => {
    expect(parse("****", false)).toEqual([[]]);
    expect(parse("``", false)).toEqual([[]]);
    expect(parse("a****b", false)).toEqual([[run("ab", "regular")]]);
  });

  test("bold_text_adjacent_to_bold_base_merges_runs", () => {
    expect(parse("a**b**c", true)).toEqual([[run("abc", "bold")]]);
  });

  test("star_runs_pair_left_to_right", () => {
    expect(parse("***a**", false)).toEqual([[run("*a", "bold")]]);
    expect(parse("**a***", false)).toEqual([[run("a", "bold"), run("*", "regular")]]);
  });

  test("code_keeps_spaces_and_single_stars", () => {
    expect(parse("`  a * b  `", false)).toEqual([[run("  a * b  ", "mono")]]);
  });

  test("escaped_markers_inside_code_and_bold", () => {
    expect(parse("`a\\`b`", false)).toEqual([[run("a`b", "mono")]]);
    expect(parse("**a\\*\\*b**", false)).toEqual([[run("a**b", "bold")]]);
  });

  test("blank_lines_and_trailing_newline_keep_their_lines", () => {
    expect(parse("a\n\nb\n", false)).toEqual([
      [run("a", "regular")],
      [],
      [run("b", "regular")],
      [],
    ]);
  });

  test("non_ascii_text_is_kept", () => {
    expect(parse("é**日**", false)).toEqual([[run("é", "regular"), run("日", "bold")]]);
  });
});
