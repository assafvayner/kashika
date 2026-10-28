mod common;

use common::*;

#[test]
fn line_crossing_a_group_it_does_not_connect_to() {
    let d = lint(
        "group-transit",
        "group g at (100,0) size (100,100) {\n}\nline l (0,50) -> h 300\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "l passes through group 'g' without connecting to anything inside it"
    );
    assert!(lint(
        "group-transit",
        "group g at (100,0) size (100,100) {\n  box a at (40,40) size (20,20)\n}\nline l (0,50) -> a.left\n"
    )
    .is_empty());
}
