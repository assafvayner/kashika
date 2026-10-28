use kashika_syntax::{lex, lex_with_comments, Tok};

#[test]
fn comments_are_collected_with_spans() {
    let src = "box a // hi \n/* b\n c */ box d\n";
    let (toks, comments, diags) = lex_with_comments(src);
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].text, "// hi");
    assert_eq!((comments[0].span.line, comments[0].span.col), (1, 7));
    assert_eq!(comments[1].text, "/* b\n c */");
    assert_eq!((comments[1].span.line, comments[1].span.end_line), (2, 3));
    assert_eq!(toks, lex(src).0);
    assert!(toks
        .iter()
        .all(|t| !matches!(&t.tok, Tok::Ident(s) if s.contains('/'))));
}

use kashika_syntax::format;

fn fmt(src: &str) -> String {
    let out = format(src).unwrap_or_else(|d| panic!("{d:#?}"));
    assert_eq!(format(&out).unwrap(), out, "not idempotent");
    out
}

#[test]
fn normalizes_token_spacing() {
    assert_eq!(
        fmt("box  a \"A\" at ( 10 , 20 ) size (1,1)   fill = #fff\n"),
        "box a \"A\" at (10,20) size (1,1) fill=#fff\n"
    );
    assert_eq!(
        fmt("line l  db.top + ( -10 , 0 ) -> h 20 label @ 0.5 = \"x\"\n"),
        "line l db.top+(-10,0) -> h 20 label@0.5=\"x\"\n"
    );
    assert_eq!(
        fmt("line l db.top - ( -10 , 0 ) -> b.left@0.25 -(0,4)\n"),
        "line l db.top-(-10,0) -> b.left@0.25-(0,4)\n"
    );
    assert_eq!(
        fmt("line l a.top -> curve  left   30 b.top allow = cross : m , near\n"),
        "line l a.top -> curve left 30 b.top allow=cross:m,near\n"
    );
    assert_eq!(
        fmt("junction j at (a.bottom.x,130)\nline l (j.x , 10) -> ( 5 , 6 )\n"),
        "junction j at (a.bottom.x, 130)\nline l (j.x, 10) -> (5,6)\n"
    );
}

#[test]
fn keeps_number_and_string_spelling() {
    assert_eq!(
        fmt("box a \"a\\*b \\\"q\\\"\" at (10.50,020) size (1,1)\n"),
        "box a \"a\\*b \\\"q\\\"\" at (10.50,020) size (1,1)\n"
    );
}

#[test]
fn indents_blocks_and_continuations() {
    let src = "diagram \"d\" size (100,100)\n\
group g at (0,0) size (100,100) {\n\
box a at (10,30) size (20,20)\n\
    line l a.right\n\
  -> h 20\n\
        end=arrow\n\
}\n";
    let want = "diagram \"d\" size (100,100)\n\
group g at (0,0) size (100,100) {\n\
\x20 box a at (10,30) size (20,20)\n\
\x20 line l a.right\n\
\x20   -> h 20\n\
\x20   end=arrow\n\
}\n";
    assert_eq!(fmt(src), want);
}

#[test]
fn style_block_lines_are_not_continuations() {
    assert_eq!(
        fmt("style s {\nfill=#fff\n    stroke=#000\n}\n"),
        "style s {\n  fill=#fff\n  stroke=#000\n}\n"
    );
    assert_eq!(fmt("style s {fill=#fff}\n"), "style s { fill=#fff }\n");
}

#[test]
fn keeps_comments_and_collapses_blank_lines() {
    let src = "\n\n// header\n\n\nbox a at (0,0) size (1,1)    // trailing\n/* block\n   comment */\ngroup g at (0,0) size (9,9) {\n\n    // inside\nbox b at (1,1) size (1,1)\n\n}\n\n\n";
    let want = "// header\n\nbox a at (0,0) size (1,1)  // trailing\n/* block\n   comment */\ngroup g at (0,0) size (9,9) {\n  // inside\n  box b at (1,1) size (1,1)\n}\n";
    assert_eq!(fmt(src), want);
}

#[test]
fn formats_statements_the_parser_does_not_build_yet() {
    assert_eq!(
        fmt("import   \"lib.kash\"  as aws\n"),
        "import \"lib.kash\" as aws\n"
    );
}

#[test]
fn formats_future_blocks_without_knowing_their_grammar() {
    assert_eq!(
        fmt("box api at (100,80) size (200,120) text-align=left {\ntext \"**Payments API**\"\n      text \"v2.4.1\" size=11 muted\n}\n"),
        "box api at (100,80) size (200,120) text-align=left {\n  text \"**Payments API**\"\n  text \"v2.4.1\" size=11 muted\n}\n"
    );
    assert_eq!(
        fmt("shape lb_icon {\npath \"M 0 0.5 L 1 0.5\"\n    outline 1\ntext-area ( 0.2,0.3 ,0.6,0.4)\n}\n"),
        "shape lb_icon {\n  path \"M 0 0.5 L 1 0.5\"\n  outline 1\n  text-area (0.2,0.3,0.6,0.4)\n}\n"
    );
}

#[test]
fn refuses_parse_errors() {
    let errs = format("box a at\n").unwrap_err();
    assert_eq!(errs[0].code, "parse");
}

#[test]
fn drops_a_byte_order_mark() {
    assert_eq!(
        fmt("\u{feff}box a at (1,2) size (3,4)\n"),
        "box a at (1,2) size (3,4)\n"
    );
}

#[test]
fn empty_input_stays_empty() {
    assert_eq!(fmt(""), "");
    assert_eq!(fmt("\n\n"), "");
}

#[test]
fn version_attribute_round_trips() {
    let src = "diagram \"t\" size (1,1) kashika=\"0.1.0\"\n";
    assert_eq!(fmt(src), src);
}

#[test]
fn keeps_at_size_and_attribute_order() {
    assert_eq!(
        fmt("box  a \"A\" style=service size ( 9 , 9 )  at (1,2)\njunction j dot  at (0,0)\n"),
        "box a \"A\" style=service size (9,9) at (1,2)\njunction j dot at (0,0)\n"
    );
}
