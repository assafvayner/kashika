//! Parity of the geometric prefilter, [`Field::estimate`], with `kashika check`.
//!
//! The prefilter drops a plan outright (`None`) for three reasons: it doubles back, it leaves
//! or enters a side anchor the wrong way, or it enters a box. The first two are not findings
//! of `check` (`rank` drops such paths on its own), so the invariant is about the third:
//!
//! **A plan the prefilter drops for entering a box is one `check` reports `through` for**, so
//! the prefilter never drops a candidate that `check` finds clear of boxes. Its crossing,
//! transit and near-miss counts only rank the plans it keeps and are not checked here.

use std::path::{Path, PathBuf};

use kashika_check::ctx::Ctx;
use kashika_geom::{Pt, dot};
use kashika_model::{Options, Scene};

use super::*;
use crate::endpoint::{PROBE, resolve_endpoints};
use crate::source::Source;
use crate::text::{path_text, statement};
use crate::{Frame, SQUARE, ast_lines, existing, probe_id, score};

/// A line to route, with what `rank` needs to write its probes.
struct Case {
    name: String,
    src: String,
    opts: Options,
    from: Endpoint,
    to: Endpoint,
    /// The line being rerouted, which isn't an obstacle.
    line: Option<String>,
    arrow: bool,
    attrs: String,
    frame: Frame,
}

fn scene_of(src: &str, opts: &Options) -> Scene {
    kashika_model::compile_with(src, opts).0
}

/// Every named line of every example, rerouted as `kashika route --line` would.
fn example_cases() -> Vec<Case> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .flat_map(|d| std::fs::read_dir(d.unwrap().path()).into_iter().flatten())
        .map(|f| f.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "kash"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for path in files {
        let src = std::fs::read_to_string(&path).unwrap();
        let opts = Options {
            path: Some(path.clone()),
            ..Options::default()
        };
        let scene = scene_of(&src, &opts);
        let (file, _) = kashika_syntax::parse(&src);
        let source = Source::new(&src);
        let ids: Vec<String> = ast_lines(&file.stmts)
            .into_iter()
            .filter_map(|l| Some(l.id.as_ref()?.name.clone()))
            .collect();
        for id in ids {
            let Ok(t) = existing(&file, &scene, &source, &id) else {
                continue;
            };
            out.push(Case {
                name: format!("{}:{id}", path.display()),
                src: src.clone(),
                opts: opts.clone(),
                from: t.from,
                to: t.to,
                line: Some(id),
                arrow: t.arrow,
                attrs: crate::attrs(Some(&t.attrs), Style::Corner, 0.0),
                frame: Frame {
                    origin: t.origin,
                    after: Some(t.end),
                },
            });
        }
    }
    out
}

const SYNTHETIC: &[(&str, &str, &str)] = &[
    (
        "diagram \"wall\" size (460,240)\n\
         box a \"A\" at (20,100) size (100,40)\n\
         box wall \"W\" at (200,40) size (40,160)\n\
         box b \"B\" at (320,100) size (100,40)\n",
        "a.right",
        "b.left",
    ),
    (
        "diagram \"maze\" size (520,320)\n\
         box a at (20,20) size (80,40)\n\
         box p at (140,0) size (40,200)\n\
         box q at (260,120) size (40,200)\n\
         box r at (360,40) size (120,30) shape=diamond\n\
         box b at (400,240) size (80,40)\n",
        "a.bottom",
        "b.top",
    ),
    (
        "diagram \"groups\" size (520,320)\n\
         group g \"G\" at (160,40) size (200,240) {\n  box inner at (40,60) size (80,40)\n}\n\
         box a at (20,140) size (80,40)\n\
         box b at (420,140) size (80,40)\n\
         line down inner.bottom -> (260,300)\n",
        "a.right",
        "b.left",
    ),
    (
        "diagram \"corner\" size (400,300)\n\
         box a at (20,20) size (100,60)\n\
         box b at (260,200) size (100,60)\n\
         box mid at (150,110) size (80,60) shape=ellipse\n",
        "(70,80)",
        "b.top",
    ),
];

fn synthetic_cases() -> Vec<Case> {
    SYNTHETIC
        .iter()
        .enumerate()
        .map(|(i, &(src, from, to))| {
            let opts = Options::default();
            let (_, diags) = kashika_model::compile_with(src, &opts);
            assert!(diags.is_empty(), "synthetic {i}: {diags:#?}");
            let (from, to) = resolve_endpoints(src, from, to, &opts).unwrap();
            Case {
                name: format!("synthetic {i}"),
                src: src.to_string(),
                opts,
                from,
                to,
                line: None,
                arrow: false,
                attrs: crate::attrs(None, Style::Corner, 0.0),
                frame: Frame::default(),
            }
        })
        .collect()
}

/// The one-lane plans and detours `orthogonal` builds before keeping the best few, with fewer
/// four-elbow detours to keep the test quick.
fn all_plans(ctx: &Ctx, a: &Endpoint, b: &Endpoint, skip: &[&str]) -> Vec<Plan> {
    let mut out = Vec::new();
    for (axis, cross) in [(Axis::X, Hop::Vh), (Axis::Y, Hop::Hv)] {
        for lane in lane_pool(ctx, a, b, axis, skip) {
            out.push(Plan {
                hops: vec![hop(axis, &lane), cross.clone()],
                lane: Some((axis, lane.value)),
            });
        }
    }
    let xs = detour_lanes(ctx, a, b, Axis::X, skip);
    let ys = detour_lanes(ctx, a, b, Axis::Y, skip);
    for (lead, back) in [(Axis::X, Axis::Y), (Axis::Y, Axis::X)] {
        let (outer, inner) = if lead == Axis::X {
            (&xs, &ys)
        } else {
            (&ys, &xs)
        };
        let (cross, along) = if lead == Axis::X {
            (Hop::Hv, Hop::Vh)
        } else {
            (Hop::Vh, Hop::Hv)
        };
        for l1 in outer {
            for l2 in inner {
                let lane = Some((back, l2.value));
                out.push(Plan {
                    hops: vec![hop(lead, l1), hop(back, l2), cross.clone()],
                    lane,
                });
                for l3 in outer.iter().take(4) {
                    out.push(Plan {
                        hops: vec![hop(lead, l1), hop(back, l2), hop(lead, l3), along.clone()],
                        lane,
                    });
                }
            }
        }
    }
    out
}

/// Whether the prefilter drops `pts` for entering a box, rather than for its shape.
fn dropped_for_a_box(field: &Field, pts: &[Pt]) -> bool {
    let n = pts.len() - 1;
    let dir = |i: usize| (pts[i + 1] - pts[i]).norm();
    let wrong_way = field
        .from
        .side
        .is_some_and(|s| dot(dir(0), s.outward()) <= SQUARE)
        || field
            .to
            .side
            .is_some_and(|s| dot(dir(n - 1), s.outward()) >= -SQUARE);
    !doubles_back(pts) && !wrong_way && field.estimate(pts).is_none()
}

/// For each plan, whether `check` keeps it clear of `through`, as `rank` scores it; `None`
/// when its probe doesn't resolve.
fn clear_of_boxes(case: &Case, scene: &Scene, plans: &[Plan]) -> Vec<Option<bool>> {
    let src = Source::new(&case.src);
    let probes: Vec<String> = plans
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let path = path_text(&case.from, &case.to, p, case.frame.origin);
            statement(case.arrow, &probe_id(i), &path, &case.attrs)
        })
        .collect();
    let (text, _) = src.with_lines(case.frame.after, &probes);
    let probed = scene_of(&text, &case.opts);
    let Some(at) = probed.lines.iter().position(|l| l.id.starts_with(PROBE)) else {
        return vec![None; plans.len()];
    };
    let mut work = scene.clone();
    (0..plans.len())
        .map(|i| {
            let line = probed.lines[at..].iter().find(|l| l.id == probe_id(i))?;
            work.lines.insert(at, line.clone());
            let s = score::score(&work, &work.lines[at], case.line.as_deref());
            work.lines.remove(at);
            Some(s.is_some())
        })
        .collect()
}

/// Probes scored with `check` per case, spread over the dropped plans.
const SAMPLE: usize = 24;

#[test]
fn the_prefilter_drops_only_plans_check_finds_through_a_box() {
    let cases: Vec<Case> = synthetic_cases()
        .into_iter()
        .chain(example_cases())
        .collect();
    assert!(cases.len() > 40, "only {} cases", cases.len());
    let (mut considered, mut dropped, mut scored) = (0, 0, 0);
    let mut wrong = Vec::new();
    for case in &cases {
        let scene = scene_of(&case.src, &case.opts);
        let ctx = Ctx::new(&scene);
        let skip: Vec<&str> = case.line.as_deref().into_iter().collect();
        let field = Field::new(&ctx, &case.from, &case.to, &skip);
        let plans = all_plans(&ctx, &case.from, &case.to, &skip);
        considered += plans.len();
        let mut rejected: Vec<Plan> = plans
            .into_iter()
            .filter(|p| {
                let pts = corners(case.from.at, case.to.at, p).expect("orthogonal hops");
                dropped_for_a_box(&field, &pts)
            })
            .collect();
        dropped += rejected.len();
        let step = rejected.len().div_ceil(SAMPLE).max(1);
        rejected = rejected.into_iter().step_by(step).collect();
        for (plan, clear) in rejected.iter().zip(clear_of_boxes(case, &scene, &rejected)) {
            let Some(clear) = clear else { continue };
            scored += 1;
            if clear {
                let path = path_text(&case.from, &case.to, plan, case.frame.origin);
                wrong.push(format!("{}: {path}", case.name));
            }
        }
    }
    assert!(
        dropped > 0 && scored > 0,
        "nothing dropped of {considered} plans"
    );
    assert!(
        wrong.is_empty(),
        "the prefilter dropped {} of {scored} scored plans that check finds clear of boxes:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
