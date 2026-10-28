use std::f64::consts::FRAC_1_SQRT_2;

use kashika_geom::{PathSeg, Rect, parse_unit_path};

/// Every built-in `shape=` name. `rect` and `rounded` use native outlines; the rest are unit paths.
pub const BUILTIN_SHAPES: &[&str] = &[
    "rect", "rounded", "cylinder", "diamond", "ellipse", "hexagon", "doc", "cloud", "person",
];

/// Largest cylinder cap height in px.
const CAP: f64 = 20.0;

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
    let unit = Rect::new;
    Some(match name {
        "cylinder" => {
            let c = cap(h) / h;
            let b = 1.0 - c;
            (
                format!(
                    "M 0 {c} A 0.5 {c} 0 0 1 1 {c} L 1 {b} A 0.5 {c} 0 0 1 0 {b} Z \
                     M 1 {c} A 0.5 {c} 0 0 1 0 {c}"
                ),
                1,
                unit(0.0, 2.0 * c, 1.0, 1.0 - 2.0 * c),
            )
        }
        "diamond" => (
            "M 0.5 0 L 1 0.5 L 0.5 1 L 0 0.5 Z".into(),
            1,
            unit(0.25, 0.25, 0.5, 0.5),
        ),
        "ellipse" => {
            let k = 0.5 - 0.5 * FRAC_1_SQRT_2;
            (
                "M 0 0.5 A 0.5 0.5 0 0 1 1 0.5 A 0.5 0.5 0 0 1 0 0.5 Z".into(),
                1,
                unit(k, k, FRAC_1_SQRT_2, FRAC_1_SQRT_2),
            )
        }
        "hexagon" => {
            let n = hex_inset(w, h) / w;
            let m = 1.0 - n;
            (
                format!("M {n} 0 L {m} 0 L 1 0.5 L {m} 1 L {n} 1 L 0 0.5 Z"),
                1,
                unit(n, 0.0, 1.0 - 2.0 * n, 1.0),
            )
        }
        "doc" => (
            "M 0 0 L 1 0 L 1 0.85 Q 0.75 0.7 0.5 0.85 Q 0.25 1 0 0.85 Z".into(),
            1,
            unit(0.0, 0.0, 1.0, 0.85),
        ),
        "cloud" => (
            "M 0.25 0.9 C 0.05 0.9 0 0.6 0.18 0.5 C 0.1 0.2 0.4 0.1 0.5 0.25 \
             C 0.6 0.05 0.95 0.15 0.85 0.45 C 1 0.55 1 0.9 0.75 0.9 Z"
                .into(),
            1,
            unit(0.2, 0.4, 0.6, 0.4),
        ),
        "person" => (
            "M 0.4 0.35 A 0.18 0.18 0 1 1 0.6 0.35 L 0.7 0.42 Q 0.9 0.42 0.9 0.6 \
             L 0.9 1 L 0.1 1 L 0.1 0.6 Q 0.1 0.42 0.3 0.42 Z"
                .into(),
            1,
            unit(0.1, 0.6, 0.8, 0.4),
        ),
        _ => return None,
    })
}

fn cap(h: f64) -> f64 {
    CAP.min(h / 4.0)
}

fn hex_inset(w: f64, h: f64) -> f64 {
    (h / 4.0).min(w / 2.0)
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

/// The smallest box size whose unit text area holds `need` (width, height), given that
/// the box is at least `rect`'s size. `custom` is the unit text area of a custom shape.
pub(crate) fn required_size(
    shape: &str,
    custom: Option<Rect>,
    need: (f64, f64),
    rect: Rect,
) -> (f64, f64) {
    let (w, h) = need;
    if let Some(unit) = custom {
        return (w / unit.w, h / unit.h);
    }
    match shape {
        "cylinder" => (
            w,
            if h >= 2.0 * CAP {
                h + 2.0 * CAP
            } else {
                2.0 * h
            },
        ),
        "hexagon" => (w + h.max(rect.h) / 2.0, h),
        _ => match unit_text_area(shape, rect.w, rect.h) {
            Some(unit) => (w / unit.w, h / unit.h),
            None => (w, h),
        },
    }
}
