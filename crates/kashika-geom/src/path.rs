use serde::Serialize;

use crate::point::{Pt, TOLERANCE};

/// Maximum distance between a curve and its flattened polyline.
pub const FLATTEN_TOLERANCE: f64 = 0.25;

/// Upper bound on the pieces one curve is flattened into, whatever its size.
pub const MAX_FLATTEN_STEPS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PathSeg {
    Line { from: Pt, to: Pt },
    Quad { from: Pt, ctrl: Pt, to: Pt },
    Cubic { from: Pt, c1: Pt, c2: Pt, to: Pt },
}

impl PathSeg {
    pub fn from(&self) -> Pt {
        match *self {
            PathSeg::Line { from, .. }
            | PathSeg::Quad { from, .. }
            | PathSeg::Cubic { from, .. } => from,
        }
    }

    pub fn to(&self) -> Pt {
        match *self {
            PathSeg::Line { to, .. } | PathSeg::Quad { to, .. } | PathSeg::Cubic { to, .. } => to,
        }
    }

    /// Applies `f` to every point of the segment, control points included.
    pub fn map(&self, f: impl Fn(Pt) -> Pt) -> PathSeg {
        match *self {
            PathSeg::Line { from, to } => PathSeg::Line {
                from: f(from),
                to: f(to),
            },
            PathSeg::Quad { from, ctrl, to } => PathSeg::Quad {
                from: f(from),
                ctrl: f(ctrl),
                to: f(to),
            },
            PathSeg::Cubic { from, c1, c2, to } => PathSeg::Cubic {
                from: f(from),
                c1: f(c1),
                c2: f(c2),
                to: f(to),
            },
        }
    }

    pub fn eval(&self, t: f64) -> Pt {
        let s = 1.0 - t;
        match *self {
            PathSeg::Line { from, to } => from.lerp(to, t),
            PathSeg::Quad { from, ctrl, to } => {
                from * (s * s) + ctrl * (2.0 * s * t) + to * (t * t)
            }
            PathSeg::Cubic { from, c1, c2, to } => {
                from * (s * s * s)
                    + c1 * (3.0 * s * s * t)
                    + c2 * (3.0 * s * t * t)
                    + to * (t * t * t)
            }
        }
    }

    pub fn is_axis_aligned(&self) -> bool {
        match *self {
            PathSeg::Line { from, to } => {
                (from.x - to.x).abs() <= TOLERANCE || (from.y - to.y).abs() <= TOLERANCE
            }
            _ => false,
        }
    }

    pub fn start_dir(&self) -> Pt {
        let d = match *self {
            PathSeg::Line { from, to } => to - from,
            PathSeg::Quad { from, ctrl, to } => nonzero(ctrl - from, to - from),
            PathSeg::Cubic { from, c1, to, .. } => nonzero(c1 - from, to - from),
        };
        d.norm()
    }

    pub fn end_dir(&self) -> Pt {
        let d = match *self {
            PathSeg::Line { from, to } => to - from,
            PathSeg::Quad { from, ctrl, to } => nonzero(to - ctrl, to - from),
            PathSeg::Cubic { from, c2, to, .. } => nonzero(to - c2, to - from),
        };
        d.norm()
    }

    pub fn length(&self) -> f64 {
        length(std::slice::from_ref(self))
    }

    /// Splits the segment at parameter `t` (de Casteljau).
    pub fn split(&self, t: f64) -> (PathSeg, PathSeg) {
        match *self {
            PathSeg::Line { from, to } => {
                let m = from.lerp(to, t);
                (PathSeg::Line { from, to: m }, PathSeg::Line { from: m, to })
            }
            PathSeg::Quad { from, ctrl, to } => {
                let a = from.lerp(ctrl, t);
                let b = ctrl.lerp(to, t);
                let m = a.lerp(b, t);
                (
                    PathSeg::Quad {
                        from,
                        ctrl: a,
                        to: m,
                    },
                    PathSeg::Quad {
                        from: m,
                        ctrl: b,
                        to,
                    },
                )
            }
            PathSeg::Cubic { from, c1, c2, to } => {
                let a = from.lerp(c1, t);
                let b = c1.lerp(c2, t);
                let c = c2.lerp(to, t);
                let d = a.lerp(b, t);
                let e = b.lerp(c, t);
                let m = d.lerp(e, t);
                (
                    PathSeg::Cubic {
                        from,
                        c1: a,
                        c2: d,
                        to: m,
                    },
                    PathSeg::Cubic {
                        from: m,
                        c1: e,
                        c2: c,
                        to,
                    },
                )
            }
        }
    }

    /// Removes `start` and `end` px of arc length from the two ends. When they add up to
    /// more than the segment's length, each end gives up length in proportion.
    pub fn trim(&self, start: f64, end: f64) -> PathSeg {
        let len = self.length();
        let (mut a, mut b) = (start.max(0.0), end.max(0.0));
        if a + b > len {
            let k = len / (a + b);
            a *= k;
            b *= k;
        }
        if let PathSeg::Line { from, to } = *self {
            let dir = (to - from).norm();
            return PathSeg::Line {
                from: from + dir * a,
                to: to - dir * b,
            };
        }
        let t0 = self.param_at(a);
        let t1 = self.param_at(len - b).max(t0);
        let mut seg = *self;
        if a > 0.0 {
            seg = seg.split(t0).1;
        }
        if b > 0.0 && t0 < 1.0 {
            seg = seg.split((t1 - t0) / (1.0 - t0)).0;
        }
        seg
    }

    /// The parameter at arc length `s` from the start, measured on the flattened curve.
    fn param_at(&self, s: f64) -> f64 {
        let n = self.flatten_steps();
        let mut prev = self.from();
        let mut acc = 0.0;
        for k in 1..=n {
            let p = self.eval(k as f64 / n as f64);
            let d = prev.dist(p);
            if acc + d >= s && d > 0.0 {
                return ((k - 1) as f64 + (s - acc) / d) / n as f64;
            }
            acc += d;
            prev = p;
        }
        1.0
    }

    fn flatten_steps(&self) -> usize {
        match *self {
            PathSeg::Line { .. } => 1,
            PathSeg::Quad { from, ctrl, to } => steps((from - ctrl * 2.0 + to).length() / 4.0),
            PathSeg::Cubic { from, c1, c2, to } => steps(
                0.75 * (from - c1 * 2.0 + c2)
                    .length()
                    .max((c1 - c2 * 2.0 + to).length()),
            ),
        }
    }
}

fn nonzero(a: Pt, fallback: Pt) -> Pt {
    if a.length() > 1e-9 { a } else { fallback }
}

/// Uniform step count that keeps the chord error of a curve with the given
/// second-derivative bound under FLATTEN_TOLERANCE.
fn steps(deviation: f64) -> usize {
    ((deviation / FLATTEN_TOLERANCE).sqrt().ceil() as usize).clamp(1, MAX_FLATTEN_STEPS)
}

pub fn flatten(path: &[PathSeg]) -> Vec<Pt> {
    let mut pts = Vec::new();
    for seg in path {
        if pts.is_empty() {
            pts.push(seg.from());
        }
        let n = seg.flatten_steps();
        pts.extend((1..=n).map(|k| seg.eval(k as f64 / n as f64)));
    }
    pts
}

pub fn length(path: &[PathSeg]) -> f64 {
    flatten(path).windows(2).map(|w| w[0].dist(w[1])).sum()
}

pub fn point_at(path: &[PathSeg], t: f64) -> Pt {
    let pts = flatten(path);
    let total: f64 = pts.windows(2).map(|w| w[0].dist(w[1])).sum();
    let mut remaining = total * t.clamp(0.0, 1.0);
    for w in pts.windows(2) {
        let d = w[0].dist(w[1]);
        if remaining <= d && d > 0.0 {
            return w[0].lerp(w[1], remaining / d);
        }
        remaining -= d;
    }
    pts.last().copied().unwrap_or_default()
}

/// Rounds the corners between consecutive straight segments; curves pass through unchanged.
pub fn round_corners(path: &[PathSeg], r: f64) -> Vec<PathSeg> {
    let tagged: Vec<(PathSeg, ())> = path.iter().map(|s| (*s, ())).collect();
    round_corners_tagged(&tagged, r)
        .into_iter()
        .map(|(s, _)| s)
        .collect()
}

/// Like `round_corners`, carrying each input segment's tag to the pieces made from it.
/// A corner arc takes the tag of the segment that leads into it.
pub fn round_corners_tagged<T: Copy>(path: &[(PathSeg, T)], r: f64) -> Vec<(PathSeg, T)> {
    let mut out = Vec::new();
    let mut run: Vec<(Pt, T)> = Vec::new();
    for &(seg, tag) in path {
        match seg {
            PathSeg::Line { from, to } => {
                if run.is_empty() {
                    run.push((from, tag));
                }
                run.push((to, tag));
            }
            other => {
                flush_run(&mut run, r, &mut out);
                out.push((other, tag));
            }
        }
    }
    flush_run(&mut run, r, &mut out);
    out
}

/// `run[0]` is the start point; `run[k]` is the end of straight segment `k-1` with its tag.
fn flush_run<T: Copy>(run: &mut Vec<(Pt, T)>, r: f64, out: &mut Vec<(PathSeg, T)>) {
    if run.len() < 2 {
        run.clear();
        return;
    }
    let mut cursor = run[0].0;
    for w in run.windows(3) {
        let (prev, (corner, tag), next) = (w[0].0, w[1], w[2].0);
        let re = r.min(prev.dist(corner) / 2.0).min(corner.dist(next) / 2.0);
        if re <= 1e-9 {
            if cursor.dist(corner) > 1e-9 {
                out.push((
                    PathSeg::Line {
                        from: cursor,
                        to: corner,
                    },
                    tag,
                ));
            }
            cursor = corner;
            continue;
        }
        let a = corner + (prev - corner).norm() * re;
        let b = corner + (next - corner).norm() * re;
        if cursor.dist(a) > 1e-9 {
            out.push((
                PathSeg::Line {
                    from: cursor,
                    to: a,
                },
                tag,
            ));
        }
        out.push((
            PathSeg::Quad {
                from: a,
                ctrl: corner,
                to: b,
            },
            tag,
        ));
        cursor = b;
    }
    let (last, tag) = run[run.len() - 1];
    if cursor.dist(last) > 1e-9 {
        out.push((
            PathSeg::Line {
                from: cursor,
                to: last,
            },
            tag,
        ));
    }
    run.clear();
}
