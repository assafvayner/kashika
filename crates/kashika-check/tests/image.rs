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
