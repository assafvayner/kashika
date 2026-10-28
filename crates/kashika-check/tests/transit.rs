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

const TRUNK: &str = "box src at (0,40) size (40,20)\n\
group r2 at (100,0) size (100,100) {\n  box b at (30,60) size (40,20)\n}\n\
group r3 at (300,0) size (100,100) {\n  box c at (30,40) size (40,20)\n}\n\
line trunk src.right -> c.left\n";

#[test]
fn a_trunk_whose_junction_taps_into_the_group_connects_to_it() {
    let outside = format!("{TRUNK}junction mid at (80,50)\nline tap mid -> vh b.left\n");
    assert!(lint("group-transit", &outside).is_empty());
    let inside = format!("{TRUNK}junction mid at (150,50)\nline tap mid -> b.top\n");
    assert!(lint("group-transit", &inside).is_empty());
}

#[test]
fn a_trunk_without_a_tap_into_the_group_still_transits_it() {
    let d = lint("group-transit", TRUNK);
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "trunk passes through group 'r2' without connecting to anything inside it"
    );
    let elsewhere = format!(
        "{TRUNK}box e at (60,200) size (40,20)\njunction mid at (80,50)\nline tap mid -> e.top\n"
    );
    assert_eq!(lint("group-transit", &elsewhere).len(), 1);
    let off_the_trunk = format!("{TRUNK}junction mid at (80,90)\nline tap mid -> h 50\n");
    assert_eq!(lint("group-transit", &off_the_trunk).len(), 1);
}
