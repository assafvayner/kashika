use kashika_text::{Font, Run, parse_markup};

fn run(text: &str, font: Font) -> Run {
    Run {
        text: text.into(),
        font,
    }
}

#[test]
fn plain_text_is_one_regular_run() {
    assert_eq!(
        parse_markup("API", false).unwrap(),
        vec![vec![run("API", Font::Regular)]]
    );
}

#[test]
fn bold_and_code_runs() {
    assert_eq!(
        parse_markup("a **b** `c`", false).unwrap(),
        vec![vec![
            run("a ", Font::Regular),
            run("b", Font::Bold),
            run(" ", Font::Regular),
            run("c", Font::Mono)
        ]]
    );
}

#[test]
fn bold_base_makes_plain_runs_bold() {
    assert_eq!(
        parse_markup("a `c`", true).unwrap(),
        vec![vec![run("a ", Font::Bold), run("c", Font::Mono)]]
    );
}

#[test]
fn markers_do_not_nest() {
    assert_eq!(
        parse_markup("`**x**`", false).unwrap(),
        vec![vec![run("**x**", Font::Mono)]]
    );
    assert_eq!(
        parse_markup("**`x`**", false).unwrap(),
        vec![vec![run("`x`", Font::Bold)]]
    );
}

#[test]
fn escapes_and_single_stars_are_literal() {
    assert_eq!(
        parse_markup(r"\*\*x\*\* a*b \`", false).unwrap(),
        vec![vec![run("**x** a*b `", Font::Regular)]]
    );
}

#[test]
fn escaped_backslash_is_literal_and_does_not_escape_a_marker() {
    assert_eq!(
        parse_markup(r"a\\b \\**c**", false).unwrap(),
        vec![vec![run(r"a\b \", Font::Regular), run("c", Font::Bold)]]
    );
}

#[test]
fn newlines_split_lines_and_markers_carry_across() {
    assert_eq!(
        parse_markup("**a\nb**", false).unwrap(),
        vec![vec![run("a", Font::Bold)], vec![run("b", Font::Bold)]]
    );
    assert_eq!(parse_markup("", false).unwrap(), vec![Vec::<Run>::new()]);
}

#[test]
fn unclosed_markers_are_errors() {
    assert_eq!(parse_markup("**a", false).unwrap_err().marker, "**");
    assert_eq!(parse_markup("`a", false).unwrap_err().marker, "`");
    assert_eq!(
        parse_markup("**a", false).unwrap_err().to_string(),
        "unclosed `**` in text"
    );
}

#[test]
fn tabs_become_four_spaces() {
    assert_eq!(
        parse_markup("a\tb", false).unwrap(),
        vec![vec![run("a    b", Font::Regular)]]
    );
    assert_eq!(kashika_text::TAB_WIDTH, 4);
}

#[test]
fn trailing_backslash_and_lone_escapes_stay_literal() {
    assert_eq!(
        parse_markup(r"a\", false).unwrap(),
        vec![vec![run(r"a\", Font::Regular)]]
    );
    assert_eq!(
        parse_markup(r"\n \x", false).unwrap(),
        vec![vec![run(r"\n \x", Font::Regular)]]
    );
}

#[test]
fn empty_markers_make_no_runs() {
    assert_eq!(
        parse_markup("****", false).unwrap(),
        vec![Vec::<Run>::new()]
    );
    assert_eq!(parse_markup("``", false).unwrap(), vec![Vec::<Run>::new()]);
    assert_eq!(
        parse_markup("a****b", false).unwrap(),
        vec![vec![run("ab", Font::Regular)]]
    );
}

#[test]
fn bold_text_adjacent_to_bold_base_merges_runs() {
    assert_eq!(
        parse_markup("a**b**c", true).unwrap(),
        vec![vec![run("abc", Font::Bold)]]
    );
}

#[test]
fn star_runs_pair_left_to_right() {
    assert_eq!(
        parse_markup("***a**", false).unwrap(),
        vec![vec![run("*a", Font::Bold)]]
    );
    assert_eq!(
        parse_markup("**a***", false).unwrap(),
        vec![vec![run("a", Font::Bold), run("*", Font::Regular)]]
    );
}

#[test]
fn code_keeps_spaces_and_single_stars() {
    assert_eq!(
        parse_markup("`  a * b  `", false).unwrap(),
        vec![vec![run("  a * b  ", Font::Mono)]]
    );
}

#[test]
fn escaped_markers_inside_code_and_bold() {
    assert_eq!(
        parse_markup(r"`a\`b`", false).unwrap(),
        vec![vec![run("a`b", Font::Mono)]]
    );
    assert_eq!(
        parse_markup(r"**a\*\*b**", false).unwrap(),
        vec![vec![run("a**b", Font::Bold)]]
    );
}

#[test]
fn blank_lines_and_trailing_newline_keep_their_lines() {
    assert_eq!(
        parse_markup("a\n\nb\n", false).unwrap(),
        vec![
            vec![run("a", Font::Regular)],
            vec![],
            vec![run("b", Font::Regular)],
            vec![]
        ]
    );
}

#[test]
fn non_ascii_text_is_kept() {
    assert_eq!(
        parse_markup("é**日**", false).unwrap(),
        vec![vec![run("é", Font::Regular), run("日", Font::Bold)]]
    );
}
