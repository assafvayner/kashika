use kashika_diag::{Diagnostic, Severity};
use kashika_geom::Pt;
use kashika_syntax::ast::{self, SegKind};
use serde::Serialize;

use crate::Error;
use crate::source::Source;

/// Id prefix of the probe lines that route adds to the source to resolve and score paths.
pub(crate) const PROBE: &str = "kashika_route_probe";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    /// Unit vector pointing out of the shape.
    pub fn outward(self) -> Pt {
        match self {
            Side::Top => Pt::new(0.0, -1.0),
            Side::Bottom => Pt::new(0.0, 1.0),
            Side::Left => Pt::new(-1.0, 0.0),
            Side::Right => Pt::new(1.0, 0.0),
        }
    }

    fn of(anchor: &str) -> Option<Side> {
        match anchor {
            "top" => Some(Side::Top),
            "bottom" => Some(Side::Bottom),
            "left" => Some(Side::Left),
            "right" => Some(Side::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Endpoint {
    /// Source text of a reference such as `pg.left@0.25` or a mixed point such as
    /// `(db.left.x, 120)`; `None` for literal points.
    pub expr: Option<String>,
    /// Canvas position.
    pub at: Pt,
    /// The id the reference names.
    pub owner: Option<String>,
    /// The side of `owner` the anchor is on, for `top`/`bottom`/`left`/`right` anchors.
    pub side: Option<Side>,
    /// A reference split into its text without the offset, and the offset.
    #[serde(skip)]
    pub(crate) anchor: Option<(String, Pt)>,
}

impl Endpoint {
    /// An endpoint written as a literal, or that no point names.
    pub(crate) fn literal(at: Pt) -> Self {
        Endpoint {
            expr: None,
            at,
            owner: None,
            side: None,
            anchor: None,
        }
    }

    /// The endpoint a point of a `line` statement resolves to: `text` is how it is written and
    /// `at` where it lands.
    pub(crate) fn of(p: &ast::Point, text: String, at: Pt) -> Self {
        Endpoint::shifted(p, text, at, Pt::new(0.0, 0.0))
    }

    /// As [`Endpoint::of`], for a point written with offsets that add up to `shift` after it.
    fn shifted(p: &ast::Point, text: String, at: Pt, shift: Pt) -> Self {
        match p {
            ast::Point::Ref { r, offset, .. } => {
                let (dx, dy) = offset.map_or((0.0, 0.0), |o| o.delta());
                Endpoint {
                    expr: Some(text),
                    at,
                    owner: Some(r.id.name.clone()),
                    side: r.anchor.as_ref().and_then(|a| Side::of(&a.name.name)),
                    anchor: Some((ref_text(r), shift + Pt::new(dx, dy))),
                }
            }
            ast::Point::Lit { x, y, .. } => {
                let mixed = [x, y].iter().any(|c| matches!(c, ast::Coord::Axis { .. }));
                Endpoint {
                    expr: mixed.then_some(text),
                    ..Endpoint::literal(at)
                }
            }
            ast::Point::Group { inner, .. } => Endpoint::shifted(inner, text, at, shift),
            ast::Point::Shifted { base, offsets, .. } => {
                let shift = offsets.iter().fold(shift, |s, o| {
                    let (dx, dy) = o.delta();
                    s + Pt::new(dx, dy)
                });
                Endpoint::shifted(base, text, at, shift)
            }
        }
    }
}

/// A reference as written without its offset: `db`, `db.left` or `db.left@0.25`.
fn ref_text(r: &ast::Ref) -> String {
    let mut s = r.id.name.clone();
    if let Some(a) = &r.anchor {
        s.push('.');
        s.push_str(&a.name.name);
        if let Some(t) = a.t {
            s.push_str(&format!("@{t}"));
        }
    }
    s
}

/// The point a line's last segment ends on, when that segment names one.
pub(crate) fn end_point(line: &ast::Line) -> Option<&ast::Point> {
    match &line.segs.last()?.kind {
        SegKind::To(p)
        | SegKind::Hv(p)
        | SegKind::Vh(p)
        | SegKind::Vhv { to: p, .. }
        | SegKind::Hvh { to: p, .. }
        | SegKind::Curve { to: p, .. }
        | SegKind::CurveVia { to: p, .. } => Some(p),
        SegKind::H(_) | SegKind::V(_) | SegKind::X(_) | SegKind::Y(_) => None,
    }
}

/// Resolves two points, written as in a `.kash` file, the way a `line <from> -> <to>`
/// statement appended to `src` would. Literal points are in the canvas frame.
pub(crate) fn resolve_endpoints(
    src: &str,
    from: &str,
    to: &str,
    opts: &kashika_model::Options,
) -> Result<(Endpoint, Endpoint), Error> {
    let stmt = format!("line {PROBE} {from} -> {to}");
    let first_probe_line = src.lines().count() as u32 + 1;
    let (scene, diags) = kashika_model::compile_with(&format!("{src}\n{stmt}\n"), opts);
    let errors: Vec<Diagnostic> = diags
        .into_iter()
        .filter(|d| {
            d.severity == Severity::Error
                && d.span.line >= first_probe_line
                && d.code != "not-ortho"
        })
        .collect();
    if !errors.is_empty() {
        return Err(Error::Diagnostics(errors));
    }
    let unresolved = || Error::Usage(format!("cannot resolve `{from}` -> `{to}`"));
    let (file, _) = kashika_syntax::parse(&stmt);
    let points = match file.stmts.as_slice() {
        [ast::Stmt::Line(l)] => match l.segs.as_slice() {
            [
                ast::Seg {
                    kind: SegKind::To(end),
                    ..
                },
            ] => Some((&l.start, end)),
            _ => None,
        },
        _ => None,
    };
    let Some((start, end)) = points else {
        return Err(Error::Usage(format!(
            "`<from>` and `<to>` must each be one point, not `{from}` -> `{to}`"
        )));
    };
    let line = scene
        .lines
        .iter()
        .find(|l| l.id == PROBE)
        .ok_or_else(unresolved)?;
    let (Some(a), Some(b)) = (line.path.first(), line.path.last()) else {
        return Err(unresolved());
    };
    let text = Source::new(&stmt);
    Ok((
        Endpoint::of(start, text.slice(start.span()), a.from()),
        Endpoint::of(end, text.slice(end.span()), b.to()),
    ))
}
