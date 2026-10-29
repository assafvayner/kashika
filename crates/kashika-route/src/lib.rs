mod candidates;
mod endpoint;
mod score;
mod source;
#[cfg(test)]
mod tests;
mod text;

use std::collections::HashMap;

use kashika_check::ctx::Ctx;
use kashika_diag::{Diagnostic, Severity};
use kashika_geom::{PathSeg, Pt, flatten, fmt_num};
use kashika_model::{Line, Scene};
use kashika_syntax::ast;
use serde::Serialize;

use crate::candidates::{LaneAnchors, Plan, corners, generate, key};
use crate::endpoint::{PROBE, end_point, resolve_endpoints};
use crate::source::Source;
use crate::text::{path_text, statement};

pub use endpoint::{Endpoint, Side};
pub use kashika_check::free::Axis;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    Corner,
    Arc,
    Bezier,
}

#[derive(Debug)]
pub enum Error {
    /// Bad arguments; the CLI exits 3.
    Usage(String),
    /// `<from>` or `<to>` doesn't resolve; the CLI exits 2.
    Diagnostics(Vec<Diagnostic>),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Crossing {
    pub line: String,
    pub at: Pt,
}

/// A group the path passes through without connecting to anything inside it, and where it
/// enters, as `check` reports with `group-transit`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Transit {
    pub group: String,
    pub at: Pt,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Corridor {
    /// `y` for a horizontal lane (the band is a y range), `x` for a vertical one.
    pub axis: Axis,
    pub band: [f64; 2],
}

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub crossings: usize,
    pub crossed: Vec<Crossing>,
    pub transits: usize,
    pub transited: Vec<Transit>,
    pub near: usize,
    pub segments: usize,
    pub length: f64,
    pub corridor: Option<Corridor>,
    /// A ready-to-paste `line` statement, or `arrow` when rerouting one.
    pub statement: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub from: Endpoint,
    pub to: Endpoint,
    pub style: Style,
    pub candidates: Vec<Candidate>,
    /// Existing lines between the same two shapes, in either direction, when no `--line` was
    /// given; they count as obstacles for the candidates.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub existing_lines: Vec<String>,
}

pub struct Request<'a> {
    pub src: &'a str,
    pub from: Option<&'a str>,
    pub to: Option<&'a str>,
    pub style: Style,
    /// `--line <id>`: reroute this existing line.
    pub line: Option<&'a str>,
    pub radius: f64,
    pub max: usize,
    /// How the input's files resolve.
    pub opts: &'a kashika_model::Options,
}

pub fn suggest(req: &Request) -> Result<Suggestion, Error> {
    let (file, _) = kashika_syntax::parse(req.src);
    let (scene, _) = kashika_model::compile_with(req.src, req.opts);
    let src = Source::new(req.src);
    let target = req
        .line
        .map(|id| existing(&file, &scene, &src, id))
        .transpose()?;
    let (from, to) = match (req.from, req.to, &target) {
        (Some(f), Some(t), _) => resolve_endpoints(req.src, f, t, req.opts)?,
        (None, None, Some(t)) => (t.from.clone(), t.to.clone()),
        _ => {
            return Err(Error::Usage(
                "give `<from> <to>`, or `--line <id>` to reroute a line".into(),
            ));
        }
    };
    let arrow = target.as_ref().is_some_and(|t| t.arrow);
    let (id, attrs, frame) = match &target {
        Some(t) => (
            t.id.clone(),
            attrs(Some(&t.attrs), req.style, req.radius),
            Frame {
                origin: t.origin,
                after: Some(t.end),
            },
        ),
        None => (
            default_id(&scene, &from, &to),
            attrs(None, req.style, req.radius),
            Frame::default(),
        ),
    };
    let existing_lines = match target {
        Some(_) => Vec::new(),
        None => connecting_lines(&file, &scene, &from, &to),
    };
    let candidates = rank(req, &scene, &src, (&from, &to), (arrow, &id, &attrs), frame);
    Ok(Suggestion {
        from,
        to,
        style: req.style,
        candidates,
        existing_lines,
    })
}

/// Named lines that resolve and run between the owners of `from` and `to`, either way.
fn connecting_lines(
    file: &ast::File,
    scene: &Scene,
    from: &Endpoint,
    to: &Endpoint,
) -> Vec<String> {
    let (Some(a), Some(b)) = (&from.owner, &to.owner) else {
        return Vec::new();
    };
    fn owner(p: Option<&ast::Point>) -> Option<&str> {
        match p.map(ast::Point::base) {
            Some(ast::Point::Ref { r, .. }) => Some(r.id.name.as_str()),
            _ => None,
        }
    }
    let resolves = |id: &str| scene.lines.iter().any(|l| l.id == id && !l.path.is_empty());
    ast_lines(&file.stmts)
        .into_iter()
        .filter_map(|l| {
            let id = &l.id.as_ref()?.name;
            let (x, y) = (owner(Some(&l.start))?, owner(end_point(l))?);
            ((x == a && y == b || x == b && y == a) && resolves(id)).then(|| id.clone())
        })
        .collect()
}

fn ast_lines(stmts: &[ast::Stmt]) -> Vec<&ast::Line> {
    let mut out = Vec::new();
    for s in stmts {
        match s {
            ast::Stmt::Line(l) => out.push(l),
            ast::Stmt::Group(g) => out.extend(ast_lines(&g.children)),
            _ => {}
        }
    }
    out
}

/// Where a candidate is written: the origin of its frame, and the source position right
/// after which the probes go, so their literals mean the same as in the final statement.
#[derive(Default, Clone, Copy)]
struct Frame {
    origin: Pt,
    /// 1-based line and exclusive column; `None` appends the probes at the end of the file.
    after: Option<(u32, u32)>,
}

/// An existing line to reroute: its endpoints as written, its attributes' source text, the
/// origin of the frame its literals are written in and where its statement ends.
struct Target {
    id: String,
    /// Written as an `arrow` statement.
    arrow: bool,
    from: Endpoint,
    to: Endpoint,
    attrs: Vec<String>,
    origin: Pt,
    end: (u32, u32),
}

fn existing(file: &ast::File, scene: &Scene, src: &Source, id: &str) -> Result<Target, Error> {
    let ast_line = ast_lines(&file.stmts)
        .into_iter()
        .find(|l| l.id.as_ref().is_some_and(|i| i.name == id))
        .ok_or_else(|| Error::Usage(format!("no line `{id}` in the file")))?;
    let line = scene.lines.iter().find(|l| l.id == id).ok_or_else(|| {
        Error::Usage(format!(
            "line `{id}` doesn't resolve; fix it with `kashika check` first"
        ))
    })?;
    let (Some(first), Some(last)) = (line.path.first(), line.path.last()) else {
        return Err(Error::Usage(format!("line `{id}` has no segments")));
    };
    let endpoint = |p: Option<&ast::Point>, at: Pt| match p {
        Some(p) => Endpoint::of(p, src.slice(p.span()), at),
        None => Endpoint::literal(at),
    };
    Ok(Target {
        id: id.to_string(),
        arrow: ast_line.arrow,
        from: endpoint(Some(&ast_line.start), first.from()),
        to: endpoint(end_point(ast_line), last.to()),
        attrs: ast_line.attrs.iter().map(|a| src.slice(a.span)).collect(),
        origin: line.origin,
        end: (ast_line.span.end_line, ast_line.span.end_col),
    })
}

fn probe_id(i: usize) -> String {
    format!("{PROBE}_{i}")
}

/// Scores every plan as a real line. All probes are resolved in one compile, then each is
/// checked on its own against the scene, in the place its statement would take.
fn rank(
    req: &Request,
    scene: &Scene,
    src: &Source,
    (from, to): (&Endpoint, &Endpoint),
    (arrow, id, attrs): (bool, &str, &str),
    frame: Frame,
) -> Vec<Candidate> {
    let ctx = Ctx::new(scene);
    let skip: Vec<&str> = req.line.into_iter().collect();
    let plans = generate(&ctx, from, to, req.style, &skip);
    let paths: Vec<String> = plans
        .iter()
        .map(|p| path_text(from, to, p, frame.origin))
        .collect();
    let probes: Vec<String> = paths
        .iter()
        .enumerate()
        .map(|(i, p)| statement(arrow, &probe_id(i), p, attrs))
        .collect();
    let (text, first) = src.with_lines(frame.after, &probes);
    let (probed, diags) = kashika_model::compile_with(&text, req.opts);
    let failed: Vec<u32> = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .filter_map(|d| d.span.line.checked_sub(first))
        .collect();
    let Some(at) = probed.lines.iter().position(|l| l.id.starts_with(PROBE)) else {
        return Vec::new();
    };
    let resolved: HashMap<&str, &Line> = probed.lines[at..]
        .iter()
        .map(|l| (l.id.as_str(), l))
        .collect();
    let mut work = scene.clone();
    let mut seen: Vec<Vec<(i64, i64)>> = Vec::new();
    let mut candidates = Vec::new();
    let mut detour = Vec::new();
    let mut ties = Vec::new();
    for (i, (plan, path)) in plans.iter().zip(&paths).enumerate() {
        if failed.contains(&(i as u32)) {
            continue;
        }
        let Some(&line) = resolved.get(probe_id(i).as_str()) else {
            continue;
        };
        if !directions_ok(line, from, to, req.style) {
            continue;
        }
        let shape = key(flatten(&line.path));
        if seen.contains(&shape) {
            continue;
        }
        seen.push(shape);
        work.lines.insert(at, line.clone());
        let s = score::score(&work, &work.lines[at], req.line);
        work.lines.remove(at);
        let Some(s) = s else {
            continue;
        };
        let corridor = plan.lane.and_then(|(axis, v)| {
            let span = lane_span(from, to, plan, axis, v)?;
            score::corridor(&ctx, axis, v, span, &skip)
        });
        detour.push(plan.hops.len() >= 3);
        ties.push(plan.lane_anchors(from, to));
        candidates.push(Candidate {
            crossings: s.crossed.len(),
            crossed: s.crossed,
            transits: s.transited.len(),
            transited: s.transited,
            near: s.near,
            segments: segments(&line.path, req.style),
            length: s.length.round(),
            corridor,
            statement: statement(arrow, id, path, attrs),
        });
    }
    let simple: Vec<&Candidate> = candidates
        .iter()
        .zip(&detour)
        .filter_map(|(c, d)| (!d).then_some(c))
        .collect();
    let drop: Vec<bool> = candidates
        .iter()
        .zip(&detour)
        .map(|(c, d)| *d && dominated(c, &simple))
        .collect();
    let mut ranked: Vec<(LaneAnchors, Candidate)> = ties
        .into_iter()
        .zip(candidates)
        .zip(drop)
        .filter_map(|(c, d)| (!d).then_some(c))
        .collect();
    ranked.sort_by(|(ta, a), (tb, b)| {
        (a.crossings + a.transits, a.near, a.segments)
            .cmp(&(b.crossings + b.transits, b.near, b.segments))
            .then(a.length.total_cmp(&b.length))
            .then(ta.cmp(tb))
    });
    ranked.into_iter().take(req.max).map(|(_, c)| c).collect()
}

/// A detour is dropped when a simpler candidate is at least as good on crossings and
/// transits, near-misses and length.
fn dominated(c: &Candidate, simple: &[&Candidate]) -> bool {
    simple.iter().any(|s| {
        s.segments < c.segments
            && s.crossings + s.transits <= c.crossings + c.transits
            && s.near <= c.near
            && s.length <= c.length
    })
}

/// The extent, on the other axis, of the segment that runs along a plan's lane.
fn lane_span(
    from: &Endpoint,
    to: &Endpoint,
    plan: &Plan,
    axis: Axis,
    v: f64,
) -> Option<(f64, f64)> {
    let pts = corners(from.at, to.at, plan)?;
    pts.windows(2).find_map(|w| {
        let (p, q) = (w[0], w[1]);
        if axis == Axis::X && p.x == v && q.x == v {
            Some((p.y, q.y))
        } else if axis == Axis::Y && p.y == v && q.y == v {
            Some((p.x, q.x))
        } else {
            None
        }
    })
}

/// Orthogonal styles must stay orthogonal. A path must leave a side anchor along its outward
/// normal and enter a side anchor against it, so the arrowhead lands square on the edge.
fn directions_ok(line: &Line, from: &Endpoint, to: &Endpoint, style: Style) -> bool {
    let (Some(first), Some(last)) = (line.path.first(), line.path.last()) else {
        return false;
    };
    if style != Style::Bezier
        && line
            .path
            .iter()
            .any(|s| matches!(s, PathSeg::Line { .. }) && !s.is_axis_aligned())
    {
        return false;
    }
    let dot = |a: Pt, b: Pt| a.x * b.x + a.y * b.y;
    from.side
        .is_none_or(|s| dot(first.start_dir(), s.outward()) > SQUARE)
        && to
            .side
            .is_none_or(|s| dot(last.end_dir(), s.outward()) < -SQUARE)
}

/// Cosine of the largest angle, about 2.5°, between a path's end and a side's normal.
pub(crate) const SQUARE: f64 = 0.999;

fn segments(path: &[PathSeg], style: Style) -> usize {
    match style {
        Style::Arc => {
            1 + path
                .iter()
                .filter(|s| matches!(s, PathSeg::Quad { .. }))
                .count()
        }
        Style::Corner | Style::Bezier => path.len(),
    }
}

fn default_id(scene: &Scene, from: &Endpoint, to: &Endpoint) -> String {
    let base = format!(
        "{}_{}",
        from.owner.as_deref().unwrap_or("p"),
        to.owner.as_deref().unwrap_or("p")
    );
    let taken = |id: &str| {
        scene.shapes.iter().any(|s| s.id == id)
            || scene.lines.iter().any(|l| l.id == id)
            || scene.junctions.iter().any(|j| j.id == id)
            || scene.texts.iter().any(|t| t.id.as_deref() == Some(id))
    };
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}_{n}"))
        .find(|c| !taken(c))
        .expect("some suffix is free")
}

/// Attributes for the suggested statement, which the probes share: the rerouted line's own,
/// minus `radius` (and `ortho` for curves), or `end=arrow` for a new line; `arc` adds `radius=N`.
fn attrs(existing: Option<&[String]>, style: Style, radius: f64) -> String {
    let mut out: Vec<String> = match existing {
        Some(list) => list
            .iter()
            .filter(|a| !a.starts_with("radius=") && !(style == Style::Bezier && *a == "ortho"))
            .cloned()
            .collect(),
        None => vec!["end=arrow".into()],
    };
    if style == Style::Arc {
        out.push(format!("radius={}", fmt_num(radius)));
    }
    out.join(" ")
}
