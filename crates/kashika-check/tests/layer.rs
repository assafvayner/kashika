mod common;

use common::*;

fn codes(body: &str) -> Vec<String> {
    kashika_check::check(&scene(body))
        .into_iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

#[test]
fn layered_shapes_are_not_obstacles() {
    let shapes = |layer: &str| {
        format!(
            "box sq at (100,100) size (100,100) fill=#eeeeee {layer}\n\
             box piece \"K\" at (130,130) size (40,40)\n\
             line l (20,150) -> h 300 label=\"move\"\n\
             line m (20,97) -> h 300\n\
             line n (150,20) -> v 77\n\
             text \"note\" at (150,190)\n"
        )
    };
    let normal = codes(&shapes(""));
    for code in [
        "overlap",
        "through",
        "near",
        "label-collision",
        "endpoint-off-edge",
    ] {
        assert!(
            normal
                .iter()
                .any(|d| d.starts_with(code) && d.contains("'sq'")),
            "{code} missing: {normal:#?}"
        );
    }
    for layer in ["layer=under", "layer=over"] {
        let d = codes(&shapes(layer));
        assert!(
            d.iter().all(|d| !d.contains("'sq'") && !d.contains(" sq ")),
            "{layer}: {d:#?}"
        );
    }
}

#[test]
fn layered_shapes_keep_their_own_checks() {
    let d = codes("box sq at (780,20) size (100,100) layer=under\n");
    assert!(d.iter().any(|d| d.starts_with("out-of-canvas")), "{d:#?}");
    let d = codes(
        "group g \"G\" at (0,0) size (200,200) {\n  box sq at (150,150) size (100,100) layer=over\n}\n",
    );
    assert!(d.iter().any(|d| d.starts_with("spill")), "{d:#?}");
}

#[test]
fn contrast_follows_paint_order() {
    let body = |layer: &str| {
        format!(
            "box sq at (100,100) size (100,100) fill=#222222 {layer}\ntext \"dark\" at (150,150) color=#333333\n"
        )
    };
    assert_eq!(lint("low-contrast", &body("")).len(), 1);
    assert_eq!(lint("low-contrast", &body("layer=under")).len(), 1);
    assert!(lint("low-contrast", &body("layer=over")).is_empty());
    let d = lint(
        "low-contrast",
        "box ring \"R\" at (100,100) size (100,100) fill=#222222 color=#333333 layer=over\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
}

#[test]
fn an_under_layer_fill_inside_a_group_sits_above_the_group_fill() {
    let body = |inner: &str, outer: &str| {
        format!(
            "{outer}group g at (0,0) size (300,300) fill=#1e293b {{\n{inner}  text \"dark\" at (150,150) color=#333333\n}}\n"
        )
    };
    let light = "box sq at (100,100) size (100,100) fill=#fde68a layer=under\n";
    assert!(lint("low-contrast", &body(&format!("  {light}"), "")).is_empty());
    assert_eq!(lint("low-contrast", &body("", light)).len(), 1);
    assert_eq!(lint("low-contrast", &body("", "")).len(), 1);
}

#[test]
fn layer_values_and_where_it_applies() {
    let (_, d) = kashika_model::compile(&format!(
        "{HEAD}box a at (0,0) size (10,10) layer=top\ngroup g at (0,0) size (10,10) layer=under {{\n}}\n"
    ));
    let msgs: Vec<&str> = d.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        msgs,
        [
            "`layer` expects one of: under, over",
            "unknown attribute `layer` on group"
        ]
    );
}

#[test]
fn layered_images_still_report_distortion() {
    let dir = tempfile::tempdir().unwrap();
    let d = lint_in(
        dir.path(),
        "image-distort",
        "image logo \"logo.png\" at (20,20) size (100,100) fit=stretch layer=under\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
}
