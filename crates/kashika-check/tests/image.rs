mod common;

use common::*;

#[test]
fn image_distort_flags_stretch_beyond_five_percent() {
    let dir = tempfile::tempdir().unwrap();
    let d = lint_in(
        dir.path(),
        "image-distort",
        "image a \"logo.png\" at (0,0) size (80,80) fit=stretch\n\
         image b \"logo.png\" at (100,0) size (80,41) fit=stretch\n\
         image c \"logo.png\" at (200,0) size (80,80)\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].span.line, 2);
    assert_eq!(
        d[0].message,
        "image 'a' is stretched 50% away from its 200x100 aspect ratio"
    );
    assert_eq!(d[0].facts["natural"], serde_json::json!([200.0, 100.0]));
    assert_eq!(d[0].facts["distortion"], serde_json::json!(0.5));
    assert!(
        d[0].help.iter().any(|h| h.contains("size (80, 40)")),
        "{:?}",
        d[0].help
    );
}

#[test]
fn image_distort_is_quiet_for_kept_ratios_and_other_fits() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        lint_in(
            dir.path(),
            "image-distort",
            "image a \"logo.png\" at (0,0) size (80,40) fit=stretch\n\
         image b \"logo.png\" at (100,0) size (80,80) fit=cover\n\
         image c \"logo.png\" at (200,0) size (80,80) fit=contain\n",
        )
        .is_empty()
    );
}

#[test]
fn image_distort_can_be_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let d = lint_in(
        dir.path(),
        "image-distort",
        "image a \"logo.png\" at (0,0) size (80,80) fit=stretch allow=image-distort\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(d[0].suppressed);
}

#[test]
fn image_distort_flags_stretched_svg_shapes() {
    let dir = tempfile::tempdir().unwrap();
    let d = lint_in(
        dir.path(),
        "image-distort",
        "shape s from \"server\" fit=stretch\nshape c from \"server\"\n\
         box a at (0,0) size (80,40) shape=s\n\
         box b at (100,0) size (60,60) shape=s\n\
         box k at (200,0) size (80,40) shape=c\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].span.line, 4);
    assert_eq!(
        d[0].message,
        "box 'a' is stretched 100% away from its 24x24 aspect ratio"
    );
    assert!(
        d[0].help
            .iter()
            .any(|h| h.contains("fit=cover on shape `s`, or size (80, 80)")),
        "{:?}",
        d[0].help
    );
}

const SVG_HEAD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">"#;

#[test]
fn svg_dropped_flags_lost_images_once_per_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("pic.svg"),
        format!(
            r#"{SVG_HEAD}<image href="photo.png" width="10" height="10"/><image href="https://example.com/x.png" width="10" height="10"/><rect width="5" height="5"/></svg>"#
        ),
    )
    .unwrap();
    let d = lint_in(
        dir.path(),
        "svg-dropped",
        "image a \"pic.svg\" at (0,0) size (40,40)\nbox b \"B\" at (100,0) size (120,40) icon=\"pic.svg\"\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].span.line, 2);
    assert_eq!(
        d[0].message,
        "'pic.svg' drops 2 `<image>` elements when embedded"
    );
    assert_eq!(d[0].facts["dropped"], serde_json::json!({"image": 2}));
    assert_eq!(d[0].help, ["embed it as a path, or use a separate `image`"]);
}

#[test]
fn svg_dropped_is_quiet_for_data_images_and_text() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("ok.svg"),
        format!(
            r#"{SVG_HEAD}<image href="data:image/svg+xml;utf8,&lt;svg xmlns='http://www.w3.org/2000/svg' width='4' height='4'&gt;&lt;rect width='4' height='4'/&gt;&lt;/svg&gt;" width="10" height="10"/><text x="2" y="15" font-size="10">Hi</text></svg>"#
        ),
    )
    .unwrap();
    assert!(
        lint_in(
            dir.path(),
            "svg-dropped",
            "image a \"ok.svg\" at (0,0) size (40,40)\n"
        )
        .is_empty()
    );
}
