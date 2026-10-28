use kashika_syntax::ast::*;
use kashika_syntax::parse;

fn parse_ok(src: &str) -> File {
    let (file, diags) = parse(src);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:#?}");
    file
}

#[test]
fn parses_diagram_and_box() {
    let f = parse_ok("diagram \"T\" size (400,300) grid=10\nbox api \"API\" at (10,20) size (120,60) style=service\n");
    assert_eq!(f.stmts.len(), 2);
    let Stmt::Box(b) = &f.stmts[1] else {
        panic!("expected box")
    };
    assert_eq!(b.id.name, "api");
    assert_eq!(b.label.as_deref(), Some("API"));
    assert_eq!(b.size, (120.0, 60.0));
    assert_eq!(b.attrs[0].key.name, "style");
    assert_eq!(b.attrs[0].value, Value::Word("service".into()));
    let Point::Lit {
        x: Coord::Num(x),
        y: Coord::Num(y),
        ..
    } = &b.at
    else {
        panic!("expected literal point")
    };
    assert_eq!((*x, *y), (10.0, 20.0));
}

#[test]
fn parses_every_segment_kind() {
    let f = parse_ok(
        "line l1: a.right -> h 10 -> v -20 -> x 50 -> y b.top -> hv c.left@0.25 -> vh (1,2) \
         -> curve left:30 d.top -> curve via (0,0) (5,5) e.bottom -> f.left+(0,-4) end=arrow ortho\n",
    );
    let Stmt::Line(l) = &f.stmts[0] else {
        panic!("expected line")
    };
    assert_eq!(l.id.as_ref().unwrap().name, "l1");
    let kinds: Vec<&str> = l
        .segs
        .iter()
        .map(|s| match s.kind {
            SegKind::To(_) => "to",
            SegKind::H(_) => "h",
            SegKind::V(_) => "v",
            SegKind::X(_) => "x",
            SegKind::Y(_) => "y",
            SegKind::Hv(_) => "hv",
            SegKind::Vh(_) => "vh",
            SegKind::Curve { .. } => "curve",
            SegKind::CurveVia { .. } => "via",
        })
        .collect();
    assert_eq!(
        kinds,
        ["h", "v", "x", "y", "hv", "vh", "curve", "via", "to"]
    );
    assert_eq!(l.segs[1].kind, SegKind::V(-20.0));
    assert!(
        matches!(&l.segs[6].kind, SegKind::Curve { side: Side::Left, amount, .. } if *amount == 30.0)
    );
    assert_eq!(l.attrs.len(), 2);
    assert_eq!(l.attrs[1].value, Value::Flag);
}

#[test]
fn parses_mixed_coordinates_and_offsets() {
    let f = parse_ok("junction j at (db.left.x, lane.y)\ntext \"hi\" at db.top@0.5+(0,-10)\n");
    let Stmt::Junction(j) = &f.stmts[0] else {
        panic!("expected junction")
    };
    let Point::Lit {
        x: Coord::Axis { r, axis: Axis::X },
        y: Coord::Axis {
            r: r2,
            axis: Axis::Y,
        },
        ..
    } = &j.at
    else {
        panic!("{:?}", j.at)
    };
    assert_eq!(r.anchor.as_ref().unwrap().name.name, "left");
    assert_eq!(r2.id.name, "lane");
    assert!(r2.anchor.is_none());
    let Stmt::Text(t) = &f.stmts[1] else {
        panic!("expected text")
    };
    let Point::Ref { r, offset, .. } = &t.at else {
        panic!("expected ref")
    };
    assert_eq!(r.anchor.as_ref().unwrap().t, Some(0.5));
    assert_eq!(*offset, Some((0.0, -10.0)));
}

#[test]
fn continues_lines_starting_with_arrow_or_attribute() {
    let f = parse_ok("line a.top\n  -> y 40\n  -> hv b.left\n  end=arrow label=\"SQL\"\nbox c at (0,0) size (1,1)\n");
    assert_eq!(f.stmts.len(), 2);
    let Stmt::Line(l) = &f.stmts[0] else {
        panic!("expected line")
    };
    assert_eq!(l.segs.len(), 2);
    assert_eq!(l.attrs.len(), 2);
}

#[test]
fn parses_groups_styles_comments_and_allow_lists() {
    let f = parse_ok(
        "// header\nstyle svc { fill=#e8f0fe\n stroke=#1a73e8 }\n\
         group vpc \"VPC\" at (0,0) size (100,100) {\n  box a at (1,1) size (10,10) // trailing\n}\n\
         line (0,0) -> h 5 allow=cross:l2,near label@0.3=\"x\"\n",
    );
    assert_eq!(f.stmts.len(), 3);
    let Stmt::Style(s) = &f.stmts[0] else {
        panic!("expected style")
    };
    assert_eq!(s.attrs[0].value, Value::Color("#e8f0fe".into()));
    let Stmt::Group(g) = &f.stmts[1] else {
        panic!("expected group")
    };
    assert_eq!(g.children.len(), 1);
    let Stmt::Line(l) = &f.stmts[2] else {
        panic!("expected line")
    };
    assert_eq!(l.attrs[0].value, Value::Word("cross:l2,near".into()));
    assert_eq!(l.attrs[1].at_t, Some(0.3));
}

#[test]
fn recovers_and_reports_multiple_errors() {
    let (f, diags) = parse("bx a at (0,0) size (1,1)\nbox b at (0,0) size (1,1)\nline c.top ->\nbox d at (0,0) size (1,1)\n");
    assert_eq!(diags.len(), 2, "{diags:#?}");
    assert!(diags[0]
        .help
        .iter()
        .any(|h| h.contains("did you mean `box`?")));
    assert!(diags[1].message.contains("found end of line"));
    assert_eq!(f.stmts.len(), 2);
}

#[test]
fn unclosed_group_is_reported() {
    let (_, diags) = parse("group g at (0,0) size (1,1) {\n box a at (0,0) size (1,1)\n");
    assert!(diags.iter().any(|d| d.message.contains("expected `}`")));
}

#[test]
fn block_comment_between_statements() {
    let f = parse_ok(
        "box a \"A\" at (0,0) size (10,10)\n/* spans\nlines */\nbox b \"B\" at (20,0) size (10,10)\n",
    );
    assert_eq!(f.stmts.len(), 2);
}

#[test]
fn identifiers_allow_underscores_digits_and_hyphens() {
    let f = parse_ok("box _a1 at (0,0) size (1,1)\nbox a--b at (0,0) size (1,1)\nbox c- at (0,0) size (1,1)\nline a--b.right->c-.left\n");
    let ids: Vec<_> = f
        .stmts
        .iter()
        .filter_map(|s| match s {
            Stmt::Box(b) => Some(b.id.name.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(ids, ["_a1", "a--b", "c-"]);
    let Stmt::Line(l) = &f.stmts[3] else {
        panic!("expected line")
    };
    assert_eq!(l.segs.len(), 1);
}

#[test]
fn numbers_are_integers_or_decimals_optionally_negative() {
    let f = parse_ok("box a at (-1.5,20) size (0.25,3)\n");
    let Stmt::Box(b) = &f.stmts[0] else {
        panic!("expected box")
    };
    let Point::Lit {
        x: Coord::Num(x),
        y: Coord::Num(y),
        ..
    } = &b.at
    else {
        panic!("expected literal point")
    };
    assert_eq!((*x, *y, b.size), (-1.5, 20.0, (0.25, 3.0)));
}

#[test]
fn colors_take_three_six_or_eight_hex_digits() {
    let f = parse_ok("style s { fill=#abc stroke=#a1b2c3 color=#a1b2c3d4 }\n");
    let Stmt::Style(s) = &f.stmts[0] else {
        panic!("expected style")
    };
    assert_eq!(s.attrs[2].value, Value::Color("#a1b2c3d4".into()));
    for bad in ["#ab", "#abcd", "#ggg"] {
        let (_, d) = parse(&format!("style s {{ fill={bad} }}\n"));
        assert_eq!(d[0].code, "parse", "{bad}");
    }
}

#[test]
fn statements_end_at_a_newline() {
    let (_, d) = parse("box a at (0,0) size (1,1) box b at (0,0) size (1,1)\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "parse");
    let (_, d) = parse("box a at (0,0)\n  size (1,1)\n");
    assert!(!d.is_empty());
}

#[test]
fn arrow_continuation_skips_blank_and_comment_lines() {
    let f = parse_ok("line a.top\n\n  // first hop\n  -> y 40\n  /* second\n     hop */\n  -> h 10 // trailing\n\n  end=arrow\nbox c at (0,0) size (1,1)\n");
    assert_eq!(f.stmts.len(), 2);
    let Stmt::Line(l) = &f.stmts[0] else {
        panic!("expected line")
    };
    assert_eq!((l.segs.len(), l.attrs.len()), (2, 1));
}

#[test]
fn a_new_statement_ends_a_line_without_arrow() {
    let f = parse_ok("line (0,0) -> h 5\nline (0,0)\n  -> v 5\n");
    assert_eq!(f.stmts.len(), 2);
}

#[test]
fn block_comment_spanning_lines_never_joins_statements() {
    let (_, d) = parse("box a at (0,0) /* x\n */ size (1,1)\n");
    assert!(!d.is_empty());
    assert_eq!(d[0].code, "parse");
}

#[test]
fn unclosed_block_comment_is_a_parse_error() {
    let (_, d) = parse("box a at (0,0) size (1,1)\n/* never closed\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "parse");
}

#[test]
fn line_attributes_parse() {
    let f = parse_ok("line l: (0,0) -> h 5 start=dot end=arrow stroke=#000 stroke-width=2 stroke-style=dashed style=s radius=4 label=\"a\" label@0.25=\"b\" ortho allow=cross reason=\"r\"\n");
    let Stmt::Line(l) = &f.stmts[0] else {
        panic!("expected line")
    };
    assert_eq!(l.attrs.len(), 12);
}

#[test]
fn text_parses_with_and_without_id() {
    let f = parse_ok("text t \"a\" at (0,0) size=12 align=left\ntext \"b\" at (0,0)\n");
    let ids: Vec<_> = f
        .stmts
        .iter()
        .map(|s| match s {
            Stmt::Text(t) => t.id.as_ref().map(|i| i.name.clone()),
            _ => panic!("expected text"),
        })
        .collect();
    assert_eq!(ids, [Some("t".to_string()), None]);
}

#[test]
fn junction_with_dot_flag() {
    let f = parse_ok("junction j at (0,0) dot\n");
    let Stmt::Junction(j) = &f.stmts[0] else {
        panic!("expected junction")
    };
    assert_eq!(j.attrs[0].value, Value::Flag);
}

#[test]
fn line_needs_a_segment() {
    let (_, d) = parse("line (0,0)\n");
    assert_eq!(d[0].code, "parse");
    assert!(d[0].message.contains("`->`"), "{d:#?}");
}

#[test]
fn nested_groups_parse() {
    let f = parse_ok("group a at (0,0) size (9,9) {\n  group b at (0,0) size (5,5) {\n    junction j at (0,0)\n    text \"t\" at (0,0)\n    line (0,0) -> h 1\n  }\n}\n");
    let Stmt::Group(a) = &f.stmts[0] else {
        panic!("expected group")
    };
    let Stmt::Group(b) = &a.children[0] else {
        panic!("expected group")
    };
    assert_eq!(b.children.len(), 3);
}

#[test]
fn unbuilt_statements_report_unsupported_and_recover() {
    let (f, diags) = parse(
        "import \"lib/aws.kash\" as aws\n\
         shape lb_icon {\n  path \"M 0 0 L 1 1\"\n  nested { a=1 { b=2 } }\n  text-area (0.2,0.3,0.6,0.4)\n}\n\
         shape s3 from \"icons/s3.svg\" outline=hull\n\
         image logo \"hf.png\" at (20,20) size (80,80) fit=contain\n\
         box api at (100,80) size (200,120)\n\
         box b at (0,0) size (1,1)\n",
    );
    let codes: Vec<_> = diags.iter().map(|d| d.code).collect();
    assert_eq!(codes, vec!["unsupported"; 4], "{diags:#?}");
    for d in &diags {
        assert!(d.help.iter().any(|h| h.contains("later version")), "{d:#?}");
    }
    assert_eq!(f.stmts.len(), 2, "{:#?}", f.stmts);
    assert!(matches!(&f.stmts[0], Stmt::Box(n) if n.id.name == "api"));
    assert!(matches!(&f.stmts[1], Stmt::Box(n) if n.id.name == "b"));
}

#[test]
fn unbuilt_statements_inside_groups_recover() {
    let (f, diags) = parse(
        "group g at (0,0) size (9,9) {\n  image i \"x.png\" at (0,0) size (1,1)\n  box a at (0,0) size (1,1) { text \"x\" }\n  box b at (0,0) size (1,1)\n}\nbox c at (0,0) size (1,1)\n",
    );
    let codes: Vec<_> = diags.iter().map(|d| d.code).collect();
    assert_eq!(codes, vec!["unsupported"; 1], "{diags:#?}");
    let Stmt::Group(g) = &f.stmts[0] else {
        panic!("expected group")
    };
    assert_eq!(g.children.len(), 2);
    assert_eq!(f.stmts.len(), 2);
}

#[test]
fn unclosed_unbuilt_block_is_reported() {
    let (_, diags) = parse("shape s {\n  path \"M 0 0\"\n");
    assert_eq!(diags[0].code, "unsupported");
    assert!(diags.iter().any(|d| d.message.contains("expected `}`")));
}

#[test]
fn pin_with_position_parses() {
    let f = parse_ok("box a at (0,0) size (1,1) pin=left@0.25 style=service\n");
    let Stmt::Box(n) = &f.stmts[0] else {
        panic!("expected box")
    };
    assert_eq!(n.attrs[0].value, Value::AnchorAt("left".into(), 0.25));
    assert_eq!(n.attrs.len(), 2);
}

#[test]
fn flag_lines_continue_a_line() {
    let f = parse_ok("line a.top\n  -> y 40\n  ortho\nline (0,0) -> h 5\n\n  ortho end=arrow\n  label=\"x\"\nbox c at (0,0) size (1,1)\n");
    assert_eq!(f.stmts.len(), 3);
    let Stmt::Line(l) = &f.stmts[0] else {
        panic!("expected line")
    };
    assert_eq!(l.attrs[0].key.name, "ortho");
    let Stmt::Line(l) = &f.stmts[1] else {
        panic!("expected line")
    };
    let keys: Vec<_> = l.attrs.iter().map(|a| a.key.name.as_str()).collect();
    assert_eq!(keys, ["ortho", "end", "label"]);
}

#[test]
fn flag_lines_do_not_continue_other_statements() {
    let (_, diags) = parse("box a at (0,0) size (1,1)\n  ortho\n");
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert_eq!(diags[0].code, "parse");
}

fn nested_groups(depth: usize, close: bool) -> String {
    let mut s = String::new();
    for i in 0..depth {
        s.push_str(&format!("group g{i} at (0,0) size (1,1) {{\n"));
    }
    if close {
        s.push_str(&"}\n".repeat(depth));
    }
    s
}

#[test]
fn group_nesting_is_limited() {
    parse_ok(&nested_groups(64, true));
    for close in [true, false] {
        let (f, diags) = parse(&nested_groups(20_000, close));
        assert!(
            diags
                .iter()
                .any(|d| d.code == "parse" && d.message.contains("nested more than 64")),
            "{:#?}",
            &diags[..diags.len().min(3)]
        );
        assert_eq!(f.stmts.len(), 1);
    }
    let (f, diags) = parse(&format!(
        "{}box after at (0,0) size (1,1)\n",
        nested_groups(70, true)
    ));
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert_eq!(f.stmts.len(), 2);
}

#[test]
fn unknown_string_escapes_are_parse_errors() {
    let (_, d) = parse("text \"a\\qb\" at (0,0)\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "parse");
    assert!(d[0].message.contains("`\\q`"), "{}", d[0].message);
    parse_ok("text \"\\\" \\\\ \\n \\* \\`\" at (0,0)\n");
}

#[test]
fn leading_byte_order_mark_is_skipped() {
    let f = parse_ok("\u{feff}diagram \"t\" size (10,10)\n");
    assert_eq!(f.stmts.len(), 1);
}

#[test]
fn segment_keywords_are_reserved_ids() {
    for kw in ["h", "v", "x", "y", "hv", "vh", "curve"] {
        for src in [
            format!("box {kw} at (0,0) size (1,1)\n"),
            format!("group {kw} at (0,0) size (1,1) {{\n}}\n"),
            format!("junction {kw} at (0,0)\n"),
            format!("text {kw} \"t\" at (0,0)\n"),
            format!("line {kw}: (0,0) -> h 1\n"),
        ] {
            let (_, d) = parse(&src);
            assert_eq!(d.len(), 1, "{src}: {d:#?}");
            assert_eq!(d[0].code, "parse");
            assert!(d[0].message.contains("reserved"), "{}", d[0].message);
            assert!(!d[0].help.is_empty());
        }
    }
    parse_ok("box hx at (0,0) size (1,1)\njunction curves at (0,0)\n");
}

fn block(f: &File, i: usize) -> &Vec<BlockText> {
    let Stmt::Box(b) = &f.stmts[i] else {
        panic!("expected box")
    };
    b.block.as_ref().expect("text block")
}

#[test]
fn box_text_block_parses_stacked_and_absolute_items() {
    let f = parse_ok(
        "box api at (100,80) size (200,120) text-align=left {\n  text \"**Payments API**\"\n  text \"v2.4.1\" size=11 muted\n\n  text \"note\" at (10,90) align=right\n}\nbox b at (0,0) size (1,1)\n",
    );
    assert_eq!(f.stmts.len(), 2);
    let items = block(&f, 0);
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].content, "**Payments API**");
    assert!(items[0].at.is_none());
    assert_eq!(items[1].attrs.len(), 2);
    assert_eq!(items[1].attrs[1].value, Value::Flag);
    assert!(items[2].at.is_some());
    assert_eq!(items[2].span.line, 5);
}

#[test]
fn one_line_and_empty_text_blocks() {
    let f = parse_ok("box a at (0,0) size (9,9) { text \"x\" }\nbox b at (0,0) size (9,9) {\n}\nbox c at (0,0) size (9,9)\n");
    assert_eq!(block(&f, 0).len(), 1);
    assert!(block(&f, 1).is_empty());
    let Stmt::Box(c) = &f.stmts[2] else {
        panic!("expected box")
    };
    assert!(c.block.is_none());
}

#[test]
fn text_block_rejects_ids_and_other_statements() {
    let (f, diags) = parse(
        "box a at (0,0) size (9,9) {\n  text t \"x\"\n  box b at (0,0) size (1,1)\n  text \"ok\"\n}\nbox c at (0,0) size (1,1)\n",
    );
    let msgs: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        msgs,
        vec![
            "text block items take no id",
            "expected `text` or `}` in a text block, found `box`"
        ],
        "{diags:#?}"
    );
    assert_eq!(block(&f, 0).len(), 1);
    assert_eq!(f.stmts.len(), 2);
}

#[test]
fn unclosed_text_block_is_reported() {
    let (_, diags) = parse("box a at (0,0) size (9,9) {\n  text \"x\"\n");
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert_eq!(diags[0].message, "expected `}`, found end of file");
}
