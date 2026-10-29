use std::f64::consts::FRAC_1_SQRT_2;

use kashika_geom::{PathSeg, Rect, parse_unit_path};

/// Builds a unit-path shape for a `w`×`h` box: path data, outline subpath (1-based) and
/// unit text area.
pub type ShapeSource = fn(f64, f64) -> (String, usize, Rect);

/// A built-in `shape=`.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinShape {
    pub name: &'static str,
    /// Search keywords for `kashika icons --search`.
    pub keywords: &'static [&'static str],
    /// The unit path, outline subpath (1-based) and unit text area on a `w`×`h` box;
    /// `None` for `rect` and `rounded`, which use native outlines.
    pub source: Option<ShapeSource>,
    /// The outline subpath is used only for anchors and checks, and is not drawn.
    pub hidden_outline: bool,
}

/// Every built-in shape, in the order docs and `kashika icons` list them.
pub const SHAPES: &[BuiltinShape] = &[
    BuiltinShape {
        name: "rect",
        keywords: &["box", "rectangle", "square"],
        source: None,
        hidden_outline: false,
    },
    BuiltinShape {
        name: "rounded",
        keywords: &["process", "step"],
        source: None,
        hidden_outline: false,
    },
    BuiltinShape {
        name: "cylinder",
        keywords: &["db", "database", "storage", "datastore", "sql"],
        source: Some(cylinder),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "diamond",
        keywords: &["decision", "condition"],
        source: Some(diamond),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "ellipse",
        keywords: &["circle", "oval", "state"],
        source: Some(ellipse),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "hexagon",
        keywords: &["service", "component"],
        source: Some(hexagon),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "doc",
        keywords: &["document", "file", "page"],
        source: Some(doc),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "cloud",
        keywords: &["internet", "saas", "external"],
        source: Some(cloud),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "person",
        keywords: &["user", "users", "people", "human", "account", "actor"],
        source: Some(person),
        hidden_outline: true,
    },
    BuiltinShape {
        name: "parallelogram",
        keywords: &["io", "input", "output", "data"],
        source: Some(parallelogram),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "predefined-process",
        keywords: &["subroutine", "subprocess", "procedure", "function", "call"],
        source: Some(predefined_process),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "stadium",
        keywords: &["terminator", "terminal", "start", "end", "pill", "capsule"],
        source: Some(stadium),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "triangle",
        keywords: &["warning", "delta", "pyramid"],
        source: Some(triangle),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "octagon",
        keywords: &["stop", "halt", "block"],
        source: Some(octagon),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "note",
        keywords: &["comment", "annotation", "memo", "sticky"],
        source: Some(note),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "package",
        keywords: &["folder", "module", "namespace"],
        source: Some(package),
        hidden_outline: false,
    },
    BuiltinShape {
        name: "queue",
        keywords: &["fifo", "mq", "buffer", "pipe", "topic"],
        source: Some(queue),
        hidden_outline: false,
    },
];

/// The built-in shape called `name`.
pub fn builtin_shape(name: &str) -> Option<&'static BuiltinShape> {
    SHAPES.iter().find(|s| s.name == name)
}

/// Every built-in shape name, in `SHAPES` order.
pub fn shape_names() -> impl Iterator<Item = &'static str> {
    SHAPES.iter().map(|s| s.name)
}

/// Largest cylinder cap height in px.
const CAP: f64 = 20.0;
/// Largest `doc` wave depth in px.
const WAVE: f64 = 12.0;
/// Largest `cloud` bump height in px.
const BUMP: f64 = 24.0;
/// Largest `person` head radius in px.
const HEAD: f64 = 18.0;
/// Largest `parallelogram` slant in px.
const SLANT: f64 = 24.0;
/// Largest `predefined-process` bar inset in px.
const BAR: f64 = 12.0;
/// Largest `octagon` corner cut in px.
const CUT: f64 = 24.0;
/// Largest `note` fold in px.
const FOLD: f64 = 16.0;
/// Largest `package` tab height and width in px.
const TAB: (f64, f64) = (12.0, 56.0);
/// Largest `queue` cap width in px.
const SIDE_CAP: f64 = 12.0;

/// A shape in unit coordinates, stretched onto each box.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UnitShape {
    pub subpaths: Vec<Vec<PathSeg>>,
    /// 0-based index of the subpath used for anchors and checks.
    pub outline: usize,
    /// Whether the outline subpath is left out of the drawing.
    pub hidden_outline: bool,
    /// Unit rect of the text area, before padding.
    pub text_area: Rect,
}

/// The unit-path definition of a built-in shape for a `w`×`h` box: path data, outline
/// subpath (1-based) and unit text area. `None` for `rect`/`rounded` and unknown names.
pub fn builtin_source(name: &str, w: f64, h: f64) -> Option<(String, usize, Rect)> {
    builtin_shape(name)?.source.map(|f| f(w, h))
}

fn cylinder(_w: f64, h: f64) -> (String, usize, Rect) {
    let c = CAP.min(h / 4.0) / h;
    let b = 1.0 - c;
    (
        format!(
            "M 0 {c} A 0.5 {c} 0 0 1 1 {c} L 1 {b} A 0.5 {c} 0 0 1 0 {b} Z \
             M 1 {c} A 0.5 {c} 0 0 1 0 {c}"
        ),
        1,
        Rect::new(0.0, 2.0 * c, 1.0, 1.0 - 2.0 * c),
    )
}

fn diamond(_w: f64, _h: f64) -> (String, usize, Rect) {
    (
        "M 0.5 0 L 1 0.5 L 0.5 1 L 0 0.5 Z".into(),
        1,
        Rect::new(0.22, 0.28, 0.56, 0.44),
    )
}

fn ellipse(_w: f64, _h: f64) -> (String, usize, Rect) {
    let k = 0.5 - 0.5 * FRAC_1_SQRT_2;
    (
        "M 0 0.5 A 0.5 0.5 0 0 1 1 0.5 A 0.5 0.5 0 0 1 0 0.5 Z".into(),
        1,
        Rect::new(k, k, FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    )
}

fn hexagon(w: f64, h: f64) -> (String, usize, Rect) {
    let n = (h / 4.0).min(w / 4.0) / w;
    let m = 1.0 - n;
    (
        format!("M {n} 0 L {m} 0 L 1 0.5 L {m} 1 L {n} 1 L 0 0.5 Z"),
        1,
        Rect::new(n, 0.0, 1.0 - 2.0 * n, 1.0),
    )
}

fn doc(_w: f64, h: f64) -> (String, usize, Rect) {
    let d = WAVE.min(0.15 * h) / h;
    let b = 1.0 - d;
    let c = b - d;
    (
        format!("M 0 0 L 1 0 L 1 {b} Q 0.75 {c} 0.5 {b} Q 0.25 1 0 {b} Z"),
        1,
        Rect::new(0.0, 0.0, 1.0, b - d / 2.0),
    )
}

/// A flat-bottomed cloud: bumps of at most `BUMP` px along the top and as many
/// side lobes as fit, so stretching adds bumps instead of distorting them.
fn cloud(w: f64, h: f64) -> (String, usize, Rect) {
    let c = BUMP.min(0.3 * h).min(0.175 * w);
    let s = (0.15 * w).min((h - c) / 2.0).min(BUMP - 4.0);
    let mut d = format!("M {} 1", s / w);
    let mut arcs = |from: (f64, f64), to: (f64, f64), bulge: f64, n: f64| {
        let n = n.round().max(1.0);
        let len = (to.0 - from.0).hypot(to.1 - from.1) / n;
        let r = (len * len / 4.0 + bulge * bulge) / (2.0 * bulge);
        let large = u8::from(bulge > len / 2.0);
        for i in 1..=n as usize {
            let t = i as f64 / n;
            let x = from.0 + (to.0 - from.0) * t;
            let y = from.1 + (to.1 - from.1) * t;
            d.push_str(&format!(
                " A {} {} 0 {large} 1 {} {}",
                r / w,
                r / h,
                x / w,
                y / h
            ));
        }
    };
    let side = h - c;
    arcs((s, h), (s, c), s, side / (2.5 * s));
    arcs((s, c), (w - s, c), c, ((w - 2.0 * s) / (2.5 * c)).max(2.0));
    arcs((w - s, c), (w - s, h), s, side / (2.5 * s));
    d.push_str(" Z");
    let area = Rect::new(s / w, c / h, 1.0 - 2.0 * s / w, (h - 1.5 * c) / h);
    (d, 1, area)
}

/// A round head of radius at most `HEAD` px over a body with rounded shoulders that
/// spans the box; the text sits in the body. The third subpath, not drawn, is the
/// silhouette of both joined by a neck, for anchors and checks.
fn person(w: f64, h: f64) -> (String, usize, Rect) {
    let r = HEAD.min(0.2 * h).min(0.22 * w);
    let top = (2.0 * r + (0.25 * r).max(3.0)).min(h);
    let k = (0.6 * (h - top)).min(w / 2.0).min(1.6 * r);
    let cx = w / 2.0;
    let n = 0.6 * r;
    let p = |x: f64, y: f64| format!("{} {}", x / w, y / h);
    let head = |rx: f64, ry: f64| format!("{} {}", rx / w, ry / h);
    let shoulders = |neck: &str| {
        format!(
            "M {} L {} A {} 0 0 1 {} {neck}L {} A {} 0 0 1 {} L {} Z",
            p(0.0, h),
            p(0.0, top + k),
            head(k, k),
            p(k, top),
            p(w - k, top),
            head(k, k),
            p(w, top + k),
            p(w, h),
        )
    };
    let d = format!(
        "M {} A {} 0 1 1 {} A {} 0 1 1 {} Z {} {}",
        p(cx + r, r),
        head(r, r),
        p(cx - r, r),
        head(r, r),
        p(cx + r, r),
        shoulders(""),
        shoulders(&format!(
            "L {} L {} A {} 0 1 1 {} L {} ",
            p(cx - n, top),
            p(cx - n, 1.8 * r),
            head(r, r),
            p(cx + n, 1.8 * r),
            p(cx + n, top),
        )),
    );
    let i = 0.3 * k;
    let area = Rect::new(i / w, (top + i) / h, 1.0 - 2.0 * i / w, (h - top - i) / h);
    (d, 3, area)
}

/// Formats a px point on a `w`×`h` box as a unit-path point.
fn px(w: f64, h: f64) -> impl Fn(f64, f64) -> String {
    move |x, y| format!("{} {}", x / w, y / h)
}

/// A px rect on a `w`×`h` box as a unit rect.
fn unit_rect(w: f64, h: f64, r: Rect) -> Rect {
    Rect::new(r.x / w, r.y / h, r.w / w, r.h / h)
}

fn parallelogram(w: f64, h: f64) -> (String, usize, Rect) {
    let p = px(w, h);
    let s = SLANT.min(h / 2.0).min(w / 4.0);
    let d = format!(
        "M {} L {} L {} L {} Z",
        p(s, 0.0),
        p(w, 0.0),
        p(w - s, h),
        p(0.0, h)
    );
    (d, 1, unit_rect(w, h, Rect::new(s, 0.0, w - 2.0 * s, h)))
}

fn predefined_process(w: f64, _h: f64) -> (String, usize, Rect) {
    let i = BAR.min(w / 8.0) / w;
    let m = 1.0 - i;
    (
        format!("M 0 0 L 1 0 L 1 1 L 0 1 Z M {i} 0 L {i} 1 M {m} 0 L {m} 1"),
        1,
        Rect::new(i, 0.0, 1.0 - 2.0 * i, 1.0),
    )
}

/// A rect whose corner radius `r` is half its shorter side. The text area's corners lie on
/// the corner circles, `0.1 × r` in from the top and bottom.
fn stadium(w: f64, h: f64) -> (String, usize, Rect) {
    let p = px(w, h);
    let r = w.min(h) / 2.0;
    let arc = |x: f64, y: f64| format!(" A {} {} 0 0 1 {}", r / w, r / h, p(x, y));
    let mut d = format!("M {}", p(r, 0.0));
    let wide = w - r > r;
    let tall = h - r > r;
    if wide {
        d += &format!(" L {}", p(w - r, 0.0));
    }
    d += &arc(w, r);
    if tall {
        d += &format!(" L {}", p(w, h - r));
    }
    d += &arc(w - r, h);
    if wide {
        d += &format!(" L {}", p(r, h));
    }
    d += &arc(0.0, h - r);
    if tall {
        d += &format!(" L {}", p(0.0, r));
    }
    d += &arc(r, 0.0);
    d += " Z";
    let (a, b) = (0.57 * r, 0.1 * r);
    (
        d,
        1,
        unit_rect(w, h, Rect::new(a, b, w - 2.0 * a, h - 2.0 * b)),
    )
}

fn triangle(_w: f64, _h: f64) -> (String, usize, Rect) {
    (
        "M 0.5 0 L 1 1 L 0 1 Z".into(),
        1,
        Rect::new(0.25, 0.5, 0.5, 0.5),
    )
}

fn octagon(w: f64, h: f64) -> (String, usize, Rect) {
    let p = px(w, h);
    let c = CUT.min(0.29 * w.min(h));
    let d = format!(
        "M {} L {} L {} L {} L {} L {} L {} L {} Z",
        p(c, 0.0),
        p(w - c, 0.0),
        p(w, c),
        p(w, h - c),
        p(w - c, h),
        p(c, h),
        p(0.0, h - c),
        p(0.0, c)
    );
    (
        d,
        1,
        unit_rect(w, h, Rect::new(c / 2.0, c / 2.0, w - c, h - c)),
    )
}

/// A sheet with its top-right corner folded down; the fold is the second subpath.
fn note(w: f64, h: f64) -> (String, usize, Rect) {
    let p = px(w, h);
    let f = FOLD.min(w / 4.0).min(h / 4.0);
    let d = format!(
        "M {} L {} L {} L {} L {} Z M {} L {} L {}",
        p(0.0, 0.0),
        p(w - f, 0.0),
        p(w, f),
        p(w, h),
        p(0.0, h),
        p(w - f, 0.0),
        p(w - f, f),
        p(w, f)
    );
    (
        d,
        1,
        unit_rect(w, h, Rect::new(0.0, f / 2.0, w - f / 2.0, h - f / 2.0)),
    )
}

/// A folder: a tab on the top left over the body, which holds the text.
fn package(w: f64, h: f64) -> (String, usize, Rect) {
    let p = px(w, h);
    let t = TAB.0.min(h / 5.0);
    let tw = TAB.1.min(0.4 * w);
    let d = format!(
        "M {} L {} L {} L {} L {} L {} Z M {} L {}",
        p(0.0, 0.0),
        p(tw, 0.0),
        p(tw, t),
        p(w, t),
        p(w, h),
        p(0.0, h),
        p(0.0, t),
        p(tw, t)
    );
    (d, 1, unit_rect(w, h, Rect::new(0.0, t, w, h - t)))
}

/// A cylinder on its side: caps `min(SIDE_CAP, h/5, w/4)` px wide, the right one drawn whole.
fn queue(w: f64, h: f64) -> (String, usize, Rect) {
    let p = px(w, h);
    let c = SIDE_CAP.min(h / 5.0).min(w / 4.0);
    let rad = format!("{} 0.5", c / w);
    let d = format!(
        "M {} L {} A {rad} 0 0 1 {} L {} A {rad} 0 0 1 {} Z M {} A {rad} 0 0 0 {}",
        p(c, 0.0),
        p(w - c, 0.0),
        p(w - c, h),
        p(c, h),
        p(c, 0.0),
        p(w - c, 0.0),
        p(w - c, h)
    );
    (d, 1, unit_rect(w, h, Rect::new(c, 0.0, w - 3.0 * c, h)))
}

pub(crate) fn builtin(name: &str, w: f64, h: f64) -> Option<UnitShape> {
    let shape = builtin_shape(name)?;
    let (d, outline, text_area) = shape.source?(w, h);
    Some(UnitShape {
        subpaths: parse_unit_path(&d).expect("built-in shapes are valid unit paths"),
        outline: outline - 1,
        hidden_outline: shape.hidden_outline,
        text_area,
    })
}

/// The unit text area of `shape` on a `w`×`h` box; `None` for `rect`/`rounded`.
pub(crate) fn unit_text_area(shape: &str, w: f64, h: f64) -> Option<Rect> {
    builtin_source(shape, w, h).map(|(_, _, unit)| unit)
}

/// The smallest box size in whole pixels, at least `min`, whose unit text area holds
/// `need` (width, height). `custom` is the unit text area of a custom shape. Each axis is found by
/// bisection on the shape's own text area, so no shape needs a hand-written inverse.
pub(crate) fn required_size(
    shape: &str,
    custom: Option<Rect>,
    need: (f64, f64),
    min: (f64, f64),
) -> (f64, f64) {
    let area = |w: f64, h: f64| {
        let u = custom
            .or_else(|| unit_text_area(shape, w, h))
            .unwrap_or(Rect::new(0.0, 0.0, 1.0, 1.0));
        (u.w * w, u.h * h)
    };
    let (mut w, mut h) = (min.0.max(need.0), min.1.max(need.1));
    for _ in 0..64 {
        let h2 = least(h, |h| area(w, h).1 >= need.1 - 1e-9);
        let w2 = least(w, |w| area(w, h2).0 >= need.0 - 1e-9);
        let done = (w2, h2) == (w, h);
        (w, h) = (w2, h2);
        if done {
            break;
        }
    }
    (w, h)
}

/// The least whole number `x >= lo` for which `ok` holds, assuming `ok` is monotonic.
fn least(lo: f64, ok: impl Fn(f64) -> bool) -> f64 {
    let mut a = lo.ceil();
    if ok(a) {
        return a;
    }
    let mut b = a.max(1.0) * 2.0;
    while !ok(b) {
        if b > 1e9 {
            return b;
        }
        a = b;
        b *= 2.0;
    }
    while b - a > 1.0 {
        let m = ((a + b) / 2.0).floor();
        if ok(m) {
            b = m;
        } else {
            a = m;
        }
    }
    b
}
