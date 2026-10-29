mod common;

use common::*;
use kashika_check::ctx::{Ctx, anchor_text, overflow};
use kashika_check::outline::polygon;
use kashika_check::ports::ports;
use kashika_check::text_box::{label_box, label_box_at, text_box, title_band, title_box};
use kashika_check::{check, metrics, suppress};
use kashika_diag::{Diagnostic, Element, Span};
use kashika_geom::{Pt, Rect, boundary_nearest, point_in_polygon};

fn diag(code: &'static str, ids: &[&str]) -> Diagnostic {
    ids.iter()
        .fold(Diagnostic::warn(code, Span::default(), "x"), |d, id| {
            d.with_element(Element::new(*id, "line", Span::default()))
        })
}

#[test]
fn clean_scene_has_no_findings() {
    assert!(check(&scene("box a at (10,10) size (100,60)\n")).is_empty());
}

#[test]
fn line_allow_suppresses_matching_findings_and_keeps_the_reason() {
    let s = scene(
        "line a (0,0) -> h 10 allow=cross:b reason=\"bus\"\nline b (5,-5) -> v 10\nline c (0,20) -> h 10\n",
    );
    let mut ds = vec![
        diag("cross", &["b", "a"]),
        diag("cross", &["c", "a"]),
        diag("cross", &["b", "c"]),
    ];
    suppress(&s, &mut ds);
    assert!(ds[0].suppressed);
    assert_eq!(ds[0].reason.as_deref(), Some("bus"));
    assert!(!ds[1].suppressed);
    assert!(!ds[2].suppressed);
}

#[test]
fn shape_and_diagram_allow() {
    let s = scene_src(
        "diagram \"t\" size (100,100) allow=near\nbox a at (0,0) size (10,10) allow=overlap\nline l (0,50) -> h 10 allow=cross\n",
    );
    let mut ds = vec![
        diag("near", &["l"]),
        diag("overlap", &["a", "z"]),
        diag("cross", &["l", "m"]),
        diag("through", &["l", "a"]),
    ];
    suppress(&s, &mut ds);
    let flags: Vec<bool> = ds.iter().map(|d| d.suppressed).collect();
    assert_eq!(flags, [true, true, true, false]);
}

#[test]
fn metrics_count_crossings_and_wire() {
    let s = scene("line a (0,0) -> h 10\nline b (0,20) -> h 30\n");
    let m = metrics(&s, &[diag("cross", &["a", "b"]), diag("near", &["a"])]);
    assert_eq!((m.crossings, m.wire_px), (1, 40));
}

#[test]
fn outlines_and_attachment() {
    let s = scene("box r at (0,0) size (100,60)\nbox q at (200,0) size (100,60) shape=rounded\n");
    let ctx = Ctx::new(&s);
    assert_eq!(polygon(&s.shapes[0]).len(), 4);
    let q = polygon(&s.shapes[1]);
    assert!(!point_in_polygon(Pt::new(200.5, 0.5), &q));
    assert!(boundary_nearest(Pt::new(200.0, 30.0), &q).0 <= 0.5);
    assert_eq!(ctx.attached(Pt::new(100.0, 30.0)), vec!["r"]);
    assert_eq!(ctx.attached(Pt::new(50.0, 30.0)), vec!["r"]);
    assert!(ctx.attached(Pt::new(150.0, 30.0)).is_empty());
}

#[test]
fn rounded_outline_stays_within_a_quarter_pixel_of_the_arc() {
    let s = scene("box q at (0,0) size (100,60) radius=20\n");
    let q = polygon(&s.shapes[0]);
    let (c, r) = (Pt::new(20.0, 20.0), 20.0);
    for w in q.windows(2).filter(|w| w[0].x < 20.0 && w[0].y < 20.0) {
        let mid = Pt::new((w[0].x + w[1].x) / 2.0, (w[0].y + w[1].y) / 2.0);
        assert!(r - mid.dist(c) <= 0.25, "{mid:?}");
    }
}

#[test]
fn rect_and_anchor_helpers() {
    let s = scene("box a at (100,100) size (100,60)\n");
    let a = &s.shapes[0];
    assert_eq!(anchor_text(a, Pt::new(100.0, 115.0)), "a.left@0.25");
    assert_eq!(anchor_text(a, Pt::new(150.0, 160.0)), "a.bottom");
    assert_eq!(
        overflow(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Rect::new(-5.0, 10.0, 120.0, 10.0)
        ),
        vec![("left", 5.0), ("right", 15.0)]
    );
}

#[test]
fn text_boxes_are_the_measured_rects() {
    let s = scene("text \"abc\" at (100,50) size=10\nline l (0,300) -> h 200 label@0.25=\"hi\"\n");
    let t = &s.texts[0];
    assert_eq!(text_box(t), t.rect);
    assert!(t.rect.w > 0.0 && t.rect.h > 0.0);
    assert!((t.rect.center().x - 100.0).abs() < 1e-9);
    assert!((t.rect.center().y - 50.0).abs() < 1e-9);

    let l = &s.lines[0];
    let lab = &l.labels[0];
    assert_eq!(label_box(lab), lab.rect);
    assert!((lab.rect.center().x - 50.0).abs() < 1e-9);
    let moved = label_box_at(lab, Pt::new(150.0, 300.0));
    assert_eq!((moved.w, moved.h), (lab.rect.w, lab.rect.h));
    assert!((moved.center().x - 150.0).abs() < 1e-9);
    assert!((moved.center().y - 300.0).abs() < 1e-9);
}

#[test]
fn group_title_band_and_box() {
    let s = scene(
        "group g \"VPC\" at (0,100) size (200,100) {\n}\ngroup u at (300,100) size (200,100) {\n}\nbox b \"x\" at (0,300) size (100,60)\n",
    );
    let g = &s.shapes[0];
    let band = title_band(g).unwrap();
    assert_eq!(band, Rect::new(0.0, 100.0, 200.0, g.title_band));
    assert!(g.title_band > g.style.padding.y);
    let title = title_box(g).unwrap();
    assert!(title.x >= band.x && title.bottom() <= band.bottom() + 1e-9);
    assert_eq!(title.y, 100.0 + g.style.padding.y);
    assert_eq!(title_band(&s.shapes[1]), None);
    assert_eq!(title_box(&s.shapes[1]), None);
    assert_eq!(title_band(&s.shapes[2]), None);
    assert_eq!(title_box(&s.shapes[2]), None);
    let s = scene("group g \"VPC\" at (0,100) size (200,100) title=bottom {\n}\n");
    let g = &s.shapes[0];
    let band = title_band(g).unwrap();
    assert_eq!(
        band,
        Rect::new(0.0, 200.0 - g.title_band, 200.0, g.title_band)
    );
    assert!((title_box(g).unwrap().bottom() - (200.0 - g.style.padding.y)).abs() < 1e-9);
}

#[test]
fn ports_list_endpoints_on_outlines() {
    let s = scene(
        "box a at (100,100) size (100,60)\njunction j at (0,0)\nline l a.top -> v -50 end=arrow\nline m j -> a.left@0.25\n",
    );
    let p = ports(&Ctx::new(&s));
    let got: Vec<(&str, &str, f64, &str, &str, bool)> = p
        .iter()
        .map(|x| {
            (
                x.shape.as_str(),
                x.side,
                x.t,
                x.line.as_str(),
                x.end,
                x.marker,
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            ("a", "top", 0.5, "l", "start", false),
            ("a", "left", 0.25, "m", "end", false)
        ]
    );
}
