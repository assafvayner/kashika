mod common;

use common::*;

#[test]
fn diagonal_last_hop_after_orthogonal_ones() {
    let d = lint(
        "diagonal-tail",
        "line l (0,0) -> h 100 -> v 50 -> (150,100)\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "the last segment of l runs diagonally from (100,50) to (150,100)"
    );
    assert_eq!(d[0].elements[0].segment, Some(3));
    assert!(lint("diagonal-tail", "line l (0,0) -> (150,100)\n").is_empty());
    assert!(lint("diagonal-tail", "line l (0,0) -> (50,20) -> (150,100)\n").is_empty());
    assert!(lint("diagonal-tail", "line l (0,0) -> h 100 -> v 50\n").is_empty());
}
