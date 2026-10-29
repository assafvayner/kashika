use std::collections::BTreeMap;

use kashika_check::ctx::{Ctx, bbox, kind_name, text_id};
use kashika_check::free::corridors as free_corridors;
use kashika_check::ports::ports as find_ports;
use kashika_diag::{Diagnostic, Severity};
use kashika_geom::{Pt, Rect, length, point_at, round2};
use kashika_model::{EdgeKind, Fit, LabelSide, Scene, SegmentKind};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::report::plural;

pub const SCHEMA: &str = "kashika.inspect/1";
const ANCHORS: [&str; 9] = [
    "top", "bottom", "left", "right", "center", "tl", "tr", "bl", "br",
];

type P = [f64; 2];

fn p(q: Pt) -> P {
    [round2(q.x), round2(q.y)]
}

pub struct Request<'r> {
    pub ids: &'r [String],
    pub ports: bool,
    pub corridors: Option<Rect>,
}

#[derive(Serialize)]
pub struct Inspect {
    schema: &'static str,
    tool: &'static str,
    input: Input,
    counts: Counts,
    /// The canvas size, after any `auto` axis is fitted.
    canvas: P,
    /// Whether the text output lists everything, so it starts with the canvas.
    #[serde(skip)]
    all: bool,
    shapes: Vec<ShapeInfo>,
    junctions: Vec<JunctionInfo>,
    texts: Vec<TextInfo>,
    lines: Vec<LineInfo>,
    placement: Vec<Edge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ports: Option<Vec<PortInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    corridors: Option<Corridors>,
}

#[derive(Serialize)]
struct Input {
    name: String,
    sha256: String,
}

#[derive(Serialize)]
struct Counts {
    error: usize,
    warn: usize,
}

#[derive(Serialize)]
struct ShapeInfo {
    id: String,
    kind: &'static str,
    label: Option<String>,
    parent: Option<String>,
    line: u32,
    bbox: [P; 2],
    local_bbox: [P; 2],
    anchors: BTreeMap<&'static str, P>,
    local_anchors: BTreeMap<&'static str, P>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<ImageInfo>,
}

/// The file an image or `shape from` SVG draws.
#[derive(Serialize)]
struct ImageInfo {
    path: String,
    natural: P,
    aspect: f64,
    fit: Fit,
}

#[derive(Serialize)]
struct JunctionInfo {
    id: String,
    parent: Option<String>,
    line: u32,
    at: P,
    local_at: P,
}

#[derive(Serialize)]
struct TextInfo {
    id: String,
    content: String,
    parent: Option<String>,
    line: u32,
    at: P,
    local_at: P,
    bbox: [P; 2],
}

#[derive(Serialize)]
struct LineInfo {
    id: String,
    parent: Option<String>,
    line: u32,
    length: f64,
    vertices: Vec<P>,
    local_vertices: Vec<P>,
    segments: Vec<SegInfo>,
    labels: Vec<LabelInfo>,
}

#[derive(Serialize)]
struct SegInfo {
    index: usize,
    kind: SegmentKind,
    line: u32,
    col: u32,
    from: P,
    to: P,
}

#[derive(Serialize)]
struct LabelInfo {
    t: f64,
    text: String,
    at: P,
    #[serde(skip_serializing_if = "LabelSide::is_on")]
    side: LabelSide,
    bbox: [P; 2],
}

#[derive(Serialize)]
struct Edge {
    from: String,
    to: String,
    kind: EdgeKind,
}

#[derive(Serialize)]
struct PortInfo {
    shape: String,
    side: &'static str,
    t: f64,
    at: P,
    line: String,
    end: &'static str,
}

#[derive(Serialize)]
struct Corridors {
    rect: [P; 2],
    horizontal: Vec<P>,
    vertical: Vec<P>,
}

/// `lints` counts the unsuppressed warnings `check` found besides those in `diags`.
pub fn build(
    name: &str,
    src: &str,
    scene: &Scene,
    diags: &[Diagnostic],
    lints: usize,
    req: &Request,
) -> Inspect {
    let corridors_only = req.ids.is_empty() && req.corridors.is_some();
    let keep = |id: &str| {
        if req.ids.is_empty() {
            !corridors_only
        } else {
            req.ids.iter().any(|i| i == id)
        }
    };
    let errors = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let shapes = scene
        .shapes
        .iter()
        .filter(|s| keep(&s.id))
        .map(|s| {
            let pts: Vec<(&'static str, Pt)> = ANCHORS
                .iter()
                .filter_map(|a| s.anchor(a, None).map(|q| (*a, q)))
                .collect();
            ShapeInfo {
                id: s.id.clone(),
                kind: kind_name(s),
                label: s.label.clone(),
                parent: s.parent.clone(),
                line: s.span.line,
                bbox: bbox(s.rect),
                local_bbox: bbox(s.local_rect()),
                anchors: pts.iter().map(|&(a, q)| (a, p(q))).collect(),
                local_anchors: pts.iter().map(|&(a, q)| (a, p(q - s.origin))).collect(),
                image: s.image.as_ref().and_then(|img| {
                    let a = scene.assets.get(img.asset)?;
                    let (w, h) = a.natural;
                    Some(ImageInfo {
                        path: a.path.clone(),
                        natural: [round2(w), round2(h)],
                        aspect: if h > 0.0 { round2(w / h) } else { 0.0 },
                        fit: img.fit,
                    })
                }),
            }
        })
        .collect();
    let junctions = scene
        .junctions
        .iter()
        .filter(|j| keep(&j.id))
        .map(|j| JunctionInfo {
            id: j.id.clone(),
            parent: j.parent.clone(),
            line: j.span.line,
            at: p(j.at),
            local_at: p(j.at - j.origin),
        })
        .collect();
    let texts = scene
        .texts
        .iter()
        .filter(|t| keep(&text_id(t)))
        .map(|t| TextInfo {
            id: text_id(t),
            content: t.content.clone(),
            parent: t.parent.clone(),
            line: t.span.line,
            at: p(t.at),
            local_at: p(t.at - t.origin),
            bbox: bbox(t.rect),
        })
        .collect();
    let lines = scene
        .lines
        .iter()
        .filter(|l| keep(&l.id))
        .map(|l| {
            let mut vertices: Vec<Pt> = l.path.first().map(|s| s.from()).into_iter().collect();
            vertices.extend(l.path.iter().map(|s| s.to()));
            LineInfo {
                id: l.id.clone(),
                parent: l.parent.clone(),
                line: l.span.line,
                length: round2(length(&l.path)),
                local_vertices: vertices.iter().map(|v| p(*v - l.origin)).collect(),
                vertices: vertices.iter().map(|v| p(*v)).collect(),
                segments: l
                    .segments
                    .iter()
                    .enumerate()
                    .map(|(i, g)| SegInfo {
                        index: i + 1,
                        kind: g.kind,
                        line: g.span.line,
                        col: g.span.col,
                        from: p(g.from),
                        to: p(g.to),
                    })
                    .collect(),
                labels: l
                    .labels
                    .iter()
                    .map(|lab| LabelInfo {
                        t: lab.t,
                        text: lab.text.clone(),
                        at: p(point_at(&l.path, lab.t)),
                        side: lab.side,
                        bbox: bbox(lab.rect),
                    })
                    .collect(),
            }
        })
        .collect();
    let placement = scene
        .placement
        .iter()
        .filter(|e| keep(&e.from) || keep(&e.to))
        .map(|e| Edge {
            from: e.from.clone(),
            to: e.to.clone(),
            kind: e.kind,
        })
        .collect();
    let ports = req.ports.then(|| {
        let ctx = Ctx::new(scene);
        find_ports(&ctx)
            .into_iter()
            .filter(|x| req.ids.is_empty() || keep(&x.shape) || keep(&x.line))
            .map(|x| PortInfo {
                shape: x.shape,
                side: x.side,
                t: round2(x.t),
                at: p(x.at),
                line: x.line,
                end: x.end,
            })
            .collect()
    });
    let corridors = req.corridors.map(|r| {
        let (h, v) = free_corridors(scene, r);
        Corridors {
            rect: bbox(r),
            horizontal: h.into_iter().map(|(a, b)| [a, b]).collect(),
            vertical: v.into_iter().map(|(a, b)| [a, b]).collect(),
        }
    });
    Inspect {
        schema: SCHEMA,
        tool: env!("CARGO_PKG_VERSION"),
        input: Input {
            name: name.to_string(),
            sha256: format!("{:x}", Sha256::digest(src.as_bytes())),
        },
        counts: Counts {
            error: errors,
            warn: diags.len() - errors + lints,
        },
        canvas: [round2(scene.size.0), round2(scene.size.1)],
        all: req.ids.is_empty() && !corridors_only,
        shapes,
        junctions,
        texts,
        lines,
        placement,
        ports,
        corridors,
    }
}

fn fit_name(f: Fit) -> &'static str {
    match f {
        Fit::Contain => "contain",
        Fit::Cover => "cover",
        Fit::Stretch => "stretch",
    }
}

fn pt(q: &P) -> String {
    format!("({},{})", q[0], q[1])
}

fn pts(qs: &[P]) -> String {
    qs.iter().map(pt).collect::<Vec<_>>().join(" ")
}

fn rect(b: &[P; 2]) -> String {
    format!("{}-{}", pt(&b[0]), pt(&b[1]))
}

fn anchors(m: &BTreeMap<&'static str, P>) -> String {
    ANCHORS
        .iter()
        .filter_map(|a| m.get(a).map(|q| format!("{a} {}", pt(q))))
        .collect::<Vec<_>>()
        .join(" ")
}

fn intervals(v: &[P]) -> String {
    if v.is_empty() {
        return "none".to_string();
    }
    v.iter()
        .map(|i| format!("{}..{}", i[0], i[1]))
        .collect::<Vec<_>>()
        .join(", ")
}

fn within(parent: &Option<String>) -> String {
    parent
        .as_ref()
        .map(|p| format!(" in {p}"))
        .unwrap_or_default()
}

pub fn text(i: &Inspect) -> String {
    let mut out = String::new();
    if i.all {
        out.push_str(&format!("canvas {}x{}\n", i.canvas[0], i.canvas[1]));
    }
    for s in &i.shapes {
        let label = s
            .label
            .as_ref()
            .map(|l| format!(" {l:?}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "{} {}{label} (line {}){}\n",
            s.kind,
            s.id,
            s.line,
            within(&s.parent)
        ));
        out.push_str(&format!("  bbox {}\n", rect(&s.bbox)));
        out.push_str(&format!("  anchors {}\n", anchors(&s.anchors)));
        if let Some(img) = &s.image {
            out.push_str(&format!(
                "  image {:?} natural {}x{} aspect {} fit={}\n",
                img.path,
                img.natural[0],
                img.natural[1],
                img.aspect,
                fit_name(img.fit)
            ));
        }
        if s.parent.is_some() {
            out.push_str(&format!("  local bbox {}\n", rect(&s.local_bbox)));
            out.push_str(&format!("  local anchors {}\n", anchors(&s.local_anchors)));
        }
    }
    for j in &i.junctions {
        out.push_str(&format!(
            "junction {} at {} (line {}){}\n",
            j.id,
            pt(&j.at),
            j.line,
            within(&j.parent)
        ));
        if j.parent.is_some() {
            out.push_str(&format!("  local {}\n", pt(&j.local_at)));
        }
    }
    for t in &i.texts {
        out.push_str(&format!(
            "text {} {:?} at {} bbox {} (line {}){}\n",
            t.id,
            t.content,
            pt(&t.at),
            rect(&t.bbox),
            t.line,
            within(&t.parent)
        ));
        if t.parent.is_some() {
            out.push_str(&format!("  local {}\n", pt(&t.local_at)));
        }
    }
    for l in &i.lines {
        out.push_str(&format!(
            "line {} (line {}){} length {}px\n",
            l.id,
            l.line,
            within(&l.parent),
            l.length
        ));
        out.push_str(&format!("  vertices {}\n", pts(&l.vertices)));
        if l.parent.is_some() {
            out.push_str(&format!("  local vertices {}\n", pts(&l.local_vertices)));
        }
        for s in &l.segments {
            out.push_str(&format!(
                "  segment {} {} ({}:{}) {} -> {}\n",
                s.index,
                s.kind.name(),
                s.line,
                s.col,
                pt(&s.from),
                pt(&s.to)
            ));
        }
        for lab in &l.labels {
            let side = if lab.side.is_on() {
                String::new()
            } else {
                format!(" {}", lab.side.name())
            };
            out.push_str(&format!(
                "  label@{} {:?} at {}{side} bbox {}\n",
                lab.t,
                lab.text,
                pt(&lab.at),
                rect(&lab.bbox)
            ));
        }
    }
    if !i.placement.is_empty() {
        out.push_str("placement\n");
        for e in &i.placement {
            out.push_str(&format!("  {} -> {} ({})\n", e.from, e.to, e.kind.name()));
        }
    }
    if let Some(ports) = &i.ports {
        out.push_str("ports\n");
        for x in ports {
            let anchor = if x.t == 0.5 {
                format!("{}.{}", x.shape, x.side)
            } else {
                format!("{}.{}@{}", x.shape, x.side, x.t)
            };
            out.push_str(&format!("  {anchor} {}: {} {}\n", pt(&x.at), x.line, x.end));
        }
    }
    if let Some(c) = &i.corridors {
        out.push_str(&format!("corridors {}\n", rect(&c.rect)));
        out.push_str(&format!(
            "  horizontal, free y: {}\n",
            intervals(&c.horizontal)
        ));
        out.push_str(&format!("  vertical, free x: {}\n", intervals(&c.vertical)));
    }
    out.push_str(&format!(
        "{}, {}\n",
        plural(i.counts.error, "error"),
        plural(i.counts.warn, "warning")
    ));
    out
}
