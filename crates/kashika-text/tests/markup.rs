use kashika_text::{parse_markup, Font, Run};

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
