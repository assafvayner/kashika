use kashika_geom::Pt;
use kashika_model::compile;

#[test]
fn shape_anchor_matches_the_resolver() {
    let (scene, diags) =
        compile("diagram \"a\" size (200,100)\nbox r at (10,10) size (100,60) shape=rounded\n");
    assert!(diags.is_empty(), "{diags:#?}");
    let r = &scene.shapes[0];
    assert_eq!(r.anchor("top"), Some(Pt::new(60.0, 10.0)));
    assert_eq!(r.anchor("left"), Some(Pt::new(10.0, 40.0)));
    assert_eq!(r.anchor("tl"), Some(Pt::new(10.0, 10.0)));
    assert_eq!(r.anchor("bottom-right"), Some(Pt::new(110.0, 70.0)));
    assert_eq!(r.anchor("nope"), None);
}
