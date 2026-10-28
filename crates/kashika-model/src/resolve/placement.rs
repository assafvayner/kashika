use std::collections::HashMap;

use kashika_diag::{Diagnostic, did_you_mean};
use kashika_geom::{Pt, Rect};
use kashika_syntax::ast::{self, Axis, Coord, Point, Target};

use crate::anchor;
use crate::attrs;
use crate::form;

use super::props::shape_name;
use super::{CYCLE_SHOWN, Item, MAX_PLACEMENT_DEPTH, Resolver};

impl<'a> Resolver<'a> {
    /// The placeable entries `idx`'s position depends on: its group, then each id in its `at`.
    pub(super) fn placement_deps(&self, idx: usize) -> Vec<usize> {
        let at = match self.entries[idx].item {
            Item::Box(n) => &n.at,
            Item::Group(g) => &g.node.at,
            Item::Image(i) => &i.at,
            Item::Junction(j) => &j.at,
            Item::Text(_) | Item::Line(_) => return Vec::new(),
        };
        let refs: Vec<&ast::Ref> = match at {
            Point::Ref { r, .. } => vec![r],
            Point::Lit { x, y, .. } => [x, y]
                .into_iter()
                .filter_map(|c| match c {
                    Coord::Axis { r, .. } => Some(r),
                    Coord::Num(_) => None,
                })
                .collect(),
        };
        let placeable = |i: &usize| !matches!(self.entries[*i].item, Item::Text(_) | Item::Line(_));
        self.entries[idx]
            .parent
            .into_iter()
            .chain(
                refs.into_iter()
                    .filter_map(|r| self.ids.get(&r.id.name).copied()),
            )
            .filter(placeable)
            .collect()
    }

    /// Reports placement cycles and over-long reference chains before anything is placed,
    /// whatever the statement order, and marks the entries involved as unplaceable.
    pub(super) fn placement_graph(&mut self) {
        let n = self.entries.len();
        let deps: Vec<Vec<usize>> = (0..n)
            .map(|i| {
                if self.skipped.contains(&i) {
                    Vec::new()
                } else {
                    self.placement_deps(i)
                }
            })
            .collect();
        let sccs = tarjan(&deps);
        let mut cyclic = vec![false; n];
        for scc in &sccs {
            let looped = scc.len() > 1 || deps[scc[0]].contains(&scc[0]);
            if !looped {
                continue;
            }
            for &i in scc {
                cyclic[i] = true;
                self.placed.insert(i, None);
            }
            let start = *scc.iter().min().expect("non-empty");
            let cycle = shortest_cycle(start, &deps, scc);
            self.report_cycle(&cycle);
        }

        // `depth[i]`: references in the longest chain from `i`; `next[i]`: its next link.
        let mut depth = vec![0usize; n];
        let mut next: Vec<Option<usize>> = vec![None; n];
        for scc in &sccs {
            let i = scc[0];
            if cyclic[i] {
                continue;
            }
            for &d in &deps[i] {
                if !cyclic[d] && depth[d] + 1 > depth[i] {
                    depth[i] = depth[d] + 1;
                    next[i] = Some(d);
                }
            }
        }
        let Some(head) = (0..n)
            .filter(|&i| depth[i] > MAX_PLACEMENT_DEPTH)
            .max_by(|&a, &b| depth[a].cmp(&depth[b]).then(b.cmp(&a)))
        else {
            return;
        };
        let mut tail = head;
        while let Some(t) = next[tail] {
            tail = t;
        }
        self.diags.push(
            Diagnostic::error(
                "placement-cycle",
                self.entries[head].item.at_span(),
                format!(
                    "placement chain from `{}` to `{}` is {} references deep; the limit is {MAX_PLACEMENT_DEPTH}",
                    self.name(head),
                    self.name(tail),
                    depth[head]
                ),
            )
            .with_help("place some shapes in this chain with literal coordinates"),
        );
        for (i, &d) in depth.iter().enumerate() {
            if d > MAX_PLACEMENT_DEPTH {
                self.placed.insert(i, None);
            }
        }
    }

    pub(super) fn report_cycle(&mut self, cycle: &[usize]) {
        let names: Vec<&str> = cycle.iter().map(|&i| self.name(i)).collect();
        let first = names[0];
        let message = if names.len() <= CYCLE_SHOWN {
            format!("placement cycle: {} -> {first}", names.join(" -> "))
        } else {
            format!(
                "placement cycle of {} shapes: {} -> … -> {} -> {first}",
                names.len(),
                names[..4].join(" -> "),
                names[names.len() - 1]
            )
        };
        self.diags.push(
            Diagnostic::error(
                "placement-cycle",
                self.entries[cycle[0]].item.at_span(),
                message,
            )
            .with_help("place one of these with literal coordinates"),
        );
    }

    /// Places an entry (box, group or junction), resolving dependencies depth-first.
    pub(super) fn place(&mut self, idx: usize) -> Option<Rect> {
        if let Some(r) = self.placed.get(&idx) {
            return *r;
        }
        if self.on_stack.len() > MAX_PLACEMENT_DEPTH || !self.on_stack.insert(idx) {
            return None;
        }
        let computed = self.compute_place(idx);
        self.on_stack.remove(&idx);
        let result = if self.placed.get(&idx) == Some(&None) {
            None
        } else {
            computed
        };
        self.placed.insert(idx, result);
        result
    }

    pub(super) fn pin(&mut self, a: &ast::Attr) -> Option<(&'static str, Option<f64>)> {
        let ast::Value::AnchorAt(name, t) = &a.value else {
            return attrs::word(a, anchor::ANCHORS, &mut self.diags).map(|n| (n, None));
        };
        let Some(&name) = anchor::ANCHORS.iter().find(|n| **n == name.as_str()) else {
            let mut d =
                Diagnostic::error("attr", a.span, format!("unknown anchor `{name}` in `pin`"));
            if let Some(s) = did_you_mean(name, anchor::ANCHORS.iter().copied()) {
                d = d.with_help(format!("did you mean `{s}`?"));
            }
            self.diags.push(d);
            return None;
        };
        let unit = Rect::new(0.0, 0.0, 1.0, 1.0);
        if let Err((msg, help)) = anchor::resolve(unit, 0.0, "pin", name, Some(*t)) {
            let mut d = Diagnostic::error("unknown-anchor", a.span, format!("{msg} in `pin`"));
            if let Some(h) = help {
                d = d.with_help(h);
            }
            self.diags.push(d);
            return None;
        }
        Some((name, Some(*t)))
    }

    pub(super) fn compute_place(&mut self, idx: usize) -> Option<Rect> {
        let origin = self.frame_origin(idx)?;
        match self.entries[idx].item {
            Item::Box(n) => self.place_node(idx, &n.at, n.size, origin),
            Item::Group(g) => self.place_node(idx, &g.node.at, g.node.size, origin),
            Item::Image(i) => self.place_node(idx, &i.at, i.size, origin),
            Item::Junction(j) => self
                .eval_point(&j.at, origin)
                .map(|p| Rect::new(p.x, p.y, 0.0, 0.0)),
            Item::Text(_) | Item::Line(_) => None,
        }
    }

    pub(super) fn place_node(
        &mut self,
        idx: usize,
        at: &'a Point,
        size: (f64, f64),
        origin: Pt,
    ) -> Option<Rect> {
        if !self.nodes[&idx].size_ok {
            return None;
        }
        let at = self.eval_point(at, origin)?;
        let (w, h) = size;

        let p = &self.nodes[&idx];
        let local = Rect::new(0.0, 0.0, w, h);
        let (name, t) = p.pin;
        let radius = p.style.radius;
        let shape = shape_name(p);
        let form = self.form(&shape, local);
        let off =
            form::resolve_anchor(form.as_ref(), local, radius, "pin", name, t).unwrap_or_default();
        Some(Rect::new(at.x - off.x, at.y - off.y, w, h))
    }

    /// Canvas position of the frame an entry's literal coordinates are written in.
    pub(super) fn frame_origin(&mut self, idx: usize) -> Option<Pt> {
        match self.entries[idx].parent {
            None => Some(Pt::new(0.0, 0.0)),
            Some(p) => self.place(p).map(Rect::origin),
        }
    }

    pub(super) fn eval_point(&mut self, p: &'a Point, origin: Pt) -> Option<Pt> {
        match p {
            Point::Lit { x, y, span } => {
                let x = self.eval_coord(x, origin.x)?;
                let y = self.eval_coord(y, origin.y)?;
                self.finite(&[x, y], *span).then(|| Pt::new(x, y))
            }
            Point::Ref { r, offset, span } => {
                let base = self.eval_ref(r)?;
                let (dx, dy) = offset.map_or((0.0, 0.0), |o| o.delta());
                let p = base + Pt::new(dx, dy);
                self.finite(&[p.x, p.y], *span).then_some(p)
            }
        }
    }

    pub(super) fn eval_coord(&mut self, c: &'a Coord, origin: f64) -> Option<f64> {
        match c {
            Coord::Num(n) => Some(origin + n),
            Coord::Axis { r, axis } => {
                let p = self.eval_ref(r)?;
                Some(match axis {
                    Axis::X => p.x,
                    Axis::Y => p.y,
                })
            }
        }
    }

    pub(super) fn eval_target(&mut self, t: &'a Target, origin: Pt, axis: Axis) -> Option<f64> {
        match (t, axis) {
            (Target::Num(n), Axis::X) => Some(origin.x + n),
            (Target::Num(n), Axis::Y) => Some(origin.y + n),
            (Target::Point(p), Axis::X) => self.eval_point(p, origin).map(|p| p.x),
            (Target::Point(p), Axis::Y) => self.eval_point(p, origin).map(|p| p.y),
        }
    }

    pub(super) fn eval_ref(&mut self, r: &'a ast::Ref) -> Option<Pt> {
        let name = &r.id.name;
        let Some(&idx) = self.ids.get(name) else {
            if self.skipped_ids.contains(name) {
                return None;
            }
            let d = self.unknown_id(name, r.id.span);
            self.diags.push(d);
            return None;
        };
        match self.entries[idx].item {
            Item::Junction(_) => {
                if let Some(a) = &r.anchor {
                    self.diags.push(
                        Diagnostic::error(
                            "unknown-anchor",
                            a.name.span,
                            format!("`{name}` is a junction and has no anchors"),
                        )
                        .with_help(format!("use `{name}` on its own")),
                    );
                    return None;
                }
                self.place(idx).map(Rect::origin)
            }
            Item::Box(_) | Item::Group(_) | Item::Image(_) => {
                let Some(a) = &r.anchor else {
                    self.diags.push(
                        Diagnostic::error(
                            "unknown-anchor",
                            r.id.span,
                            format!("`{name}` needs an anchor"),
                        )
                        .with_help(format!(
                            "e.g. `{name}.left`, `{name}.top` or `{name}.center`"
                        )),
                    );
                    return None;
                };
                let rect = self.place(idx)?;
                let p = &self.nodes[&idx];
                let form = p.shape.as_deref().and_then(|s| self.form(s, rect));
                match form::resolve_anchor(
                    form.as_ref(),
                    rect,
                    p.style.radius,
                    name,
                    &a.name.name,
                    a.t,
                ) {
                    Ok(p) => Some(p),
                    Err((msg, help)) => {
                        let mut d = Diagnostic::error("unknown-anchor", a.name.span, msg);
                        if let Some(h) = help {
                            d = d.with_help(h);
                        }
                        self.diags.push(d);
                        None
                    }
                }
            }
            Item::Text(_) | Item::Line(_) => {
                self.diags.push(Diagnostic::error(
                    "unknown-id",
                    r.id.span,
                    format!("`{name}` is a text or line; only boxes, groups and junctions can be referenced"),
                ));
                None
            }
        }
    }
}

/// Strongly connected components of `deps`, each listed after every component it depends on.
pub(super) fn tarjan(deps: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = deps.len();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0; n];
    let mut on = vec![false; n];
    let mut stack = Vec::new();
    let mut sccs = Vec::new();
    let mut counter = 0;
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut work = vec![(root, 0usize)];
        while let Some(&mut (v, ref mut k)) = work.last_mut() {
            if *k == 0 && index[v] == usize::MAX {
                index[v] = counter;
                low[v] = counter;
                counter += 1;
                stack.push(v);
                on[v] = true;
            }
            if let Some(&w) = deps[v].get(*k) {
                *k += 1;
                if index[w] == usize::MAX {
                    work.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(index[w]);
                }
                continue;
            }
            work.pop();
            if let Some(&(u, _)) = work.last() {
                low[u] = low[u].min(low[v]);
            }
            if low[v] == index[v] {
                let mut scc = Vec::new();
                while let Some(w) = stack.pop() {
                    on[w] = false;
                    scc.push(w);
                    if w == v {
                        break;
                    }
                }
                scc.sort_unstable();
                sccs.push(scc);
            }
        }
    }
    sccs
}

/// The shortest cycle through `start` inside its component `scc`, starting at `start`.
pub(super) fn shortest_cycle(start: usize, deps: &[Vec<usize>], scc: &[usize]) -> Vec<usize> {
    let mut prev: HashMap<usize, usize> = HashMap::new();
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some(v) = queue.pop_front() {
        for &w in &deps[v] {
            if w == start {
                let mut cycle = vec![v];
                let mut cur = v;
                while cur != start {
                    cur = prev[&cur];
                    cycle.push(cur);
                }
                cycle.reverse();
                return cycle;
            }
            if scc.binary_search(&w).is_ok() && !prev.contains_key(&w) {
                prev.insert(w, v);
                queue.push_back(w);
            }
        }
    }
    vec![start]
}
