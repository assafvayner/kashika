use kashika_diag::{Diagnostic, code, fact};
use kashika_geom::{Hit, Pt, TOLERANCE, dist_point_seg, seg_hit};

use crate::ctx::{Ctx, LineGeom, fmt_pt};
use crate::element::{arr, line_el};
use crate::free::{Finding, with_free};
use crate::sink::Sink;

/// `cross`: two lines meet at a point that is neither a shared endpoint nor a junction.
pub fn cross(ctx: &Ctx, out: &mut Sink) {
    for (i, a) in ctx.lines.iter().enumerate() {
        for j in ctx.lines_near(ctx.line_boxes[i], TOLERANCE) {
            if j <= i {
                continue;
            }
            let b = &ctx.lines[j];
            for (ka, kb, p) in crossings(ctx, a, b) {
                out.push(report(b, b.pieces[kb].2, a, a.pieces[ka].2, p));
            }
        }
    }
}

/// One point per crossing, with the indices (into `pieces`) of the pieces that meet there.
fn crossings(ctx: &Ctx, a: &LineGeom, b: &LineGeom) -> Vec<(usize, usize, Pt)> {
    let mut shared: Vec<(Pt, Pt)> = Vec::new();
    let mut hits: Vec<(usize, usize, Pt)> = Vec::new();
    for (ka, &(a0, a1, ai)) in a.pieces.iter().enumerate() {
        for (kb, &(b0, b1, bi)) in b.pieces.iter().enumerate() {
            match seg_hit(a0, a1, b0, b1, TOLERANCE) {
                Some(Hit::Point(p)) => hits.push((ka, kb, p)),
                Some(Hit::Overlap(p, q)) if a.straight(ai) && b.straight(bi) => shared.push((p, q)),
                Some(Hit::Overlap(p, q)) => hits.push((ka, kb, p.lerp(q, 0.5))),
                None => {}
            }
        }
    }
    let near = |q: Pt, p: Pt| q.dist(p) <= TOLERANCE;
    hits.retain(|&(_, _, p)| {
        let shared_end =
            (near(a.start, p) || near(a.end, p)) && (near(b.start, p) || near(b.end, p));
        !shared_end
            && !ctx.is_junction(p)
            && !shared
                .iter()
                .any(|&(s, e)| dist_point_seg(p, s, e) <= TOLERANCE)
    });
    // Hits on neighboring flattened pieces of a curve are one crossing (or one tangency).
    let mut clusters: Vec<Vec<(usize, usize, Pt)>> = Vec::new();
    for h in hits {
        let joins = |c: &&mut Vec<(usize, usize, Pt)>| {
            c.iter().any(|&(x, y, q)| {
                q.dist(h.2) <= 1.0
                    || (x.abs_diff(h.0) <= 1
                        && y.abs_diff(h.1) <= 1
                        && !(a.straight(a.pieces[x].2) && b.straight(b.pieces[y].2)))
            })
        };
        match clusters.iter_mut().find(joins) {
            Some(c) => c.push(h),
            None => clusters.push(vec![h]),
        }
    }
    clusters
        .into_iter()
        .map(|c| {
            let p = c.iter().fold(Pt::default(), |s, h| s + h.2) * (1.0 / c.len() as f64);
            (c[0].0, c[0].1, p)
        })
        .collect()
}

fn report(l: &LineGeom, li: usize, other: &LineGeom, oi: usize, p: Pt) -> Finding {
    let (id, oid) = (l.line.id.as_str(), other.line.id.as_str());
    let d = Diagnostic::warn(
        code::CROSS,
        l.line.span,
        format!("{id} crosses {oid} at {}", fmt_pt(p)),
    )
    .with_element(line_el(l.line, Some(l.segment(li))))
    .with_element(line_el(other.line, Some(other.segment(oi))))
    .with_fact(fact::AT, arr(p));
    let allow = if other.line.named {
        format!("if intentional: allow=cross:{oid} on {id}")
    } else {
        format!(
            "if intentional: name {oid} and add allow=cross:<its id> on {id}, or allow=cross on {id}"
        )
    };
    with_free(d, l, li, &[id]).with_help(allow)
}
