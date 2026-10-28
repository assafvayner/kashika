use kashika_diag::did_you_mean;
use kashika_geom::{Pt, Rect};

pub const ANCHORS: &[&str] = &[
    "top",
    "bottom",
    "left",
    "right",
    "center",
    "tl",
    "tr",
    "bl",
    "br",
    "top-left",
    "top-right",
    "bottom-left",
    "bottom-right",
];

fn canonical(name: &str) -> &str {
    match name {
        "top-left" => "tl",
        "top-right" => "tr",
        "bottom-left" => "bl",
        "bottom-right" => "br",
        other => other,
    }
}

/// Resolves an anchor on a (possibly rounded) rectangle. Edge anchors are axis-projected
/// onto the rounded outline; corner anchors stay on the bounding box.
/// Errors carry a message and an optional help line; `owner` is the id the anchor is on.
pub fn resolve(
    rect: Rect,
    radius: f64,
    owner: &str,
    name: &str,
    t: Option<f64>,
) -> Result<Pt, (String, Option<String>)> {
    let name_c = canonical(name);
    let is_side = matches!(name_c, "top" | "bottom" | "left" | "right");
    if t.is_some() && !is_side && ANCHORS.contains(&name) {
        return Err((
            "`@t` only applies to `top`/`bottom`/`left`/`right`".into(),
            Some(format!(
                "use `{owner}.{name}` without `@t`, or an edge such as `{owner}.top@0.25`"
            )),
        ));
    }
    let t = t.unwrap_or(0.5);
    if is_side && !(0.0..=1.0).contains(&t) {
        return Err((
            "`t` must be between 0 and 1".into(),
            Some(format!(
                "`{owner}.{name}@0` is the start of the edge and `{owner}.{name}@1` the end"
            )),
        ));
    }
    let Rect { x, y, w, h } = rect;
    let p = match name_c {
        "top" => Pt::new(x + w * t, y + inset(radius, w * t, w)),
        "bottom" => Pt::new(x + w * t, y + h - inset(radius, w * t, w)),
        "left" => Pt::new(x + inset(radius, h * t, h), y + h * t),
        "right" => Pt::new(x + w - inset(radius, h * t, h), y + h * t),
        "center" => rect.center(),
        "tl" => Pt::new(x, y),
        "tr" => Pt::new(x + w, y),
        "bl" => Pt::new(x, y + h),
        "br" => Pt::new(x + w, y + h),
        _ => {
            let help =
                did_you_mean(name, ANCHORS.iter().copied()).map(|s| format!("did you mean `{s}`?"));
            return Err((format!("unknown anchor `{name}` on `{owner}`"), help));
        }
    };
    Ok(p)
}

/// How far a rounded outline sits inside its bounding-box edge at `along` px from the edge start.
fn inset(r: f64, along: f64, len: f64) -> f64 {
    let d = if along < r {
        r - along
    } else if along > len - r {
        along - (len - r)
    } else {
        return 0.0;
    };
    r - (r * r - d * d).max(0.0).sqrt()
}

/// Offset of the `pin` anchor from a box's top-left corner.
pub fn pin_offset(name: &str, w: f64, h: f64) -> Pt {
    match canonical(name) {
        "top" => Pt::new(w / 2.0, 0.0),
        "bottom" => Pt::new(w / 2.0, h),
        "left" => Pt::new(0.0, h / 2.0),
        "right" => Pt::new(w, h / 2.0),
        "center" => Pt::new(w / 2.0, h / 2.0),
        "tr" => Pt::new(w, 0.0),
        "bl" => Pt::new(0.0, h),
        "br" => Pt::new(w, h),
        _ => Pt::new(0.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: Rect = Rect::new(0.0, 0.0, 100.0, 60.0);

    #[test]
    fn rect_anchors() {
        assert_eq!(resolve(R, 0.0, "r", "top", None), Ok(Pt::new(50.0, 0.0)));
        assert_eq!(
            resolve(R, 0.0, "r", "right", Some(0.25)),
            Ok(Pt::new(100.0, 15.0))
        );
        assert_eq!(
            resolve(R, 0.0, "r", "bottom-right", None),
            Ok(Pt::new(100.0, 60.0))
        );
    }

    #[test]
    fn rounded_anchors_are_axis_projected() {
        assert_eq!(
            resolve(R, 8.0, "r", "top", Some(0.0)),
            Ok(Pt::new(0.0, 8.0))
        );
        assert_eq!(resolve(R, 8.0, "r", "top", None), Ok(Pt::new(50.0, 0.0)));
        assert_eq!(resolve(R, 8.0, "r", "tl", None), Ok(Pt::new(0.0, 0.0)));
    }

    #[test]
    fn anchor_errors() {
        assert_eq!(
            resolve(R, 0.0, "r", "rigth", None)
                .unwrap_err()
                .1
                .as_deref(),
            Some("did you mean `right`?")
        );
        assert!(resolve(R, 0.0, "r", "center", Some(0.5)).is_err());
        assert!(resolve(R, 0.0, "r", "top", Some(1.5)).is_err());
    }

    #[test]
    fn pin_offsets() {
        assert_eq!(pin_offset("left", 120.0, 60.0), Pt::new(0.0, 30.0));
        assert_eq!(
            pin_offset("bottom-right", 120.0, 60.0),
            Pt::new(120.0, 60.0)
        );
    }
}
