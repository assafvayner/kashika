mod common;

use common::*;

#[test]
fn literal_endpoint_near_an_outline() {
    let d = lint(
        "endpoint-off-edge",
        "box a at (100,100) size (100,60)\nline l (0,115) -> (97,115)\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "l ends at (97,115), 3px from the outline of 'a'"
    );
    assert_eq!(
        d[0].help,
        vec!["snap it to the outline with an anchor: `a.left@0.25`".to_string()]
    );
    assert!(
        lint(
            "endpoint-off-edge",
            "box a at (100,100) size (100,60)\nline l (0,115) -> (100,115)\n"
        )
        .is_empty()
    );
    assert!(
        lint(
            "endpoint-off-edge",
            "box a at (100,100) size (100,60)\nline l (0,115) -> a.left@0.25+(-3,0)\n"
        )
        .is_empty()
    );
    assert!(
        lint(
            "endpoint-off-edge",
            "box a at (100,100) size (100,60)\nline l (0,115) -> (80,115)\n"
        )
        .is_empty()
    );
}

#[test]
fn snap_distance_of_8px_is_inclusive() {
    let d = lint(
        "endpoint-off-edge",
        "box a at (100,100) size (100,80)\nline end_flag (0,110) -> (92,110)\nline end_clean (0,125) -> (91.9,125)\nline start_flag (92,140) -> (0,140)\nline start_clean (91.9,155) -> (0,155)\n",
    );
    assert_eq!(d.len(), 2, "{d:#?}");
    assert_eq!(
        d[0].message,
        "end_flag ends at (92,110), 8px from the outline of 'a'"
    );
    assert_eq!(
        d[1].message,
        "start_flag starts at (92,140), 8px from the outline of 'a'"
    );
}
