mod common;

use common::*;
use kashika_check::{Omitted, check_bounded, check_deferred};
use kashika_diag::Diagnostic;
use kashika_model::Scene;

/// Five horizontal lines crossing five vertical ones: 25 crossings, plus a line along a box.
fn grid() -> Scene {
    let mut body = String::from("box a at (600,100) size (80,60)\nline n (560,163) -> h 150\n");
    for i in 0..5 {
        let at = 50 + 60 * i;
        body.push_str(&format!("line h{i} (20,{at}) -> h 300\n"));
        body.push_str(&format!("line v{i} ({at},20) -> v 300\n"));
    }
    scene(&body)
}

fn all(_: &Diagnostic) -> bool {
    true
}

#[test]
fn a_roomy_limit_reports_what_check_deferred_does() {
    let s = grid();
    let b = check_bounded(&s, 1000, &all);
    assert!(b.omitted.is_empty());
    assert_eq!(b.diags, check_deferred(&s));
    assert_eq!(b.metrics(&s), kashika_check::metrics(&s, &b.diags));
}

#[test]
fn past_the_limit_findings_are_counted_and_the_first_in_order_kept() {
    let s = grid();
    let b = check_bounded(&s, 2, &all);
    let mut crosses: Vec<Diagnostic> = check_deferred(&s)
        .into_iter()
        .filter(|d| d.code == "cross")
        .collect();
    assert_eq!(crosses.len(), 25);
    kashika_diag::sort(&mut crosses);
    let kept: Vec<&Diagnostic> = b.diags.iter().filter(|d| d.code == "cross").collect();
    assert_eq!(kept, crosses.iter().take(2).collect::<Vec<_>>());
    assert_eq!(
        b.omitted,
        [Omitted {
            code: "cross",
            suppressed: false,
            count: 23
        }]
    );
    assert!(b.diags.iter().any(|d| d.code == "near"));
    assert_eq!(b.found("cross"), 25);
    assert_eq!(b.metrics(&s).crossings, 25);
}

#[test]
fn a_zero_limit_only_counts() {
    let s = grid();
    let b = check_bounded(&s, 0, &all);
    assert!(b.diags.is_empty());
    let total: usize = b.omitted.iter().map(|o| o.count).sum();
    assert_eq!(total, check_deferred(&s).len());
}

#[test]
fn the_filter_drops_findings_before_the_limit_but_metrics_count_them() {
    let s = grid();
    let only_h0 = |d: &Diagnostic| d.elements.iter().any(|e| e.id == "h0");
    let b = check_bounded(&s, 1000, &only_h0);
    assert_eq!(b.diags.len(), 5, "{:#?}", b.diags);
    assert!(b.diags.iter().all(only_h0));
    assert_eq!(b.metrics(&s).crossings, 25);
}

#[test]
fn suppressed_findings_have_their_own_limit() {
    let s = scene(
        "line a (0,50) -> h 100 allow=cross\nline b (50,0) -> v 100\nline c (60,0) -> v 100\n\
         line d (0,80) -> h 100\n",
    );
    let b = check_bounded(&s, 1, &all);
    let kept: Vec<bool> = b
        .diags
        .iter()
        .filter(|d| d.code == "cross")
        .map(|d| d.suppressed)
        .collect();
    assert_eq!(kept.len(), 2, "{:#?}", b.diags);
    assert!(kept.contains(&true) && kept.contains(&false));
    assert_eq!(
        b.omitted,
        [
            Omitted {
                code: "cross",
                suppressed: false,
                count: 1
            },
            Omitted {
                code: "cross",
                suppressed: true,
                count: 1
            }
        ]
    );
}
