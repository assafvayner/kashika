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

fn near(a: Pt, b: Pt) -> bool {
    a.dist(b) <= 0.01
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
    // Each 90° corner is one cubic flattened in 4 steps: 4 × (line end + 4 arc points), less
    // the repeated start.
    assert_eq!(poly.len(), 20);
    let k = 4.0 / 3.0 * (std::f64::consts::PI / 8.0).tan() * 8.0;
    // Bernstein weights at t = 1/4 of the first corner cubic, (92,0) (92+k,0) (100,8-k) (100,8).
    let (a, b, c) = (0.421875, 0.140625, 0.015625);
    let third = Pt::new(
        a * 92.0 + a * (92.0 + k) + b * 100.0 + c * 100.0,
        b * (8.0 - k) + c * 8.0,
    );
    assert_eq!(poly[0], Pt::new(8.0, 0.0));
    assert_eq!(poly[1], Pt::new(92.0, 0.0));
    assert!(poly[2].dist(third) < 1e-9, "{:?} vs {third:?}", poly[2]);
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
    assert!(near(start(&s, "a"), Pt::new(60.0, 0.0)));
    let b = start(&s, "b");
    assert!(
        near(b, Pt::new(30.0, 20.0 - 20.0 * 0.75f64.sqrt())),
        "{b:?}"
    );
    assert!(near(start(&s, "e"), Pt::new(60.0, 80.0)));
    assert_eq!(
        start(&s, "b").x,
        30.0,
        "a vertical line into top stays vertical"
    );
}

#[test]
fn person_anchors_reach_the_head_and_body() {
    let s = ok("box p at (0,0) size (120,120) shape=person\n\
                line a p.top -> v -20\nline b p.left@0.9 -> h -20\n\
                line c p.right@0.9 -> h 20\nline e p.bottom -> v 20\n");
    assert!(
        near(start(&s, "a"), Pt::new(60.0, 0.0)),
        "{:?}",
        start(&s, "a")
    );
    assert!(near(start(&s, "b"), Pt::new(0.0, 108.0)));
    assert!(near(start(&s, "c"), Pt::new(120.0, 108.0)));
    assert!(near(start(&s, "e"), Pt::new(60.0, 120.0)));
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
                box q at (500,300) size (300,100) shape=person padding=0\n\
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
    // The flattened outline is inscribed in the circle: each chord cuts about 0.015px inside.
    assert!(
        r - l <= 36.0 + 1e-9 && r - l > 35.95,
        "the head is a circle of radius 18: {across:?}"
    );
    let head = |id: &str| {
        let f = shape(&s, id).form.as_ref().unwrap();
        assert_eq!(f.draw.len(), 2, "the outline subpath is not drawn");
        let pts = kashika_geom::flatten(&f.draw[0]);
        let (x0, x1, y0, y1) = pts.iter().fold(
            (f64::MAX, f64::MIN, f64::MAX, f64::MIN),
            |(a, b, c, d), p| (a.min(p.x), b.max(p.x), c.min(p.y), d.max(p.y)),
        );
        (x1 - x0, y1 - y0, y0)
    };
    for (id, top) in [("p", 0.0), ("q", 300.0)] {
        let (w, h, y) = head(id);
        assert!(
            (w - 36.0).abs() < 0.01 && (h - 36.0).abs() < 0.01 && (y - top).abs() < 1e-9,
            "{id}: the head stays a circle of radius 18 at the top: {w}x{h} at y={y}"
        );
    }
    assert!(
        (shape(&s, "c").text_area.y - 324.0).abs() < 1e-9,
        "bumps are 24px high"
    );
}

#[test]
fn flowchart_and_uml_features_are_capped_in_pixels() {
    for (name, want) in [
        ("parallelogram", Rect::new(24.0, 0.0, 352.0, 300.0)),
        ("predefined-process", Rect::new(12.0, 0.0, 376.0, 300.0)),
        ("octagon", Rect::new(12.0, 12.0, 376.0, 276.0)),
        ("note", Rect::new(0.0, 8.0, 392.0, 292.0)),
        ("package", Rect::new(0.0, 12.0, 400.0, 288.0)),
        ("queue", Rect::new(12.0, 0.0, 364.0, 300.0)),
        ("stadium", Rect::new(85.5, 15.0, 229.0, 270.0)),
    ] {
        let s = ok(&format!(
            "box a at (0,0) size (400,300) shape={name} padding=0\n"
        ));
        let t = shape(&s, "a").text_area;
        assert!(
            near(Pt::new(t.x, t.y), Pt::new(want.x, want.y))
                && near(Pt::new(t.w, t.h), Pt::new(want.w, want.h)),
            "{name}: {t:?}"
        );
    }
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
    assert_eq!(d[0].help[0], "did you mean `cylinder`?");
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
    assert_eq!(d[0].help[0], "did you mean `tri`?");
}

const SIDES: [&str; 4] = ["top", "bottom", "left", "right"];
const SIZES: [(f64, f64); 3] = [(120.0, 80.0), (40.0, 40.0), (60.0, 200.0)];

fn placed(name: &str, (w, h): (f64, f64)) -> Shape {
    let radius = if name == "rounded" { " radius=8" } else { "" };
    let s = ok(&format!(
        "box a at (0,0) size ({w},{h}) shape={name}{radius}\n"
    ));
    s.shapes.into_iter().next().unwrap()
}

#[test]
fn anchor_lies_on_the_outline() {
    use kashika_geom::{boundary_nearest, horizontal_hits, vertical_hits};
    for name in kashika_model::shape_names() {
        let tol = if name == "rounded" { 0.25 } else { 0.5 };
        for size in SIZES {
            let a = placed(name, size);
            let poly = a.outline_polygon();
            for side in SIDES {
                for i in 0..=10 {
                    let t = f64::from(i) / 10.0;
                    let p = a.anchor(side, Some(t)).unwrap();
                    let (x, y) = (size.0 * t, size.1 * t);
                    let misses = match side {
                        "top" | "bottom" => vertical_hits(&poly, x).is_empty(),
                        _ => horizontal_hits(&poly, y).is_empty(),
                    };
                    if misses && a.form.is_some() {
                        let bbox = match side {
                            "top" => Pt::new(x, 0.0),
                            "bottom" => Pt::new(x, size.1),
                            "left" => Pt::new(0.0, y),
                            _ => Pt::new(size.0, y),
                        };
                        assert_eq!(p, bbox, "{name} {size:?} {side}@{t}: a miss keeps the bbox");
                        continue;
                    }
                    let d = boundary_nearest(p, &poly).0;
                    assert!(
                        d < tol,
                        "{name} {size:?} {side}@{t}: {p:?} is {d} off the outline"
                    );
                }
            }
        }
    }
}

#[test]
fn text_area_is_monotonic() {
    for name in path_shapes() {
        let px = |w: f64, h: f64| {
            let u = kashika_model::builtin_source(name, w, h).unwrap().2;
            (u.w * w, u.h * h)
        };
        for fixed in [20.0, 40.0, 80.0, 200.0, 400.0] {
            let mut last = (f64::MIN, f64::MIN);
            for i in 0..=38 {
                let v = f64::from(20 + 10 * i);
                let (w, _) = px(v, fixed);
                let (_, h) = px(fixed, v);
                assert!(
                    w + 1e-9 >= last.0,
                    "{name}: width {w} at w={v} h={fixed} < {}",
                    last.0
                );
                assert!(
                    h + 1e-9 >= last.1,
                    "{name}: height {h} at h={v} w={fixed} < {}",
                    last.1
                );
                last = (w, h);
            }
        }
    }
}

#[test]
fn fitted_size_is_monotonic_in_label() {
    let labels = [
        "a",
        "ab",
        "abc",
        "abcd",
        "abcde",
        "abcdef",
        "abcdefg",
        "abcdefgh",
        "abcdefghi",
        "abcdefghij",
    ];
    for name in kashika_model::shape_names() {
        let body: String = labels
            .iter()
            .enumerate()
            .map(|(i, l)| format!("box b{i} \"{l}\" at ({},0) shape={name}\n", 300 * i))
            .collect();
        let (s, d) = compile(&format!("diagram \"t\" size (4000,400)\n{body}"));
        assert!(d.is_empty(), "{name}: {d:#?}");
        let sizes: Vec<(f64, f64)> = s.shapes.iter().map(|b| (b.rect.w, b.rect.h)).collect();
        for (i, p) in sizes.windows(2).enumerate() {
            assert!(
                p[1].0 >= p[0].0 && p[1].1 >= p[0].1,
                "{name}: {:?} for {:?} is smaller than {:?} for {:?}",
                p[1],
                labels[i + 1],
                p[0],
                labels[i]
            );
        }
    }
}

#[test]
fn pin_round_trips_on_every_shape() {
    let names = [
        "top",
        "bottom",
        "left",
        "right",
        "center",
        "tl",
        "tr",
        "bl",
        "br",
        "top-left",
        "top-right",
        "bottom-left",
        "bottom-right",
    ];
    let at = Pt::new(400.0, 300.0);
    for shape in kashika_model::shape_names() {
        for size in SIZES {
            let mut pins: Vec<(String, &str, Option<f64>)> =
                names.iter().map(|n| (n.to_string(), *n, None)).collect();
            for side in SIDES {
                for t in [0.1, 0.25, 0.9] {
                    pins.push((format!("{side}@{t}"), side, Some(t)));
                }
            }
            for (pin, name, t) in pins {
                let s = ok(&format!(
                    "box b at (400,300) size ({},{}) shape={shape} pin={pin}\n",
                    size.0, size.1
                ));
                let p = s.shapes[0].anchor(name, t).unwrap();
                assert!(
                    p.dist(at) <= 0.01,
                    "{shape} {size:?} pin={pin}: b.{pin} is {p:?}, not {at:?}"
                );
            }
        }
    }
}

#[test]
fn outlines_are_simple_closed_polygons() {
    use kashika_geom::seg_hit;
    for name in kashika_model::shape_names() {
        for size in SIZES {
            let poly = placed(name, size).outline_polygon();
            let n = poly.len();
            assert!(n >= 3, "{name} {size:?}: {n} vertices");
            assert!(
                poly[0].dist(poly[n - 1]) > 1e-9,
                "{name} {size:?}: the closing point is repeated"
            );
            let area: f64 = (0..n)
                .map(|i| {
                    let (a, b) = (poly[i], poly[(i + 1) % n]);
                    a.x * b.y - b.x * a.y
                })
                .sum::<f64>()
                / 2.0;
            assert!(area.abs() > 1.0, "{name} {size:?}: area {area}");
            for i in 0..n {
                for j in i + 2..n {
                    if i == 0 && j == n - 1 {
                        continue;
                    }
                    let hit = seg_hit(poly[i], poly[(i + 1) % n], poly[j], poly[(j + 1) % n], 0.0);
                    assert!(
                        hit.is_none(),
                        "{name} {size:?}: edges {i} and {j} meet at {hit:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn translating_a_group_translates_descendants() {
    let src = |x: f64, y: f64| {
        format!(
            "group outer \"Outer\" at ({x},{y}) size (460,380) {{\n\
             \x20 box a \"A\" at (20,40) size (80,40) shape=rounded\n\
             \x20 text t \"note\" at (40,340)\n\
             \x20 group inner \"Inner\" at (160,40) size (260,260) {{\n\
             \x20   box b \"B\" at (20,40) size (120,80) shape=diamond\n\
             \x20   box c \"C\" at b.bottom+(0,40) size (100,70) shape=cloud pin=top\n\
             \x20   junction j at (220,220)\n\
             \x20   line m b.right -> h 40 -> j label=\"m\"\n\
             \x20 }}\n\
             \x20 line l a.right -> b.left end=arrow label=\"l\"\n\
             \x20 line k a.bottom -> v 120 -> c.left\n\
             }}\n"
        )
    };
    let base = ok(&src(40.0, 30.0));
    let moved = ok(&src(77.0, 83.0));
    let d = Pt::new(37.0, 53.0);
    let same = |a: Pt, b: Pt, what: &str| {
        assert!(
            (b.x - a.x - d.x).abs() < 1e-9 && (b.y - a.y - d.y).abs() < 1e-9,
            "{what}: {a:?} -> {b:?}"
        );
    };
    let rect = |a: Rect, b: Rect, what: &str| {
        same(a.origin(), b.origin(), what);
        assert!(
            (a.w - b.w).abs() < 1e-9 && (a.h - b.h).abs() < 1e-9,
            "{what}: {a:?} -> {b:?}"
        );
    };
    assert_eq!(base.shapes.len(), 5);
    for (a, b) in base.shapes.iter().zip(&moved.shapes) {
        rect(a.rect, b.rect, &a.id);
        rect(a.text_area, b.text_area, &a.id);
        for (p, q) in a.outline_polygon().iter().zip(&b.outline_polygon()) {
            same(*p, *q, &a.id);
        }
        for (p, q) in a.text.iter().zip(&b.text) {
            rect(p.rect, q.rect, &a.id);
        }
    }
    assert_eq!(base.junctions.len(), 1);
    for (a, b) in base.junctions.iter().zip(&moved.junctions) {
        same(a.at, b.at, &a.id);
    }
    assert_eq!(base.texts.len(), 1);
    for (a, b) in base.texts.iter().zip(&moved.texts) {
        same(a.at, b.at, "text");
        rect(a.rect, b.rect, "text");
    }
    assert_eq!(base.lines.len(), 3);
    for (a, b) in base.lines.iter().zip(&moved.lines) {
        assert_eq!(a.path.len(), b.path.len(), "{}", a.id);
        for (p, q) in a.path.iter().zip(&b.path) {
            for i in 0..=4 {
                let t = f64::from(i) / 4.0;
                same(p.eval(t), q.eval(t), &a.id);
            }
        }
        for (p, q) in a.labels.iter().zip(&b.labels) {
            same(p.at, q.at, &a.id);
            rect(p.rect, q.rect, &a.id);
        }
    }
}
