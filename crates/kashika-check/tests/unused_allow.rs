mod common;

use common::*;
use kashika_check::check_bounded;
use kashika_check::ctx::Ctx;
use kashika_diag::Span;

const CROSSING: &str = "line a (0,50) -> h 100\nline b (50,0) -> v 100\n";

#[test]
fn reports_each_stale_entry_at_the_entry() {
    let d = lint(
        "unused-allow",
        &format!("{CROSSING}line c (0,150) -> h 100 allow=cross:a,near reason=\"x\"\n"),
    );
    assert_eq!(d.len(), 2, "{d:#?}");
    assert_eq!(d[0].message, "`allow=cross:a` on c suppresses nothing");
    assert_eq!(d[0].span, Span::new(4, 31, 4, 38));
    assert_eq!(d[1].message, "`allow=near` on c suppresses nothing");
    assert_eq!(d[1].span, Span::new(4, 39, 4, 43));
    assert!(d.iter().all(|d| !d.suppressed && d.elements[0].id == "c"));
    assert!(d[0].help[0].contains("`reason=`"), "{:?}", d[0].help);
}

#[test]
fn a_used_entry_is_not_reported() {
    let body = CROSSING.replace("v 100", "v 100 allow=cross:a,near reason=\"bus\"");
    let d = lint("unused-allow", &body);
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "`allow=near` on b suppresses nothing");
    assert_eq!(d[0].help, ["remove it"]);
    assert!(
        lint(
            "unused-allow",
            &CROSSING.replace("v 100", "v 100 allow=cross")
        )
        .is_empty()
    );
}

#[test]
fn shapes_name_their_kind() {
    let d = lint(
        "unused-allow",
        "box card at (20,20) size (60,40) allow=overlap\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "`allow=overlap` on box 'card' suppresses nothing"
    );
}

#[test]
fn allow_unused_allow_silences_the_others() {
    let d = lint(
        "unused-allow",
        "box card at (20,20) size (60,40) allow=overlap,unused-allow\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(d[0].suppressed);
    assert!(
        lint(
            "unused-allow",
            "box card at (20,20) size (60,40) allow=unused-allow\n"
        )
        .is_empty()
    );
}

#[test]
fn output_limits_do_not_make_entries_unused() {
    let s = scene(&CROSSING.replace("v 100", "v 100 allow=cross:a"));
    let only_unused = |d: &kashika_diag::Diagnostic| d.code == "unused-allow";
    let b = check_bounded(&Ctx::new(&s), 0, &only_unused);
    assert_eq!(b.found("unused-allow"), 0);
    let b = check_bounded(&Ctx::new(&s), 1, &|d| {
        d.elements.iter().any(|e| e.id == "z")
    });
    assert_eq!(b.found("unused-allow"), 0);
}
