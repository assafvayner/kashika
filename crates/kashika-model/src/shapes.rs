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
}

/// Every built-in shape, in the order docs and `kashika icons` list them.
pub const SHAPES: &[BuiltinShape] = &[
    BuiltinShape {
        name: "rect",
        keywords: &["box", "rectangle", "square"],
        source: None,
    },
    BuiltinShape {
        name: "rounded",
        keywords: &["process", "step"],
        source: None,
    },
    BuiltinShape {
        name: "cylinder",
        keywords: &["db", "database", "storage", "datastore", "sql"],
        source: Some(cylinder),
    },
    BuiltinShape {
        name: "diamond",
        keywords: &["decision", "condition"],
        source: Some(diamond),
    },
    BuiltinShape {
        name: "ellipse",
        keywords: &["circle", "oval", "state"],
        source: Some(ellipse),
    },
    BuiltinShape {
        name: "hexagon",
        keywords: &["service", "component"],
        source: Some(hexagon),
    },
    BuiltinShape {
        name: "doc",
        keywords: &["document", "file", "page"],
        source: Some(doc),
    },
    BuiltinShape {
        name: "cloud",
        keywords: &["internet", "saas", "external"],
        source: Some(cloud),
    },
    BuiltinShape {
        name: "person",
        keywords: &["user", "users", "people", "human", "account", "actor"],
        source: Some(person),
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

/// A shape in unit coordinates, stretched onto each box.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UnitShape {
    pub subpaths: Vec<Vec<PathSeg>>,
    /// 0-based index of the subpath used for anchors and checks.
    pub outline: usize,
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

/// A head of radius at most `HEAD` px over shoulders that slope from the neck to the
/// sides of the box; the text sits in the torso.
fn person(w: f64, h: f64) -> (String, usize, Rect) {
    let r = HEAD.min(h / 4.5).min(w / 4.0);
    let y = 2.2 * r;
    let k = r.min((h - y) / 2.0);
    let cx = w / 2.0;
    let p = |x: f64, y: f64| format!("{} {}", x / w, y / h);
    let d = format!(
        "M {} A {} {} 0 1 1 {} L {} Q {} {} L {} L {} L {} Q {} {} Z",
        p(cx - 0.6 * r, 1.8 * r),
        r / w,
        r / h,
        p(cx + 0.6 * r, 1.8 * r),
        p(cx + r, y),
        p(w, y),
        p(w, y + k),
        p(w, h),
        p(0.0, h),
        p(0.0, y + k),
        p(0.0, y),
        p(cx - r, y),
    );
    let inset = (cx - r) / 4.0;
    let top = y + k / 4.0;
    let area = Rect::new(inset / w, top / h, 1.0 - 2.0 * inset / w, 1.0 - top / h);
    (d, 1, area)
}

pub(crate) fn builtin(name: &str, w: f64, h: f64) -> Option<UnitShape> {
    let (d, outline, text_area) = builtin_source(name, w, h)?;
    Some(UnitShape {
        subpaths: parse_unit_path(&d).expect("built-in shapes are valid unit paths"),
        outline: outline - 1,
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
