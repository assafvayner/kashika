use kashika_syntax::ast::*;
use kashika_syntax::parse;

fn parse_ok(src: &str) -> File {
    let (file, diags) = parse(src);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:#?}");
    file
}

fn sized(w: f64, h: f64) -> Option<Size> {
    Some(Size {
        w: Some(w),
        h: Some(h),
    })
}

#[test]
fn parses_diagram_and_box() {
    let f = parse_ok(
        "diagram \"T\" size (400,300) grid=10\nbox api \"API\" at (10,20) size (120,60) style=service\n",
    );
    assert_eq!(f.stmts.len(), 2);
    let Stmt::Box(b) = &f.stmts[1] else {
        panic!("expected box")
    };
    assert_eq!(b.id.name, "api");
    assert_eq!(b.label.as_deref(), Some("API"));
    assert_eq!(b.size, sized(120.0, 60.0));
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
        "line l1 a.right -> h 10 -> v -20 -> x 50 -> y b.top -> hv c.left@0.25 -> vh (1,2) \
         -> vhv g.top -> hvh@0.25 (3,4) -> curve left 30 d.top -> curve via (0,0) (5,5) e.bottom -> f.left+(0,-4) end=arrow ortho\n",
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
            SegKind::Vhv { .. } => "vhv",
            SegKind::Hvh { .. } => "hvh",
            SegKind::Curve { .. } => "curve",
            SegKind::CurveVia { .. } => "via",
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "h", "v", "x", "y", "hv", "vh", "vhv", "hvh", "curve", "via", "to"
        ]
    );
    assert_eq!(l.segs[1].kind, SegKind::V(-20.0));
    assert!(matches!(&l.segs[6].kind, SegKind::Vhv { t: None, .. }));
    assert!(matches!(&l.segs[7].kind, SegKind::Hvh { t: Some(t), .. } if *t == 0.25));
    assert!(
        matches!(&l.segs[8].kind, SegKind::Curve { side: Side::Left, amount, .. } if *amount == 30.0)
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
    assert_eq!(
        *offset,
        Some(Offset {
            neg: false,
            dx: 0.0,
            dy: -10.0
        })
    );
}

#[test]
fn continues_lines_starting_with_arrow_or_attribute() {
    let f = parse_ok(
        "line a.top\n  -> y 40\n  -> hv b.left\n  end=arrow label=\"SQL\"\nbox c at (0,0) size (1,1)\n",
    );
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
    let (f, diags) = parse(
        "bx a at (0,0) size (1,1)\nbox b at (0,0) size (1,1)\nline c.top ->\nbox d at (0,0) size (1,1)\n",
    );
    assert_eq!(diags.len(), 2, "{diags:#?}");
    assert!(
        diags[0]
            .help
            .iter()
            .any(|h| h.contains("did you mean `box`?"))
    );
    assert!(diags[1].message.contains("must be followed by a segment"));
    assert_eq!(f.stmts.len(), 2);
}

#[test]
fn unclosed_group_is_reported() {
    let (_, diags) = parse("group g at (0,0) size (1,1) {\n box a at (0,0) size (1,1)\n");
    assert!(diags.iter().any(|d| d.message.contains("expected `}`")));
}

#[test]
fn failed_statements_record_their_ids() {
    let (f, diags) = parse(
        "diagram \"t\" size (1\nbox a at (0,0) size (1\njunction j at\ntext t \"x\" at\n\
         line l a.right ->\nimage i \"x.png\" at\nbox at (0,0) size (1,1)\nbox h at (0,0)\n\
         group g at (0,0) size (9,9) {\n  box inner at\n  box ok at (0,0) size (1,1)\n} junk\n\
         box fine at (0,0) size (1,1)\n",
    );
    assert!(!diags.is_empty());
    let names: Vec<&str> = f.failed_ids.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["a", "j", "t", "l", "i", "at", "inner", "g", "ok"]);
    assert!(f.failed_diagram);
    assert_eq!(f.stmts.len(), 1);
    let (f, _) = parse("diagram \"t\" size (1,1)\nbox a at\n");
    assert!(!f.failed_diagram);
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
    let f = parse_ok(
        "box _a1 at (0,0) size (1,1)\nbox a--b at (0,0) size (1,1)\nbox c- at (0,0) size (1,1)\nline a--b.right->c-.left\n",
    );
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
    assert_eq!((*x, *y, b.size), (-1.5, 20.0, sized(0.25, 3.0)));
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
    let f = parse_ok(
        "line a.top\n\n  // first hop\n  -> y 40\n  /* second\n     hop */\n  -> h 10 // trailing\n\n  end=arrow\nbox c at (0,0) size (1,1)\n",
    );
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
    let f = parse_ok(
        "line l (0,0) -> h 5 start=dot end=arrow stroke=#000 stroke-width=2 stroke-style=dashed style=s radius=4 label=\"a\" label@0.25=\"b\" ortho allow=cross reason=\"r\"\n",
    );
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
    let f = parse_ok(
        "group a at (0,0) size (9,9) {\n  group b at (0,0) size (5,5) {\n    junction j at (0,0)\n    text \"t\" at (0,0)\n    line (0,0) -> h 1\n  }\n}\n",
    );
    let Stmt::Group(a) = &f.stmts[0] else {
        panic!("expected group")
    };
    let Stmt::Group(b) = &a.children[0] else {
        panic!("expected group")
    };
    assert_eq!(b.children.len(), 3);
}

#[test]
fn parses_imports_shapes_and_images() {
    let f = parse_ok(
        "import \"lib/aws.kash\" as aws\nimport \"base.kash\"\n\
         shape lb {\n  path \"M 0 0 L 1 1\" // diagonal\n  outline 1\n  text-area (0.2,0.3,0.6,0.4)\n}\n\
         shape s3 from \"icons/s3.svg\" outline=hull text-area=below\n\
         image logo \"hf.png\" at (20,20) size (80,80) fit=contain\n\
         box b at (0,0) size (1,1) shape=aws.s3 style=aws.service\n",
    );
    assert!(
        matches!(&f.stmts[0], Stmt::Import(i) if i.path == "lib/aws.kash" && i.ns.as_ref().unwrap().name == "aws")
    );
    assert!(matches!(&f.stmts[1], Stmt::Import(i) if i.ns.is_none()));
    let Stmt::Shape(s) = &f.stmts[2] else {
        panic!("expected shape")
    };
    assert_eq!(s.name.name, "lb");
    let ShapeBody::Paths {
        path,
        outline,
        text_area,
    } = &s.body
    else {
        panic!("expected a path body")
    };
    assert_eq!(path.0, "M 0 0 L 1 1");
    assert_eq!(outline.map(|o| o.0), Some(1.0));
    assert_eq!(text_area.map(|t| t.0), Some([0.2, 0.3, 0.6, 0.4]));
    let Stmt::Shape(s) = &f.stmts[3] else {
        panic!("expected shape")
    };
    assert!(
        matches!(&s.body, ShapeBody::From { file, attrs, .. } if file == "icons/s3.svg" && attrs.len() == 2)
    );
    assert!(
        matches!(&f.stmts[4], Stmt::Image(i) if i.id.name == "logo" && i.file == "hf.png" && i.size == (80.0, 80.0) && i.attrs.len() == 1)
    );
    let Stmt::Box(b) = &f.stmts[5] else {
        panic!("expected box")
    };
    assert_eq!(b.attrs[0].value, Value::Word("aws.s3".into()));
    assert_eq!(b.attrs[1].value, Value::Word("aws.service".into()));
}

#[test]
fn shape_body_errors_recover() {
    let (f, d) = parse(
        "shape a {\n  pth \"M 0 0\"\n}\nshape b {\n  outline 1\n}\nshape c {\n  path \"M 0 0\"\n  path \"M 1 1\"\n}\nbox z at (0,0) size (1,1)\n",
    );
    let got: Vec<_> = d.iter().map(|x| (x.code, x.span.line)).collect();
    assert_eq!(
        got,
        vec![("parse", 2), ("parse", 4), ("parse", 9)],
        "{d:#?}"
    );
    assert_eq!(d[0].help, vec!["did you mean `path`?".to_string()]);
    assert!(d[1].message.contains("needs a `path`"), "{}", d[1].message);
    assert!(matches!(f.stmts.last(), Some(Stmt::Box(_))));
    assert!(matches!(&f.stmts[0], Stmt::Shape(s) if s.name.name == "c"));
}

#[test]
fn image_needs_a_file_string() {
    let (_, d) = parse("image logo at (0,0) size (1,1)\n");
    assert_eq!(d[0].code, "parse");
    assert!(
        d[0].message.contains("a file path string"),
        "{}",
        d[0].message
    );
}

#[test]
fn image_inside_a_group_parses() {
    let (f, diags) = parse(
        "group g at (0,0) size (9,9) {\n  image i \"x.png\" at (0,0) size (1,1)\n  box a at (0,0) size (1,1) { text \"x\" }\n  box b at (0,0) size (1,1)\n}\nbox c at (0,0) size (1,1)\n",
    );
    assert!(diags.is_empty(), "{diags:#?}");
    let Stmt::Group(g) = &f.stmts[0] else {
        panic!("expected group")
    };
    assert_eq!(g.children.len(), 3);
    assert!(matches!(&g.children[0], Stmt::Image(_)));
    assert_eq!(f.stmts.len(), 2);
}

#[test]
fn unclosed_shape_block_is_reported() {
    let (f, diags) = parse("shape s {\n  path \"M 0 0\"\n");
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert_eq!(diags[0].code, "parse");
    assert!(diags[0].message.contains("expected `}`"), "{diags:#?}");
    assert!(f.stmts.is_empty());
}

#[test]
fn dotted_words_need_an_identifier_after_the_dot() {
    let (_, d) = parse("box b at (0,0) size (1,1) style=aws.\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "parse");
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
    let f = parse_ok(
        "line a.top\n  -> y 40\n  ortho\nline (0,0) -> h 5\n\n  ortho end=arrow\n  label=\"x\"\nbox c at (0,0) size (1,1)\n",
    );
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
            format!("line {kw} (0,0) -> h 1\n"),
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
    let f = parse_ok(
        "box a at (0,0) size (9,9) { text \"x\" }\nbox b at (0,0) size (9,9) {\n}\nbox c at (0,0) size (9,9)\n",
    );
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

#[test]
fn reserved_id_statements_are_dropped() {
    let (f, d) = parse(
        "box h at (0,0) size (1,1)\ngroup v at (0,0) size (9,9) {\n  box inner at (1,1) size (1,1)\n}\njunction x at (0,0)\ntext y \"t\" at (0,0)\nline hv (0,0) -> h 1\nimage vh \"a.png\" at (0,0) size (1,1)\nbox ok at (0,0) size (1,1)\n",
    );
    assert_eq!(d.len(), 6, "{d:#?}");
    assert_eq!(f.stmts.len(), 1, "{:#?}", f.stmts);
    let Stmt::Box(b) = &f.stmts[0] else {
        panic!("expected box")
    };
    assert_eq!(b.id.name, "ok");
}

#[test]
fn hash_comment_is_one_error_with_help() {
    for src in [
        "# a note\nbox a at (0,0) size (1,1)\n",
        "#note\nbox a at (0,0) size (1,1)\n",
        "  # indented\nbox a at (0,0) size (1,1)\n",
    ] {
        let (f, d) = parse(src);
        assert_eq!(d.len(), 1, "{src:?}: {d:#?}");
        assert_eq!(d[0].code, "parse");
        assert!(d[0].message.contains("comment"), "{}", d[0].message);
        assert!(d[0].help.iter().any(|h| h.contains("`//`")), "{d:#?}");
        assert_eq!(f.stmts.len(), 1);
    }
    let (_, d) = parse("box a at (0,0) size (1,1) # trailing\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(d[0].help.iter().any(|h| h.contains("`//`")), "{d:#?}");
}

#[test]
fn at_size_and_attributes_come_in_any_order() {
    let f = parse_ok(
        "diagram \"t\" grid=10 size (400,300) clearance=4\n\
         box a \"A\" sublabel=\"v2\" at (20,20) size (100,60)\n\
         box b size (10,20) style=service at (1,2) pin=tl\n\
         group g \"G\" size (90,90) at (0,0) {\n}\n\
         junction j dot at (5,5)\n\
         text \"t\" size=12 muted at (3,4)\n\
         image i \"a.png\" fit=cover size (8,8) at (0,0)\n\
         box c at (0,0) size (9,9) {\n  text \"x\" muted at (1,1)\n}\n",
    );
    assert_eq!(f.stmts.len(), 8);
    let Stmt::Diagram(d) = &f.stmts[0] else {
        panic!("expected diagram")
    };
    assert_eq!(d.size, sized(400.0, 300.0));
    assert_eq!(d.attrs.len(), 2);
    let Stmt::Box(b) = &f.stmts[2] else {
        panic!("expected box")
    };
    assert_eq!(b.size, sized(10.0, 20.0));
    assert_eq!(b.attrs.len(), 2);
    assert!(matches!(b.at, Point::Lit { x: Coord::Num(x), .. } if x == 1.0));
    let Stmt::Junction(j) = &f.stmts[4] else {
        panic!("expected junction")
    };
    assert_eq!(j.attrs[0].value, Value::Flag);
    let Stmt::Text(t) = &f.stmts[5] else {
        panic!("expected text")
    };
    assert_eq!(t.attrs.len(), 2);
    assert!(block(&f, 7)[0].at.is_some());
}

#[test]
fn duplicate_or_missing_at_and_size_are_parse_errors() {
    for (src, msg) in [
        ("box a at (0,0) size (1,1) at (2,2)\n", "duplicate `at`"),
        ("box a size (1,1) at (0,0) size (2,2)\n", "duplicate `size`"),
        ("diagram \"t\" size (1,1) size (2,2)\n", "duplicate `size`"),
        ("junction j at (0,0) at (1,1)\n", "duplicate `at`"),
        ("box a fill=red size (1,1)\n", "expected `at`"),
        ("image i \"x.png\" at (0,0) fit=cover\n", "expected `size`"),
    ] {
        let (f, d) = parse(src);
        assert_eq!(d.len(), 1, "{src:?}: {d:#?}");
        assert_eq!(d[0].code, "parse");
        assert!(d[0].message.contains(msg), "{src:?}: {}", d[0].message);
        assert!(f.stmts.is_empty(), "{src:?}");
    }
}

#[test]
fn box_size_is_optional_and_takes_auto() {
    let f = parse_ok(
        "box a \"A\" at (0,0)\nbox b \"B\" at (0,0) size (auto, 40)\nbox c \"C\" size (auto,auto) at (0,0)\n",
    );
    let sizes: Vec<Option<Size>> = f
        .stmts
        .iter()
        .map(|s| match s {
            Stmt::Box(b) => b.size,
            _ => panic!("expected box"),
        })
        .collect();
    let auto = |w, h| Some(Size { w, h });
    assert_eq!(sizes, [None, auto(None, Some(40.0)), auto(None, None)]);
}

#[test]
fn auto_size_is_for_boxes_groups_and_the_diagram() {
    for (src, size) in [
        ("diagram \"t\"\n", None),
        (
            "diagram \"t\" size (auto,100)\n",
            Some(Size {
                w: None,
                h: Some(100.0),
            }),
        ),
    ] {
        let Stmt::Diagram(d) = &parse_ok(src).stmts[0] else {
            panic!("expected diagram")
        };
        assert_eq!(d.size, size);
    }
    let (f, d) = parse("image i \"x.png\" at (0,0) size (auto,10)\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "parse");
    assert!(f.stmts.is_empty());
    let Stmt::Group(g) = &parse_ok("group g at (0,0) size (10,auto) {\n}\n").stmts[0] else {
        panic!("expected group")
    };
    assert_eq!(
        g.node.size,
        Some(Size {
            w: Some(10.0),
            h: None
        })
    );
    let Stmt::Group(g) = &parse_ok("group g at (0,0) style=wide {\n}\n").stmts[0] else {
        panic!("expected group")
    };
    assert_eq!(g.node.size, None);
}

#[test]
fn size_attribute_takes_a_size() {
    let f = parse_ok("style s { size=(120, auto) font-size=12 }\ntext \"t\" at (0,0) size=12\n");
    let Stmt::Style(s) = &f.stmts[0] else {
        panic!("expected style")
    };
    assert_eq!(
        s.attrs[0].value,
        Value::Size(Size {
            w: Some(120.0),
            h: None
        })
    );
    let Stmt::Text(t) = &f.stmts[1] else {
        panic!("expected text")
    };
    assert_eq!(t.attrs[0].value, Value::Num(12.0));
}

fn line_stmt(f: &File) -> &Line {
    let Stmt::Line(l) = &f.stmts[0] else {
        panic!("expected line")
    };
    l
}

#[test]
fn arrow_is_a_line_statement() {
    let f = parse_ok("arrow ab a.right -> b.left label=\"x\"\narrow a.bottom -> v 20 end=none\n");
    let lines: Vec<&Line> = f
        .stmts
        .iter()
        .map(|s| match s {
            Stmt::Line(l) => l,
            _ => panic!("expected line"),
        })
        .collect();
    assert!(lines.iter().all(|l| l.arrow));
    assert_eq!(lines[0].id.as_ref().map(|i| i.name.as_str()), Some("ab"));
    assert_eq!(lines[1].id, None);
    assert_eq!(lines[1].attrs[0].key.name, "end");
    assert!(!line_stmt(&parse_ok("line a.right -> b.left\n")).arrow);
}

#[test]
fn arrow_is_reserved() {
    let h = helps("box arrow at (0,0) size (1,1)\n");
    assert_eq!(
        h,
        [(
            "`arrow` is a reserved word and cannot be an id".to_string(),
            "`arrow` is a statement keyword; pick another name, e.g. `arrow_1`".to_string()
        )]
    );
    let h = helps("arrow ab: a.right -> b.left\n");
    assert_eq!(h[0].1, "remove the `:`: write `arrow ab a.right -> b.left`");
}

fn helps(src: &str) -> Vec<(String, String)> {
    parse(src)
        .1
        .into_iter()
        .map(|d| (d.message, d.help.join("; ")))
        .collect()
}

#[test]
fn line_id_is_an_identifier_before_a_point() {
    let f = parse_ok(
        "line l1 a.right -> b.left
line l2 (0,0) -> h 10
line a -> b
line a.top -> b
line (0,0) -> b
",
    );
    let ids: Vec<Option<String>> = f
        .stmts
        .iter()
        .map(|s| match s {
            Stmt::Line(l) => l.id.as_ref().map(|i| i.name.clone()),
            _ => panic!("expected line"),
        })
        .collect();
    assert_eq!(
        ids,
        vec![Some("l1".into()), Some("l2".into()), None, None, None]
    );
}

#[test]
fn offsets_keep_their_sign() {
    for (src, neg, dx, delta) in [
        ("line a.top+(-10,0) -> h 1\n", false, -10.0, (-10.0, 0.0)),
        ("line a.top-(10,0) -> h 1\n", true, 10.0, (-10.0, 0.0)),
        ("line a.top-(-10,0) -> h 1\n", true, -10.0, (10.0, 0.0)),
        ("line a-b.left@0.25-(0,4) -> h 1\n", true, 0.0, (0.0, -4.0)),
    ] {
        let f = parse_ok(src);
        let Point::Ref { offset, .. } = &line_stmt(&f).start else {
            panic!("expected ref")
        };
        let o = offset.unwrap();
        assert_eq!((o.neg, o.dx), (neg, dx), "{src}");
        assert_eq!(o.delta(), delta, "{src}");
    }
}

#[test]
fn old_line_id_colon_is_an_error_with_help() {
    let (f, d) = parse("line l1: a.right -> b.left end=arrow\n");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].message, "a line id takes no `:`");
    assert_eq!(
        d[0].help,
        ["remove the `:`: write `line l1 a.right -> b.left end=arrow`"]
    );
    assert_eq!(line_stmt(&f).id.as_ref().unwrap().name, "l1");
}

#[test]
fn from_to_lines_are_errors_with_help() {
    let want = "write `line l1 a.right -> b.left`";
    for src in [
        "line l1 from a.right to b.left\n",
        "line l1: from a.right to b.left\n",
    ] {
        assert_eq!(
            helps(src),
            vec![(
                "a line is written with `->`, not `from … to`".to_string(),
                want.to_string()
            )],
            "{src}"
        );
    }
    assert_eq!(
        helps("line from a.right to (1,2)\n")[0].1,
        "write `line a.right -> (1,2)`"
    );
}

#[test]
fn old_curve_colon_is_an_error_with_help() {
    let (f, d) = parse("line a.right -> curve left:30 b.top+(0,-4) -> h 10\n");
    assert_eq!(d.len(), 1);
    assert_eq!(
        d[0].help,
        ["remove the `:`: write `curve left 30 b.top+(0,-4)`"]
    );
    assert!(matches!(
        line_stmt(&f).segs[0].kind,
        SegKind::Curve {
            side: Side::Left,
            amount,
            ..
        } if amount == 30.0
    ));
}

#[test]
fn kashika_version_statement_is_an_error_with_help() {
    let (f, d) = parse("kashika=\"0.1.0\"\ndiagram \"t\" size (10,10)\n");
    assert_eq!(d.len(), 1);
    assert_eq!(
        d[0].help,
        [
            "the version is an attribute of `diagram`: write `diagram \"…\" size (w,h) kashika=\"0.1.0\"`"
        ]
    );
    assert_eq!(f.first_stmt.map(|s| s.line), Some(2));
}

#[test]
fn bom_does_not_shift_help_text() {
    let (_, diags) = parse("\u{feff}kashika==\"0.1\"\n");
    assert_eq!(diags.len(), 1, "{diags:#?}");
    let (_, plain) = parse("kashika==\"0.1\"\n");
    assert_eq!(diags[0].help, plain[0].help);
}

fn box_at(src: &str) -> Point {
    let f = parse_ok(src);
    let Stmt::Box(b) = &f.stmts[0] else {
        panic!("expected box")
    };
    b.at.clone()
}

#[test]
fn parenthesized_points_mean_the_bare_point() {
    for (grouped, bare) in [
        ("(a.center)", "a.center"),
        ("(a.center+(0,40))", "a.center+(0,40)"),
        ("(db.top-(10,0))", "db.top-(10,0)"),
        ("(a.left@0.25+(4,0))", "a.left@0.25+(4,0)"),
        ("(fan)", "fan"),
        ("(fan+(0,10))", "fan+(0,10)"),
    ] {
        let g = box_at(&format!("box b at {grouped} size (1,1)\n"));
        let b = box_at(&format!("box b at  {bare} size (1,1)\n"));
        assert!(matches!(g, Point::Group { .. }), "{grouped}");
        let strip = |p: &Point| match p {
            Point::Lit { x, y, .. } => format!("{x:?} {y:?}"),
            Point::Ref { r, offset, .. } => format!("{r:?} {offset:?}"),
            Point::Group { .. } | Point::Shifted { .. } => panic!("still grouped"),
        };
        assert_eq!(strip(g.ungrouped()), strip(&b), "{grouped}");
    }
}

#[test]
fn parentheses_nest() {
    let Point::Group { inner, span } = box_at("box b at (a.center) size (1,1)\n") else {
        panic!("expected group")
    };
    assert_eq!((span.col, span.end_col), (10, 20));
    assert!(matches!(*inner, Point::Ref { .. }));
    for src in [
        "((a.center))",
        "(((a.center)))",
        "((a.center+(0,4)))",
        "((10,20))",
        "(( 10,20 ))",
        "((a.left.x, b.y))",
        "((a.left.x, b.y)+(0,4))",
        "((a.center)+(0,4))",
    ] {
        let p = box_at(&format!("box b at {src} size (1,1)\n"));
        assert!(matches!(p, Point::Group { .. }), "{src}");
    }
    let Point::Group { inner, .. } = box_at("box b at ((a.left.x, b.y)+(0,4)) size (1,1)\n") else {
        panic!("expected group")
    };
    let Point::Shifted { base, offsets, .. } = *inner else {
        panic!("expected offsets")
    };
    assert!(matches!(*base, Point::Lit { .. }));
    assert_eq!((offsets[0].dx, offsets[0].dy), (0.0, 4.0));
}

#[test]
fn offsets_follow_any_point() {
    for (src, base, n) in [
        ("(10,20)+(5,0)", "lit", 1),
        ("(a.left.x, b.y)+(0,26)", "lit", 1),
        ("(a.center)+(0,4)", "group", 1),
        ("a.center+(1,2)+(3,4)", "ref", 1),
        ("(a.center+(0,4))+(2,0)", "group", 1),
        ("(a.center)-(1,2)+(3,4)", "group", 2),
    ] {
        let Point::Shifted {
            base: b, offsets, ..
        } = box_at(&format!("box b at {src} size (1,1)\n"))
        else {
            panic!("{src}: expected offsets")
        };
        let kind = match *b {
            Point::Lit { .. } => "lit",
            Point::Ref { .. } => "ref",
            Point::Group { .. } => "group",
            Point::Shifted { .. } => "shifted",
        };
        assert_eq!((kind, offsets.len()), (base, n), "{src}");
    }
    let f = parse_ok("line l (a.center)-(0,4) -> (10,20)+(5,0) -> b.top+(1,1)-(2,2)\n");
    let l = line_stmt(&f);
    let Point::Shifted { offsets, .. } = &l.start else {
        panic!("expected offsets")
    };
    assert!(offsets[0].neg);
    assert_eq!(l.segs.len(), 2);
}

#[test]
fn parentheses_nest_at_most_16_deep() {
    let at = |n: usize| {
        format!(
            "box b at {}a.center{} size (1,1)\n",
            "(".repeat(n),
            ")".repeat(n)
        )
    };
    parse_ok(&at(16));
    parse_ok("box b at ((((((((((((((((10,20)))))))))))))))) size (1,1)\n");
    let (_, d) = parse(&at(17));
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "a point nests at most 16 sets of parentheses");
    assert_eq!((d[0].span.col, d[0].span.end_col), (26, 36));
    assert_eq!(
        d[0].help,
        ["parentheses around a point change nothing; remove the extra ones"]
    );
}

#[test]
fn deeply_nested_point_parentheses_are_one_error() {
    for close in [false, true] {
        let n = 5000;
        let src = format!(
            "box b at {}a.center{} size (1,1)\nbox c at (0,0) size (1,1)\n",
            "(".repeat(n),
            if close { ")".repeat(n) } else { String::new() }
        );
        let (f, d) = parse(&src);
        assert_eq!(d.len(), 1, "{d:#?}");
        assert_eq!(d[0].message, "a point nests at most 16 sets of parentheses");
        assert_eq!(f.stmts.len(), 1);
    }
}

#[test]
fn parenthesized_points_in_line_endpoints_and_segments() {
    let f = parse_ok(
        "line l (a.bottom) -> (b.top-(0,4)) -> x (c.left) -> y (lane) -> hv (d.top) \
         -> vh (e.left) -> curve left 20 (f.top) -> curve via (g.center) (h.center) (i.right)\n",
    );
    let l = line_stmt(&f);
    assert!(matches!(l.start, Point::Group { .. }));
    let points: Vec<&Point> = l
        .segs
        .iter()
        .flat_map(|s| match &s.kind {
            SegKind::To(p) | SegKind::Hv(p) | SegKind::Vh(p) | SegKind::Curve { to: p, .. } => {
                vec![p]
            }
            SegKind::X(Target::Point(p)) | SegKind::Y(Target::Point(p)) => vec![p],
            SegKind::CurveVia { c1, c2, to } => vec![c1, c2, to],
            _ => vec![],
        })
        .collect();
    assert_eq!(points.len(), 9);
    assert!(points.iter().all(|p| matches!(p, Point::Group { .. })));
    assert!(matches!(
        points[1].ungrouped(),
        Point::Ref { offset: None, r, .. } if r.anchor.as_ref().unwrap().name.name == "left"
    ));
}

#[test]
fn literal_and_mixed_points_are_not_groups() {
    assert!(matches!(
        box_at("box b at (10,20) size (1,1)\n"),
        Point::Lit { .. }
    ));
    let Point::Lit { x, y, .. } = box_at("box b at (a.left.x, lane.y) size (1,1)\n") else {
        panic!("expected mixed point")
    };
    assert!(matches!(x, Coord::Axis { axis: Axis::X, .. }));
    assert!(matches!(y, Coord::Axis { axis: Axis::Y, .. }));
    let (_, d) = parse("box b at (a.left, lane.y) size (1,1)\n");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].message, "expected `.`, found `,`");
}

#[test]
fn unclosed_point_paren_is_an_error_with_help() {
    let (_, d) = parse("box b at (a.center+(0,40) size (1,1)\n");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "parse");
    assert_eq!(
        d[0].message,
        "expected `)` to close the `(` around the point, found `size`"
    );
    assert_eq!((d[0].span.line, d[0].span.col), (1, 27));
    assert_eq!(d[0].help, ["close it: `(a.center+(0,40))`"]);
    let (_, d) = parse("line l (a.center -> b.top\n");
    assert_eq!(d.len(), 1);
    assert_eq!(
        d[0].message,
        "expected `)` to close the `(` around the point, found `->`"
    );
    let (_, d) = parse("box b at (a.left.x) size (1,1)\n");
    assert_eq!(d.len(), 1);
    assert_eq!(
        d[0].help,
        ["`.x`/`.y` take one axis of a mixed point, which needs both: `(a.left.x, other.y)`"]
    );
}

#[test]
fn elbow_pair_position_is_inside_0_1() {
    for (src, msg) in [
        (
            "vhv@0 b.top",
            "`vhv@t` needs t between 0 and 1 (exclusive), found 0",
        ),
        (
            "hvh@1.5 b.top",
            "`hvh@t` needs t between 0 and 1 (exclusive), found 1.5",
        ),
        (
            "vhv@-0.2 b.top",
            "`vhv@t` needs t between 0 and 1 (exclusive), found -0.2",
        ),
    ] {
        let (f, d) = parse(&format!("line l a.bottom -> {src}\n"));
        assert_eq!(d.len(), 1, "{src}: {d:#?}");
        assert_eq!((d[0].code, d[0].message.as_str()), ("parse", msg));
        assert!(f.stmts.is_empty());
    }
    let (_, d) = parse("box vhv at (0,0) size (1,1)\n");
    assert_eq!(d[0].message, "`vhv` is a reserved word and cannot be an id");
}
