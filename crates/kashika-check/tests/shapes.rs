mod common;

use common::*;
use kashika_geom::{Pt, boundary_nearest, point_in_polygon};

const DIAMOND: &str = "box d at (100,100) size (100,100) shape=diamond\n";
const CYLINDER: &str = "box db at (100,100) size (100,100) shape=cylinder\n";

#[test]
fn lines_through_a_diamond_follow_its_outline() {
    let d = lint(
        "through",
        &format!("{DIAMOND}line l (0,150) -> (300,150)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l passes through box 'd'");
    assert_eq!(d[0].facts["entry"], serde_json::json!([100.0, 150.0]));
    assert!(
        lint(
            "through",
            &format!("{DIAMOND}line l (90,110) -> (110,90)\n")
        )
        .is_empty()
    );
}

#[test]
fn near_measures_to_the_diamond_edge() {
    let d = lint("near", &format!("{DIAMOND}line l (100,145) -> (145,100)\n"));
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].facts["distance"], serde_json::json!(3.54));
    assert!(lint("near", &format!("{DIAMOND}line l (90,140) -> (140,90)\n")).is_empty());
}

#[test]
fn diamonds_overlap_only_when_their_outlines_do() {
    let a = "box a at (0,0) size (100,100) shape=diamond\n";
    assert!(
        lint(
            "overlap",
            &format!("{a}box b at (80,80) size (100,100) shape=diamond\n")
        )
        .is_empty()
    );
    let d = lint(
        "overlap",
        &format!("{a}box b at (40,40) size (100,100) shape=diamond\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
}

#[test]
fn endpoints_snap_to_the_cylinder_cap() {
    let d = lint(
        "endpoint-off-edge",
        &format!("{CYLINDER}line l (150,40) -> (150,97)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].facts["nearest"], serde_json::json!([150.0, 100.0]));
    assert_eq!(
        d[0].help,
        vec!["snap it to the outline with an anchor: `db.top`"]
    );
    assert!(
        lint(
            "endpoint-off-edge",
            &format!("{CYLINDER}line l (110,40) -> (110,108)\n")
        )
        .is_empty()
    );
}

#[test]
fn anchored_lines_on_built_in_shapes_check_clean() {
    let s = scene(&format!(
        "{CYLINDER}box d at (300,100) size (100,100) shape=diamond\n\
         line a d.left -> db.right end=arrow\n\
         line b db.top@0.2 -> v -60 end=arrow\n\
         line c d.top@0.3 -> v -60 end=arrow\n"
    ));
    assert!(kashika_check::check(&s).is_empty());
    let ctx = kashika_check::ctx::Ctx::new(&s);
    let ports: Vec<_> = kashika_check::ports::ports(&ctx)
        .into_iter()
        .map(|p| format!("{}.{}@{}", p.shape, p.side, (p.t * 100.0).round() / 100.0))
        .collect();
    assert_eq!(
        ports,
        ["db.right@0.5", "db.top@0.2", "d.left@0.5", "d.top@0.3"]
    );
}

const LOGO: &str = "image logo \"logo.png\" at (100,100) size (100,50)\n";

#[test]
fn images_are_opaque_obstacles() {
    let dir = tempfile::tempdir().unwrap();
    let d = lint_in(
        dir.path(),
        "through",
        &format!("{LOGO}line l (0,125) -> (300,125)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l passes through image 'logo'");
    assert_eq!(d[0].elements[1].kind, "image");
    assert!(d[0].help[0].contains("0..94, 156..600"), "{:?}", d[0].help);
    let d = lint_in(
        dir.path(),
        "near",
        &format!("{LOGO}line l (0,97) -> (300,97)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l runs 3px from image 'logo' (clearance 6)");
    let d = lint_in(
        dir.path(),
        "overlap",
        &format!("{LOGO}box b at (180,120) size (60,60)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "box 'b' overlaps image 'logo'");
    let d = lint_in(
        dir.path(),
        "label-collision",
        &format!("{LOGO}line l (0,125) -> (300,125) label=\"to logo\"\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(
        d[0].message.ends_with("collides with image 'logo'"),
        "{}",
        d[0].message
    );
    let d = lint_in(
        dir.path(),
        "out-of-canvas",
        "image logo \"logo.png\" at (760,0) size (80,40)\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(d[0].message.starts_with("image 'logo' extends beyond"));
}

#[test]
fn lines_may_attach_to_images() {
    let dir = tempfile::tempdir().unwrap();
    for code in ["through", "near", "endpoint-off-edge"] {
        assert!(
            lint_in(
                dir.path(),
                code,
                &format!("{LOGO}box b at (400,100) size (100,50)\nline l logo.right -> b.left\n"),
            )
            .is_empty()
        );
    }
}

const BELOW: &str = "shape srv from \"server\" text-area=below\n";

#[test]
fn labels_below_a_shape_are_checked_where_they_sit() {
    let d = lint(
        "out-of-canvas",
        &format!("{BELOW}box s \"Uploads\" at (100,530) size (60,60) shape=srv\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(
        d[0].message
            .starts_with("the label under box 's' extends beyond the 800x600 canvas"),
        "{}",
        d[0].message
    );
    let body = format!("{BELOW}box s \"Uploads\" at (100,100) size (60,60) shape=srv\n");
    let d = lint(
        "label-collision",
        &format!("{body}line l (0,177) -> (300,177)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "the label under box 's' collides with line l");
    assert!(d[0].help.is_empty(), "{:#?}", d[0].help);
    assert!(lint("label-collision", &body).is_empty());
    let d = lint(
        "spill",
        &format!(
            "{BELOW}group g at (80,80) size (120,100) {{\n  box s \"Uploads\" at (20,20) size (60,60) shape=srv\n}}\n"
        ),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(
        d[0].message
            .starts_with("the label under box 's' extends outside group 'g'")
    );
}

#[test]
fn a_line_leaving_through_the_label_below_suggests_another_side() {
    let body = format!("{BELOW}box s \"Uploads\" at (100,100) size (60,60) shape=srv\n");
    let d = lint(
        "label-collision",
        &format!("{body}line l s.bottom -> v 100\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].help,
        vec![
            "move the line to another side of 's', or give 's' an `icon=` with the text inside the box instead of below"
                .to_string()
        ]
    );
}

#[test]
fn labels_below_a_shape_sit_on_the_canvas_not_the_fill() {
    let d = lint(
        "low-contrast",
        &format!(
            "{BELOW}box s \"Uploads\" at (100,100) size (60,60) shape=srv fill=#000000 color=#ffffff\n"
        ),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].facts["background"], serde_json::json!("#ffffff"));
    assert!(
        lint(
            "low-contrast",
            "box s \"Uploads\" at (100,100) size (120,60) fill=#000000 color=#ffffff\n"
        )
        .is_empty()
    );
}

#[test]
fn icons_shrink_the_text_area_without_warnings() {
    let s = scene(
        "box q \"Jobs\" at (100,100) size (160,60) icon=\"queue\" style=queue\n\
         box w \"Web\" at (100,200) size (160,80) icon=\"browser\" icon-pos=top\n\
         line l q.bottom -> w.top end=arrow\n",
    );
    assert!(kashika_check::check(&s).is_empty());
    assert!(s.shapes[0].text_area.x >= 100.0 + 24.0);
}

#[test]
fn ports_name_the_side_the_line_leaves_from() {
    let scene = scene_src(
        "diagram \"p\" size (400,300)\nbox s at (100,100) size (120,80) shape=diamond\nline l s.left@0.3 -> (0,124)\nline m s.top@0.2 -> (124,0)\n",
    );
    let ctx = kashika_check::ctx::Ctx::new(&scene);
    let ports: Vec<(String, &str, f64)> = kashika_check::ports::ports(&ctx)
        .into_iter()
        .map(|p| (p.line, p.side, (p.t * 100.0).round() / 100.0))
        .collect();
    assert_eq!(
        ports,
        vec![("l".into(), "left", 0.3), ("m".into(), "top", 0.2)]
    );
}

const SIDES: [(&str, &str); 4] = [
    ("top", "v -100"),
    ("bottom", "v 100"),
    ("left", "h -100"),
    ("right", "h 100"),
];

#[test]
fn every_shape_side_and_t_is_a_clean_port() {
    let mut table = String::new();
    for shape in kashika_model::SHAPES {
        for (side, step) in SIDES {
            for t in [0.2, 0.5, 0.8] {
                let at = format!("{} {side}@{t}", shape.name);
                let s = scene(&format!(
                    "box s at (200,200) size (120,80) shape={}\nline l s.{side}@{t} -> {step}\n",
                    shape.name
                ));
                let poly = kashika_check::outline::polygon(&s.shapes[0]);
                let start = s.lines[0].path[0].from();
                let off = kashika_geom::boundary_nearest(start, &poly).0;
                assert!(off <= 0.5, "{at}: {start:?} is {off} from the outline");
                let found: Vec<_> = kashika_check::check(&s)
                    .into_iter()
                    .filter(|d| ["through", "near", "endpoint-off-edge"].contains(&d.code))
                    .collect();
                assert!(found.is_empty(), "{at}: {found:#?}");
                let ctx = kashika_check::ctx::Ctx::new(&s);
                let ports = kashika_check::ports::ports(&ctx);
                assert_eq!(ports.len(), 1, "{at}: {ports:#?}");
                let reported = (ports[0].side, (ports[0].t * 100.0).round() / 100.0);
                assert_eq!(reported, (side, t), "{at}");
                table.push_str(&format!("{at} -> {}@{}\n", reported.0, reported.1));
            }
        }
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/ports.txt");
    if std::env::var_os("KASHIKA_BLESS").is_some_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &table).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        table == expected,
        "ports table differs from {}; if intended, run: KASHIKA_BLESS=1 cargo test -p kashika-check --test shapes\n{}",
        path.display(),
        diff_lines(&expected, &table)
    );
}

fn diff_lines(expected: &str, actual: &str) -> String {
    let (e, a): (Vec<_>, Vec<_>) = (expected.lines().collect(), actual.lines().collect());
    let mut out: String = e
        .iter()
        .zip(&a)
        .filter(|(e, a)| e != a)
        .map(|(e, a)| format!("-{e}\n+{a}\n"))
        .collect();
    if e.len() != a.len() {
        out.push_str(&format!("{} lines expected, {} actual\n", e.len(), a.len()));
    }
    out
}

/// A line that cuts the box corner `c` (`d` points inward), entering and leaving the bounding
/// box `m` px from the corner along each edge.
fn corner_chord(c: Pt, d: Pt, m: f64) -> (Pt, Pt) {
    (
        Pt::new(c.x + (m + 10.0) * d.x, c.y - 10.0 * d.y),
        Pt::new(c.x - 10.0 * d.x, c.y + (m + 10.0) * d.y),
    )
}

fn clear_of(a: Pt, b: Pt, poly: &[Pt]) -> bool {
    (0..=200).all(|i| {
        let s = i as f64 / 200.0;
        let p = Pt::new(a.x + (b.x - a.x) * s, a.y + (b.y - a.y) * s);
        !point_in_polygon(p, poly) && boundary_nearest(p, poly).0 > 1.0
    })
}

#[test]
fn lines_across_a_bounding_box_corner_outside_the_outline_are_not_through() {
    // Skipped: these outlines fill their bounding-box corners (`rounded` to within a few
    // pixels), so no line cuts a corner without crossing them.
    const FILLED: &[&str] = &["rect", "rounded", "predefined-process"];
    let corners = [
        (Pt::new(200.0, 200.0), Pt::new(1.0, 1.0)),
        (Pt::new(320.0, 200.0), Pt::new(-1.0, 1.0)),
        (Pt::new(320.0, 280.0), Pt::new(-1.0, -1.0)),
        (Pt::new(200.0, 280.0), Pt::new(1.0, -1.0)),
    ];
    let line = |box_src: &str, (a, b): (Pt, Pt)| {
        format!("{box_src}line l ({},{}) -> ({},{})\n", a.x, a.y, b.x, b.y)
    };
    let rect = "box s at (200,200) size (120,80)\n";
    let d = lint(
        "through",
        &line(rect, corner_chord(corners[0].0, corners[0].1, 6.0)),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    let mut filled = Vec::new();
    for shape in kashika_model::SHAPES {
        let box_src = format!("box s at (200,200) size (120,80) shape={}\n", shape.name);
        let poly = kashika_check::outline::polygon(&scene(&box_src).shapes[0]);
        let chord = corners.iter().find_map(|&(c, d)| {
            (4..=20)
                .rev()
                .map(|m| corner_chord(c, d, m as f64))
                .find(|&(a, b)| clear_of(a, b, &poly))
        });
        let Some(chord) = chord else {
            filled.push(shape.name);
            continue;
        };
        let src = line(&box_src, chord);
        let found = lint("through", &src);
        assert!(found.is_empty(), "{}: {src}{found:#?}", shape.name);
    }
    assert_eq!(filled, FILLED);
}
