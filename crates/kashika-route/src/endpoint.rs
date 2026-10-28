use kashika_diag::{Diagnostic, Severity};
use kashika_geom::Pt;
use serde::Serialize;

use crate::Error;

/// Id of the probe line that route appends to the source to resolve and score paths.
pub const PROBE: &str = "kashika_route_probe";

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
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Endpoint {
    /// Source text of a reference such as `pg.left@0.25`; `None` for literal points.
    pub expr: Option<String>,
    /// Canvas position.
    pub at: Pt,
    /// The id the reference names.
    pub owner: Option<String>,
    /// The side of `owner` the anchor is on, for `top`/`bottom`/`left`/`right` anchors.
    pub side: Option<Side>,
}

impl Endpoint {
    pub fn new(expr: &str, at: Pt) -> Self {
        let (owner, side) = parse_ref(expr);
        Endpoint {
            expr: owner.is_some().then(|| expr.trim().to_string()),
            at,
            owner,
            side,
        }
    }
}

/// Splits `db.left@0.25+(0,4)` into its id and the side of its anchor.
pub fn parse_ref(expr: &str) -> (Option<String>, Option<Side>) {
    let e = expr.trim();
    if e.is_empty() || e.starts_with('(') {
        return (None, None);
    }
    let base = e.split('+').next().unwrap_or(e);
    let (id, anchor) = match base.split_once('.') {
        Some((id, a)) => (id, Some(a)),
        None => (base, None),
    };
    let side = anchor
        .map(|a| a.split('@').next().unwrap_or(a))
        .and_then(|a| match a {
            "top" => Some(Side::Top),
            "bottom" => Some(Side::Bottom),
            "left" => Some(Side::Left),
            "right" => Some(Side::Right),
            _ => None,
        });
    (Some(id.to_string()), side)
}

/// Resolves two points, written as in a `.kash` file, by compiling them as a probe line
/// appended to `src`. Literal points are in the canvas frame.
pub fn resolve_endpoints(src: &str, from: &str, to: &str) -> Result<(Endpoint, Endpoint), Error> {
    resolve_endpoints_with(src, from, to, &kashika_model::Options::default())
}

/// Like [`resolve_endpoints`], resolving files as `opts` says.
pub fn resolve_endpoints_with(
    src: &str,
    from: &str,
    to: &str,
    opts: &kashika_model::Options,
) -> Result<(Endpoint, Endpoint), Error> {
    let first_probe_line = src.lines().count() as u32 + 1;
    let text = format!("{src}\nline {PROBE}: {from} -> {to}\n");
    let (scene, diags) = kashika_model::compile_with(&text, opts);
    let errors: Vec<Diagnostic> = diags
        .into_iter()
        .filter(|d| d.severity == Severity::Error && d.span.line >= first_probe_line)
        .collect();
    if !errors.is_empty() {
        return Err(Error::Diagnostics(errors));
    }
    let unresolved = || Error::Usage(format!("cannot resolve `{from}` -> `{to}`"));
    let line = scene
        .lines
        .iter()
        .find(|l| l.id == PROBE)
        .ok_or_else(unresolved)?;
    let (Some(a), Some(b)) = (line.path.first(), line.path.last()) else {
        return Err(unresolved());
    };
    Ok((Endpoint::new(from, a.from()), Endpoint::new(to, b.to())))
}
