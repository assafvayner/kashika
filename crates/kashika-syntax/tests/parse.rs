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
