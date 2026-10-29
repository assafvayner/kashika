use kashika_diag::Diagnostic;
use kashika_geom::{PathSeg, Pt, Rect};
use kashika_model::{Outline, Scene, Shape, compile};

const HEAD: &str = "diagram \"t\" size (800,600)\n";

fn ok(body: &str) -> Scene {
    let (scene, diags) = compile(&format!("{HEAD}{body}"));
    assert!(diags.is_empty(), "{diags:#?}");
    scene
}

#[allow(dead_code)]
fn errors(body: &str) -> Vec<Diagnostic> {
    compile(&format!("{HEAD}{body}")).1
}

fn shape<'a>(s: &'a Scene, id: &str) -> &'a Shape {
    s.shapes.iter().find(|x| x.id == id).unwrap()
}

#[allow(dead_code)]
fn start(s: &Scene, id: &str) -> Pt {
    s.lines.iter().find(|l| l.id == id).unwrap().path[0].from()
}

#[allow(dead_code)]
fn close(a: Pt, b: Pt) -> bool {
    a.dist(b) <= 0.3
}

#[allow(dead_code)]
fn first_seg(s: &Scene, id: &str) -> PathSeg {
    s.lines.iter().find(|l| l.id == id).unwrap().path[0]
}

#[test]
fn rect_outline_polygon_and_text_area() {
    let s = ok("box a at (10,20) size (100,60)\n");
    let a = shape(&s, "a");
    assert_eq!(a.shape, "rect");
    assert!(a.form.is_none());
    assert!(a.image.is_none() && a.icon.is_none());
    assert_eq!(
        a.outline_polygon(),
        vec![
            Pt::new(10.0, 20.0),
            Pt::new(110.0, 20.0),
            Pt::new(110.0, 80.0),
            Pt::new(10.0, 80.0)
        ]
    );
    assert_eq!(a.text_area, Rect::new(18.0, 28.0, 84.0, 44.0));
    assert!(s.assets.is_empty());
}

#[test]
fn rounded_outline_polygon_follows_the_corners() {
    let s = ok("box r at (0,0) size (100,60) shape=rounded\n");
    let r = shape(&s, "r");
    assert_eq!(r.outline, Outline::Rounded);
    assert_eq!(r.shape, "rounded");
    let poly = r.outline_polygon();
    assert!(poly.len() > 8);
    assert!(!poly.contains(&Pt::new(0.0, 0.0)), "corner is cut");
    for p in &poly {
        assert!(p.x >= -1e-9 && p.x <= 100.0 + 1e-9 && p.y >= -1e-9 && p.y <= 60.0 + 1e-9);
    }
    let corner = Pt::new(8.0, 8.0);
    let nearest = poly
        .iter()
        .map(|p| p.dist(corner))
        .fold(f64::INFINITY, f64::min);
    assert!((nearest - 8.0).abs() < 0.3, "{nearest}");
}

#[test]
fn radius_alone_makes_a_rounded_rect() {
    let s = ok("box r at (0,0) size (100,60) radius=4\n");
    let r = shape(&s, "r");
    assert_eq!(r.outline, Outline::Rounded);
    assert_eq!(r.shape, "rounded");
    assert!(r.outline_polygon().len() > 8);
}

#[test]
fn group_outline_polygon_is_its_rect() {
    let s = ok("group g \"G\" at (0,0) size (200,100) {\n}\n");
    let g = shape(&s, "g");
    assert_eq!(g.outline_polygon().len(), 4);
}

/// The built-in shapes drawn from unit paths: all but `rect` and `rounded`.
fn path_shapes() -> impl Iterator<Item = &'static str> {
    kashika_model::SHAPES
        .iter()
        .filter(|s| s.source.is_some())
        .map(|s| s.name)
}

#[test]
fn every_builtin_shape_resolves_inside_its_box() {
    for name in path_shapes() {
        let s = ok(&format!(
            "box a \"A\" at (0,0) size (160,100) shape={name}\n"
        ));
        let a = shape(&s, "a");
        assert_eq!(a.outline, Outline::Path, "{name}");
        assert_eq!(a.shape, name);
        let f = a.form.as_ref().unwrap();
        assert!(f.outline.len() >= 3, "{name}");
        assert!(
            f.outline
                .iter()
                .all(|p| p.x >= -0.01 && p.x <= 160.01 && p.y >= -0.01 && p.y <= 100.01),
            "{name} outline leaves its box"
        );
        let ta = a.text_area;
        assert!(
            ta.w > 0.0 && ta.h > 0.0 && ta.x >= 0.0 && ta.right() <= 160.0 && ta.bottom() <= 100.0,
            "{name}: {ta:?}"
        );
        assert_eq!(a.outline_polygon(), f.outline);
        let label = a.text[0].rect;
        assert!(
            (label.center().x - ta.center().x).abs() < 1e-9
                && (label.center().y - ta.center().y).abs() < 1e-9,
            "{name}: label is centered in the shape's text area"
        );
    }
}

#[test]
fn diamond_anchors_are_axis_projected() {
    let s = ok("box d at (0,0) size (120,80) shape=diamond\n\
                line l d.right@0.25 -> h 40\nline m d.top@0.25 -> v -40\n\
                line n d.tl -> v -10\nline c d.center -> h 1\n");
    assert_eq!(start(&s, "l"), Pt::new(90.0, 20.0));
    assert_eq!(start(&s, "m"), Pt::new(30.0, 20.0));
    assert_eq!(
        start(&s, "n"),
        Pt::new(0.0, 0.0),
        "corners stay on the bbox"
    );
    assert_eq!(start(&s, "c"), Pt::new(60.0, 40.0));
}

#[test]
fn cylinder_top_follows_the_cap() {
    let s = ok("box c at (0,0) size (120,80) shape=cylinder\n\
                line a c.top -> v -20\nline b c.top@0.25 -> v -20\nline e c.bottom -> v 20\n");
    assert!(close(start(&s, "a"), Pt::new(60.0, 0.0)));
    assert!(close(
        start(&s, "b"),
        Pt::new(30.0, 20.0 - 20.0 * 0.75f64.sqrt())
    ));
    assert!(close(start(&s, "e"), Pt::new(60.0, 80.0)));
    assert_eq!(
        start(&s, "b").x,
        30.0,
        "a vertical line into top stays vertical"
    );
}

#[test]
fn builtin_text_areas() {
    let s = ok(
        "box d at (0,0) size (120,80) shape=diamond\nbox c at (200,0) size (120,80) shape=cylinder\n",
    );
    let d = shape(&s, "d").text_area;
    assert!(
        close(Pt::new(d.x, d.y), Pt::new(34.4, 30.4))
            && close(Pt::new(d.w, d.h), Pt::new(51.2, 19.2)),
        "{d:?}"
    );
    assert_eq!(
        shape(&s, "c").text_area,
        Rect::new(208.0, 48.0, 104.0, 24.0)
    );
}

#[test]
fn builtin_text_areas_stay_inside_the_outline() {
    use kashika_geom::{boundary_nearest, point_in_polygon};
    for name in path_shapes() {
        for (w, h) in [
            (120, 80),
            (60, 160),
            (240, 80),
            (200, 200),
            (160, 50),
            (40, 40),
            (600, 90),
        ] {
            let s = ok(&format!(
                "box a at (0,0) size ({w},{h}) shape={name} padding=0\n"
            ));
            let a = shape(&s, "a");
            let poly = &a.form.as_ref().unwrap().outline;
            let t = a.text_area;
            // A cylinder's text area runs down to the bottom of the box, past its bottom cap's corners.
            let bottom = if name == "cylinder" { t.y } else { t.bottom() };
            for i in 0..=20 {
                let f = f64::from(i) / 20.0;
                for p in [
                    Pt::new(t.x + f * t.w, t.y),
                    Pt::new(t.x + f * t.w, bottom),
                    Pt::new(t.x, t.y + f * (bottom - t.y)),
                    Pt::new(t.right(), t.y + f * (bottom - t.y)),
                ] {
                    assert!(
                        point_in_polygon(p, poly) || boundary_nearest(p, poly).0 < 0.5,
                        "{name} {w}x{h}: {p:?} of {t:?} is outside the outline"
                    );
                }
            }
        }
    }
}

#[test]
fn tall_hexagons_keep_half_their_width_for_text() {
    let s = ok("box hx at (0,0) size (60,160) shape=hexagon padding=0\n");
    assert_eq!(shape(&s, "hx").text_area, Rect::new(15.0, 0.0, 30.0, 160.0));
}

#[test]
fn pixel_features_do_not_stretch() {
    let s = ok("box d at (0,0) size (100,200) shape=doc padding=0\n\
                box p at (200,0) size (300,300) shape=person padding=0\n\
                box c at (0,300) size (400,200) shape=cloud padding=0\n");
    assert!(
        (shape(&s, "d").text_area.h - 182.0).abs() < 1e-9,
        "the wave is 12px deep"
    );
    let p = &shape(&s, "p").form.as_ref().unwrap().outline;
    let across = kashika_geom::horizontal_hits(p, 18.0);
    let (l, r) = across
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, r), x| (l.min(*x), r.max(*x)));
    assert!(
        (r - l - 36.0).abs() < 0.5,
        "the head is a circle of radius 18: {across:?}"
    );
    assert!(
        (shape(&s, "c").text_area.y - 324.0).abs() < 1e-9,
        "bumps are 24px high"
    );
}

const BIG: &str = "diagram \"t\" size (4000,4000)\n";

#[test]
fn fitted_sizes_hold_their_text_and_are_minimal() {
    for name in kashika_model::shape_names() {
        for size in [9, 12, 14, 20, 28] {
            for label in ["Q", "Payments API", "Two\\nlines here"] {
                let (s, d) = compile(&format!(
                    "{BIG}box a \"{label}\" at (0,0) shape={name} font-size={size}\n"
                ));
                assert!(d.is_empty(), "{name} {size} {label}: {d:#?}");
                let a = shape(&s, "a");
                let need = a.text[0].rect;
                let (w, h) = (a.rect.w, a.rect.h);
                let area = |w: f64, h: f64| {
                    let (s, _) = compile(&format!(
                        "{BIG}box a \"{label}\" at (0,0) size ({w},{h}) shape={name} font-size={size}\n"
                    ));
                    shape(&s, "a").text_area
                };
                let at = area(w, h);
                assert!(
                    at.w + 1e-9 >= need.w && at.h + 1e-9 >= need.h,
                    "{name} {size} {label}: {at:?} at ({w},{h}) does not hold {need:?}"
                );
                assert!(
                    area(w - 1.0, h).w < need.w && area(w, h - 1.0).h < need.h,
                    "{name} {size} {label}: ({w},{h}) is not minimal"
                );
            }
        }
    }
}

#[test]
fn anchors_on_placed_shapes_use_the_outline() {
    let s = ok("box d at (0,0) size (120,80) shape=diamond\n\
                box e at d.right size (40,40) pin=left\n");
    assert_eq!(shape(&s, "e").rect, Rect::new(120.0, 20.0, 40.0, 40.0));
}

#[test]
fn unknown_shape_suggests_a_name() {
    let d = errors("box a at (0,0) size (10,10) shape=cylindr\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "unknown-shape");
    assert_eq!(d[0].help, vec!["did you mean `cylinder`?".to_string()]);
}

#[test]
fn label_overflow_suggests_a_size_that_fits_the_shape() {
    for name in path_shapes() {
        let d = errors(&format!(
            "box a \"A rather long label\" at (0,0) size (60,30) shape={name}\n"
        ));
        assert_eq!(d.len(), 1, "{name}: {d:#?}");
        assert_eq!(d[0].code, "label-overflow", "{name}");
        let help = &d[0].help[0];
        let size = help
            .split("size (")
            .nth(1)
            .and_then(|s| s.split(')').next())
            .unwrap_or_else(|| panic!("{name}: {help}"));
        let fixed = errors(&format!(
            "box a \"A rather long label\" at (0,0) size ({size}) shape={name}\n"
        ));
        assert!(fixed.is_empty(), "{name} at ({size}): {fixed:#?}");
    }
}

#[test]
fn builtin_source_is_exposed_for_ref() {
    let (d, outline, ta) = kashika_model::builtin_source("diamond", 120.0, 80.0).unwrap();
    assert_eq!(d, "M 0.5 0 L 1 0.5 L 0.5 1 L 0 0.5 Z");
    assert_eq!(outline, 1);
    assert_eq!(ta, Rect::new(0.22, 0.28, 0.56, 0.44));
    assert!(kashika_model::builtin_source("rect", 1.0, 1.0).is_none());
}

#[test]
fn custom_shapes_stretch_to_the_box() {
    let s = ok(
        "shape tri {\n  path \"M 0.5 0 L 1 1 L 0 1 Z  M 0.2 0.8 L 0.8 0.8\"\n  text-area (0.25,0.5,0.5,0.5)\n}\n\
                box t at (100,100) size (200,100) shape=tri\nline l t.left -> h -20\n",
    );
    let t = shape(&s, "t");
    assert_eq!(t.shape, "tri");
    assert_eq!(t.outline, Outline::Path);
    assert_eq!(t.form.as_ref().unwrap().draw.len(), 2);
    assert_eq!(start(&s, "l"), Pt::new(150.0, 150.0));
    assert_eq!(t.text_area, Rect::new(158.0, 158.0, 84.0, 34.0));
}

#[test]
fn outline_picks_the_subpath() {
    let s = ok(
        "shape two {\n  path \"M 0 0 L 1 0 L 1 0.2 L 0 0.2 Z M 0 0.5 L 1 0.5 L 1 1 L 0 1 Z\"\n  outline 2\n}\n\
                box b at (0,0) size (100,100) shape=two\nline l b.top -> v -10\n",
    );
    assert_eq!(start(&s, "l"), Pt::new(50.0, 50.0));
}

#[test]
fn custom_shape_without_text_area_uses_the_whole_box() {
    let s = ok(
        "shape tri {\n  path \"M 0.5 0 L 1 1 L 0 1 Z\"\n}\nbox t at (0,0) size (100,50) shape=tri\n",
    );
    assert_eq!(shape(&s, "t").text_area, Rect::new(8.0, 8.0, 84.0, 34.0));
}

#[test]
fn custom_shape_label_overflow_uses_its_text_area() {
    let def = "shape tri {\n  path \"M 0.5 0 L 1 1 L 0 1 Z\"\n  text-area (0.25,0.5,0.5,0.5)\n}\n";
    let d = errors(&format!(
        "{def}box a \"A rather long label\" at (0,0) size (60,30) shape=tri\n"
    ));
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "label-overflow");
    let help = &d[0].help[0];
    let size = help
        .split("size (")
        .nth(1)
        .and_then(|s| s.split(')').next())
        .unwrap_or_else(|| panic!("{help}"));
    let fixed = errors(&format!(
        "{def}box a \"A rather long label\" at (0,0) size ({size}) shape=tri\n"
    ));
    assert!(fixed.is_empty(), "at ({size}): {fixed:#?}");
}

#[test]
fn custom_shape_errors() {
    for (body, code, line) in [
        ("shape s {\n  path \"M 0 0 L 2 0\"\n}\n", "parse", 3),
        ("shape s {\n  path \"m 0 0 l 1 0\"\n}\n", "parse", 3),
        (
            "shape s {\n  path \"M 0 0 L 1 0\"\n  outline 2\n}\n",
            "parse",
            4,
        ),
        (
            "shape s {\n  path \"M 0 0 L 1 0\"\n  text-area (0.5,0.5,0.6,0.1)\n}\n",
            "parse",
            4,
        ),
        (
            "shape cloud {\n  path \"M 0 0 L 1 0\"\n}\n",
            "duplicate-style",
            2,
        ),
        (
            "shape s {\n  path \"M 0 0 L 1 0\"\n}\nshape s {\n  path \"M 0 0 L 1 1\"\n}\n",
            "duplicate-style",
            5,
        ),
        (
            "group g at (0,0) size (10,10) {\n  shape s {\n    path \"M 0 0 L 1 0\"\n  }\n}\n",
            "parse",
            3,
        ),
    ] {
        let d = errors(body);
        assert_eq!(
            d.iter().map(|x| (x.code, x.span.line)).collect::<Vec<_>>(),
            vec![(code, line)],
            "{body}: {d:#?}"
        );
    }
}

#[test]
fn unknown_custom_shape_suggests_a_definition() {
    let d = errors(
        "shape tri {\n  path \"M 0 0 L 1 0 L 0 1 Z\"\n}\nbox b at (0,0) size (10,10) shape=tr\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "unknown-shape");
    assert_eq!(d[0].help, vec!["did you mean `tri`?".to_string()]);
}
