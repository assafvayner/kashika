use std::f64::consts::{FRAC_PI_2, TAU};

use crate::path::PathSeg;
use crate::point::Pt;

enum PTok {
    Cmd(char),
    Num(f64),
}

fn tokens(d: &str) -> Result<Vec<PTok>, String> {
    let cs: Vec<char> = d.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() || c == ',' {
            i += 1;
            continue;
        }
        if c.is_ascii_alphabetic() {
            out.push(PTok::Cmd(c));
            i += 1;
            continue;
        }
        let start = i;
        if c == '-' || c == '+' {
            i += 1;
        }
        while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') {
            i += 1;
        }
        if i > start && matches!(cs.get(i), Some('e' | 'E')) {
            let mut j = i + 1;
            if matches!(cs.get(j), Some('-' | '+')) {
                j += 1;
            }
            if cs.get(j).is_some_and(char::is_ascii_digit) {
                while cs.get(j).is_some_and(char::is_ascii_digit) {
                    j += 1;
                }
                i = j;
            }
        }
        let s: String = cs[start..i.max(start + 1)].iter().collect();
        let n: f64 = s.parse().map_err(|_| format!("bad number `{s}` in path"))?;
        out.push(PTok::Num(n));
        i = i.max(start + 1);
    }
    Ok(out)
}

fn take(toks: &[PTok], i: &mut usize, n: usize, cmd: char) -> Result<Vec<f64>, String> {
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        match toks.get(*i) {
            Some(PTok::Num(x)) => {
                v.push(*x);
                *i += 1;
            }
            _ => return Err(format!("`{cmd}` expects {n} numbers")),
        }
    }
    Ok(v)
}

fn unit(x: f64, y: f64) -> Result<Pt, String> {
    let ok = |v: f64| (-1e-9..=1.0 + 1e-9).contains(&v);
    if ok(x) && ok(y) {
        Ok(Pt::new(x, y))
    } else {
        Err(format!(
            "path coordinates must be between 0 and 1, found ({x}, {y})"
        ))
    }
}

fn flag(v: f64) -> Result<bool, String> {
    if v == 0.0 {
        Ok(false)
    } else if v == 1.0 {
        Ok(true)
    } else {
        Err(format!("arc flags must be 0 or 1, found {v}"))
    }
}

/// Parses a unit-coordinate path (`M L H V C Q A Z`, absolute only) into subpaths.
/// Subpaths with no segments (a lone `M`) are dropped, and a path with none left is an
/// error. Arcs become cubic Béziers; `Z` adds the closing line when the subpath isn't closed.
pub fn parse_unit_path(d: &str) -> Result<Vec<Vec<PathSeg>>, String> {
    let toks = tokens(d)?;
    let mut subs: Vec<Vec<PathSeg>> = Vec::new();
    let mut cur: Option<Pt> = None;
    let mut start = Pt::default();
    let mut cmd: Option<char> = None;
    let mut i = 0;
    while i < toks.len() {
        if let PTok::Cmd(c) = toks[i] {
            i += 1;
            if "mlhvcqaz".contains(c) {
                return Err(format!(
                    "relative command `{c}` is not supported; use `{}`",
                    c.to_ascii_uppercase()
                ));
            }
            if !"MLHVCQAZ".contains(c) {
                return Err(format!("unknown path command `{c}`; use M L H V C Q A Z"));
            }
            if c != 'M' && cur.is_none() {
                return Err("a path must start with `M`".into());
            }
            if c == 'Z' {
                let p = cur.expect("checked above");
                if p.dist(start) > 1e-12 {
                    subs.last_mut()
                        .expect("`M` opened a subpath")
                        .push(PathSeg::Line { from: p, to: start });
                }
                cur = Some(start);
                cmd = None;
                continue;
            }
            cmd = Some(c);
        }
        let Some(c) = cmd else {
            return Err(if cur.is_none() {
                "a path must start with `M`".into()
            } else {
                "numbers must follow a command".into()
            });
        };
        if c == 'M' {
            let a = take(&toks, &mut i, 2, c)?;
            let p = unit(a[0], a[1])?;
            subs.push(Vec::new());
            cur = Some(p);
            start = p;
            cmd = Some('L');
            continue;
        }
        let from = cur.expect("`M` comes first");
        let segs = match c {
            'L' => {
                let a = take(&toks, &mut i, 2, c)?;
                vec![PathSeg::Line {
                    from,
                    to: unit(a[0], a[1])?,
                }]
            }
            'H' => {
                let a = take(&toks, &mut i, 1, c)?;
                vec![PathSeg::Line {
                    from,
                    to: unit(a[0], from.y)?,
                }]
            }
            'V' => {
                let a = take(&toks, &mut i, 1, c)?;
                vec![PathSeg::Line {
                    from,
                    to: unit(from.x, a[0])?,
                }]
            }
            'C' => {
                let a = take(&toks, &mut i, 6, c)?;
                vec![PathSeg::Cubic {
                    from,
                    c1: unit(a[0], a[1])?,
                    c2: unit(a[2], a[3])?,
                    to: unit(a[4], a[5])?,
                }]
            }
            'Q' => {
                let a = take(&toks, &mut i, 4, c)?;
                vec![PathSeg::Quad {
                    from,
                    ctrl: unit(a[0], a[1])?,
                    to: unit(a[2], a[3])?,
                }]
            }
            'A' => {
                let a = take(&toks, &mut i, 7, c)?;
                let to = unit(a[5], a[6])?;
                arc_to_cubics(from, a[0], a[1], a[2], flag(a[3])?, flag(a[4])?, to)
            }
            _ => unreachable!("commands are validated above"),
        };
        if let Some(last) = segs.last() {
            cur = Some(last.to());
        }
        subs.last_mut().expect("`M` opened a subpath").extend(segs);
    }
    subs.retain(|s| !s.is_empty());
    if subs.is_empty() {
        return Err("empty path".into());
    }
    Ok(subs)
}

/// Converts an SVG elliptical arc (endpoint form, SVG 1.1 F.6.5) to cubic Béziers of at
/// most 90° each. Radii that are too small are scaled up, as SVG requires.
pub fn arc_to_cubics(
    from: Pt,
    rx: f64,
    ry: f64,
    phi_deg: f64,
    large: bool,
    sweep: bool,
    to: Pt,
) -> Vec<PathSeg> {
    if from.dist(to) < 1e-12 {
        return Vec::new();
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-12 || ry < 1e-12 {
        return vec![PathSeg::Line { from, to }];
    }
    let (sin, cos) = phi_deg.to_radians().sin_cos();
    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let x1p = cos * dx + sin * dy;
    let y1p = -sin * dx + cos * dy;
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let mut coef = (num / den).max(0.0).sqrt();
    if large == sweep {
        coef = -coef;
    }
    let cxp = coef * rx * y1p / ry;
    let cyp = -coef * ry * x1p / rx;
    let cx = cos * cxp - sin * cyp + (from.x + to.x) / 2.0;
    let cy = sin * cxp + cos * cyp + (from.y + to.y) / 2.0;
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let (ux, uy) = ((x1p - cxp) / rx, (y1p - cyp) / ry);
    let (vx, vy) = ((-x1p - cxp) / rx, (-y1p - cyp) / ry);
    let theta1 = angle(1.0, 0.0, ux, uy);
    let mut dtheta = angle(ux, uy, vx, vy);
    if !sweep && dtheta > 0.0 {
        dtheta -= TAU;
    } else if sweep && dtheta < 0.0 {
        dtheta += TAU;
    }
    let n = ((dtheta.abs() / FRAC_PI_2).ceil() as usize).max(1);
    let step = dtheta / n as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let at = |t: f64| {
        let (st, ct) = t.sin_cos();
        let p = Pt::new(
            cx + rx * ct * cos - ry * st * sin,
            cy + rx * ct * sin + ry * st * cos,
        );
        let d = Pt::new(
            -rx * st * cos - ry * ct * sin,
            -rx * st * sin + ry * ct * cos,
        );
        (p, d)
    };
    let mut segs = Vec::with_capacity(n);
    let mut prev = from;
    for i in 0..n {
        let t1 = theta1 + step * i as f64;
        let (_, d1) = at(t1);
        let (p2, d2) = at(t1 + step);
        let end = if i == n - 1 { to } else { p2 };
        segs.push(PathSeg::Cubic {
            from: prev,
            c1: prev + d1 * k,
            c2: end - d2 * k,
            to: end,
        });
        prev = end;
    }
    segs
}
