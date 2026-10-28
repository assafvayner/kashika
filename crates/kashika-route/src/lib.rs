mod candidates;
mod endpoint;
mod score;
mod text;

use kashika_diag::{Diagnostic, Severity, Span};
use kashika_geom::{flatten, PathSeg, Pt};
use kashika_model::{Line, Scene};
use kashika_syntax::ast::{self, SegKind};
use serde::Serialize;

pub use candidates::{generate, Ctrl, Hop, Lane, Plan};
pub use endpoint::{parse_ref, resolve_endpoints, Endpoint, Side, PROBE};
pub use text::{num, path_text, statement};

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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Corridor {
    /// `"y"` for a horizontal lane (the band is a y range), `"x"` for a vertical one.
    pub axis: &'static str,
    pub band: [f64; 2],
}

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub crossings: usize,
    pub crossed: Vec<Crossing>,
    pub near: usize,
    pub segments: usize,
    pub length: f64,
    pub corridor: Option<Corridor>,
    /// A ready-to-paste `line` statement.
    pub statement: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub from: Endpoint,
    pub to: Endpoint,
    pub style: Style,
    pub candidates: Vec<Candidate>,
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
}

pub fn suggest(req: &Request) -> Result<Suggestion, Error> {
    let target = req.line.map(|id| existing(req.src, id)).transpose()?;
    let (from, to) = match (req.from, req.to, &target) {
        (Some(f), Some(t), _) => resolve_endpoints(req.src, f, t)?,
        (None, None, Some(t)) => (t.from.clone(), t.to.clone()),
        _ => {
            return Err(Error::Usage(
                "give `<from> <to>`, or `--line <id>` to reroute a line".into(),
            ))
        }
    };
    let (scene, _) = kashika_model::compile(req.src);
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
    Ok(rank(req, &scene, from, to, &id, &attrs, frame))
}

/// Where a candidate is written: the origin of its frame, and the source position right
/// after which the probe goes, so its literals mean the same as in the final statement.
#[derive(Default, Clone, Copy)]
struct Frame {
    origin: Pt,
    /// 1-based line and exclusive column; `None` appends the probe at the end of the file.
    after: Option<(u32, u32)>,
}

/// An existing line to reroute: its endpoints as written, its attributes' source text, the
/// origin of the frame its literals are written in and where its statement ends.
struct Target {
    id: String,
    from: Endpoint,
    to: Endpoint,
    attrs: Vec<String>,
    origin: Pt,
    end: (u32, u32),
}

fn existing(src: &str, id: &str) -> Result<Target, Error> {
    let (file, _) = kashika_syntax::parse(src);
    let ast_line = find_line(&file.stmts, id)
        .ok_or_else(|| Error::Usage(format!("no line `{id}` in the file")))?;
    let (scene, _) = kashika_model::compile(src);
    let line = scene.lines.iter().find(|l| l.id == id).ok_or_else(|| {
        Error::Usage(format!(
            "line `{id}` doesn't resolve; fix it with `kashika check` first"
        ))
    })?;
    let origin = line
        .parent
        .as_ref()
        .and_then(|p| scene.shapes.iter().find(|s| &s.id == p))
        .map_or(Pt::default(), |g| g.rect.origin());
    let lines = source_lines(src);
    let written = |p: &ast::Point| match p {
        ast::Point::Ref { span, .. } => Some(slice(&lines, *span)),
        ast::Point::Lit { x, y, span } => {
            let refers = |c: &ast::Coord| matches!(c, ast::Coord::Axis { .. });
            (refers(x) || refers(y)).then(|| slice(&lines, *span))
        }
    };
    let end = match ast_line.segs.last().map(|s| &s.kind) {
        Some(
            SegKind::To(p)
            | SegKind::Hv(p)
            | SegKind::Vh(p)
            | SegKind::Curve { to: p, .. }
            | SegKind::CurveVia { to: p, .. },
        ) => written(p),
        _ => None,
    };
    let endpoint = |expr: Option<String>, at: Pt| {
        let (owner, side) = expr.as_deref().map_or((None, None), parse_ref);
        Endpoint {
            expr,
            at,
            owner,
            side,
        }
    };
    let (Some(first), Some(last)) = (line.path.first(), line.path.last()) else {
        return Err(Error::Usage(format!("line `{id}` has no segments")));
    };
    Ok(Target {
        id: id.to_string(),
        from: endpoint(written(&ast_line.start), first.from()),
        to: endpoint(end, last.to()),
        attrs: ast_line
            .attrs
            .iter()
            .map(|a| slice(&lines, a.span))
            .collect(),
        origin,
        end: (ast_line.span.end_line, ast_line.span.end_col),
    })
}

fn find_line<'a>(stmts: &'a [ast::Stmt], id: &str) -> Option<&'a ast::Line> {
    stmts.iter().find_map(|s| match s {
        ast::Stmt::Line(l) if l.id.as_ref().is_some_and(|i| i.name == id) => Some(l),
        ast::Stmt::Group(g) => find_line(&g.children, id),
        _ => None,
    })
}

/// The source as the lexer counts it: lines of chars, without a leading BOM.
fn source_lines(src: &str) -> Vec<Vec<char>> {
    src.strip_prefix('\u{feff}')
        .unwrap_or(src)
        .split('\n')
        .map(|l| l.chars().collect())
        .collect()
}

/// Source text of a single-line span.
fn slice(lines: &[Vec<char>], span: Span) -> String {
    let line = &lines[span.line as usize - 1];
    let end = if span.end_line == span.line {
        span.end_col as usize - 1
    } else {
        line.len()
    };
    line[span.col as usize - 1..end].iter().collect()
}

/// `src` with `stmt` on a line of its own after `frame.after`, and the line number it lands on.
fn with_probe(src: &str, frame: Frame, stmt: &str) -> (String, u32) {
    let lines = source_lines(src);
    let Some((line, col)) = frame.after else {
        return (format!("{src}\n{stmt}\n"), lines.len() as u32 + 1);
    };
    let mut out = String::new();
    for (i, l) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if i + 1 == line as usize {
            let at = (col as usize - 1).min(l.len());
            out.extend(&l[..at]);
            out.push('\n');
            out.push_str(stmt);
            out.push('\n');
            out.extend(&l[at..]);
        } else {
            out.extend(l);
        }
    }
    (out, line + 1)
}

fn rank(
    req: &Request,
    scene: &Scene,
    from: Endpoint,
    to: Endpoint,
    id: &str,
    attrs: &str,
    frame: Frame,
) -> Suggestion {
    let probe_attrs = if req.style == Style::Arc {
        format!("radius={}", num(req.radius))
    } else {
        String::new()
    };
    let ctx = score::Ctx {
        start_owner: from.owner.as_deref(),
        end_owner: to.owner.as_deref(),
        ignore: req.line,
    };
    let mut seen: Vec<Vec<(i64, i64)>> = Vec::new();
    let mut candidates = Vec::new();
    for plan in generate(scene, &from, &to, req.style) {
        let path = path_text(&from, &to, &plan, frame.origin);
        let (text, probe_line) = with_probe(req.src, frame, &statement(PROBE, &path, &probe_attrs));
        let (probed, diags) = kashika_model::compile(&text);
        if diags
            .iter()
            .any(|d| d.severity == Severity::Error && d.span.line == probe_line)
        {
            continue;
        }
        let Some(line) = probed.lines.iter().find(|l| l.id == PROBE) else {
            continue;
        };
        if !directions_ok(line, &from, &to, req.style) {
            continue;
        }
        let Some(s) = score::score(&probed, line, &ctx) else {
            continue;
        };
        let key = geometry_key(&line.path);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let corridor = plan.lane.map(|(axis, v)| {
            let span = if axis == "y" {
                (from.at.x, to.at.x)
            } else {
                (from.at.y, to.at.y)
            };
            score::corridor(&probed, axis, v, span)
        });
        candidates.push(Candidate {
            crossings: s.crossed.len(),
            crossed: s.crossed,
            near: s.near,
            segments: segments(&line.path, req.style),
            length: s.length.round(),
            corridor,
            statement: statement(id, &path, attrs),
        });
    }
    candidates.sort_by(|a, b| {
        (a.crossings, a.near, a.segments)
            .cmp(&(b.crossings, b.near, b.segments))
            .then(a.length.total_cmp(&b.length))
    });
    candidates.truncate(req.max);
    Suggestion {
        from,
        to,
        style: req.style,
        candidates,
    }
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
const SQUARE: f64 = 0.999;

/// The flattened path at 0.1px, without repeated points or points in the middle of a
/// straight run, so paths that draw the same shape get the same key.
fn geometry_key(path: &[PathSeg]) -> Vec<(i64, i64)> {
    let mut key: Vec<(i64, i64)> = Vec::new();
    for p in flatten(path) {
        let q = ((p.x * 10.0).round() as i64, (p.y * 10.0).round() as i64);
        if key.last() == Some(&q) {
            continue;
        }
        if let [.., a, b] = key[..] {
            let (u, v) = ((b.0 - a.0, b.1 - a.1), (q.0 - b.0, q.1 - b.1));
            if u.0 * v.1 == u.1 * v.0 && u.0 * v.0 + u.1 * v.1 > 0 {
                key.pop();
            }
        }
        key.push(q);
    }
    key
}

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

/// Attributes for the suggested statement: the rerouted line's own, minus `radius` (and
/// `ortho` for curves), or `end=arrow` for a new line; `arc` adds `radius=N`.
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
        out.push(format!("radius={}", num(radius)));
    }
    out.join(" ")
}
