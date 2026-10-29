use std::collections::HashMap;

use kashika_diag::Diagnostic;
use kashika_geom::{Pt, Rect};
use kashika_syntax::ast::{self, Axis, Coord, Point, Target};

use crate::anchor;
use crate::attrs;
use crate::form;
use crate::scene::ShapeKind;
use crate::textlayout::{self, ShapeInput};

use super::build::Built;
use super::canvas::Reach;
use super::{CYCLE_SHOWN, Item, MAX_PLACEMENT_DEPTH, Resolver, record};

impl<'a> Resolver<'a> {
    /// The entries `idx`'s position depends on: its group (or that group's frame, when the
    /// group is fitted), then each id in its `at`. A fitted group's rect depends on its frame
    /// and everything inside it; texts and lines only take part inside a fitted group.
    fn placement_deps(&self, idx: usize, frame_of: &HashMap<usize, usize>) -> Vec<usize> {
        if frame_of.contains_key(&idx) {
            return std::iter::once(frame_of[&idx])
                .chain(self.descendants(idx).filter(|d| !self.skipped.contains(d)))
                .collect();
        }
        let refs = match self.entries[idx].item {
            Item::Box(n) => record::point_refs(&n.at),
            Item::Image(i) => record::point_refs(&i.at),
            Item::Junction(j) => record::point_refs(&j.at),
            Item::Text(t) if self.in_fitted(idx) => record::point_refs(&t.at),
            Item::Line(l) if self.in_fitted(idx) => record::line_points(l)
                .into_iter()
                .flat_map(record::point_refs)
                .collect(),
            Item::Group(_) => return self.frame_deps(idx, frame_of),
            Item::Text(_) | Item::Line(_) => return Vec::new(),
        };
        self.with_parent(idx, refs, frame_of)
    }

    /// What a group's frame depends on: its parent's frame and the ids in its `at`.
    fn frame_deps(&self, idx: usize, frame_of: &HashMap<usize, usize>) -> Vec<usize> {
        let Item::Group(g) = self.entries[idx].item else {
            return Vec::new();
        };
        self.with_parent(idx, record::point_refs(&g.node.at), frame_of)
    }

    fn with_parent(
        &self,
        idx: usize,
        refs: Vec<&ast::Ref>,
        frame_of: &HashMap<usize, usize>,
    ) -> Vec<usize> {
        let placeable = |i: &usize| !matches!(self.entries[*i].item, Item::Text(_) | Item::Line(_));
        let parent = self.entries[idx]
            .parent
            .map(|p| frame_of.get(&p).copied().unwrap_or(p));
        parent
            .into_iter()
            .chain(
                refs.into_iter()
                    .filter_map(|r| self.ids.get(&r.id.name).copied())
                    .filter(placeable),
            )
            .collect()
    }

    /// The entries inside group `idx`, at any depth; they follow it in order.
    pub(super) fn descendants(&self, idx: usize) -> std::ops::Range<usize> {
        let depth = self.entries[idx].depth;
        let end = (idx + 1..self.entries.len())
            .find(|&i| self.entries[i].depth <= depth)
            .unwrap_or(self.entries.len());
        idx + 1..end
    }

    pub(super) fn is_fitted(&self, idx: usize) -> bool {
        self.nodes
            .get(&idx)
            .is_some_and(|p| p.fitted != (false, false))
    }

    fn in_fitted(&self, idx: usize) -> bool {
        let mut p = self.entries[idx].parent;
        while let Some(i) = p {
            if self.is_fitted(i) {
                return true;
            }
            p = self.entries[i].parent;
        }
        false
    }

    /// Reports placement cycles and over-long reference chains before anything is placed,
    /// whatever the statement order, and marks the entries involved as unplaceable. A fitted
    /// group is two nodes: its frame, which its children use, and its rect, which needs them.
    pub(super) fn placement_graph(&mut self) {
        let n = self.entries.len();
        let fitted: Vec<usize> = (0..n)
            .filter(|i| !self.skipped.contains(i) && self.is_fitted(*i))
            .collect();
        let frame_of: HashMap<usize, usize> = fitted
            .iter()
            .enumerate()
            .map(|(k, &g)| (g, n + k))
            .collect();
        let mut deps: Vec<Vec<usize>> = (0..n)
            .map(|i| {
                if self.skipped.contains(&i) {
                    Vec::new()
                } else {
                    self.placement_deps(i, &frame_of)
                }
            })
            .collect();
        deps.extend(fitted.iter().map(|&g| self.frame_deps(g, &frame_of)));
        let entry = |i: usize| if i < n { i } else { fitted[i - n] };
        let total = deps.len();
        let sccs = tarjan(&deps);
        let mut cyclic = vec![false; total];
        for scc in &sccs {
            let looped = scc.len() > 1 || deps[scc[0]].contains(&scc[0]);
            if !looped {
                continue;
            }
            for &i in scc {
                cyclic[i] = true;
                self.unplaceable(i, n, &fitted);
            }
            let start = *scc.iter().min().expect("non-empty");
            let mut cycle = shortest_cycle(start, &deps, scc);
            let grown = cycle.iter().position(|&i| {
                i < n
                    && frame_of.contains_key(&i)
                    && cycle
                        .iter()
                        .any(|&d| d < n && self.descendants(i).contains(&d))
            });
            let help = grown.map(|k| {
                let g = cycle[k];
                let len = cycle.len();
                cycle.rotate_left((k + 1) % len);
                let g = self.name(g);
                format!(
                    "`{g}` is fitted to its children, so they can't be placed against its size; give `{g}` a size: `size (w,h)`"
                )
            });
            let mut named: Vec<usize> = cycle.iter().map(|&i| entry(i)).collect();
            named.dedup();
            if named.len() > 1 && named.first() == named.last() {
                named.pop();
            }
            self.report_cycle(&named, help);
        }

        // `depth[i]`: references in the longest chain from `i`; `next[i]`: its next link.
        let mut depth = vec![0usize; total];
        let mut next: Vec<Option<usize>> = vec![None; total];
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
        let Some(head) = (0..total)
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
                self.entries[entry(head)].item.at_span(),
                format!(
                    "placement chain from `{}` to `{}` is {} references deep; the limit is {MAX_PLACEMENT_DEPTH}",
                    self.name(entry(head)),
                    self.name(entry(tail)),
                    depth[head]
                ),
            )
            .with_help("place some shapes in this chain with literal coordinates"),
        );
        for (i, &d) in depth.iter().enumerate() {
            if d > MAX_PLACEMENT_DEPTH {
                self.unplaceable(i, n, &fitted);
            }
        }
    }

    fn unplaceable(&mut self, node: usize, n: usize, fitted: &[usize]) {
        if node < n {
            self.placed[node] = Some(None);
        } else {
            self.frames.insert(fitted[node - n], None);
        }
    }

    pub(super) fn report_cycle(&mut self, cycle: &[usize], help: Option<String>) {
        let names: Vec<String> = cycle.iter().map(|&i| self.name(i)).collect();
        let first = &names[0];
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
            .with_help(
                help.unwrap_or_else(|| "place one of these with literal coordinates".into()),
            ),
        );
    }

    /// Places an entry (box, group or junction), resolving dependencies depth-first.
    pub(super) fn place(&mut self, idx: usize) -> Option<Rect> {
        if let Some(r) = self.placed[idx] {
            return r;
        }
        if self.on_stack.len() > MAX_PLACEMENT_DEPTH || !self.on_stack.insert(idx) {
            return None;
        }
        let r = self.compute_place(idx);
        self.on_stack.remove(&idx);
        self.placed[idx] = Some(r);
        r
    }

    pub(super) fn pin(&mut self, a: &ast::Attr) -> Option<(&'static str, Option<f64>)> {
        let ast::Value::AnchorAt(name, t) = &a.value else {
            return attrs::word(a, anchor::ANCHOR_NAMES, &mut self.diags).map(|n| (n, None));
        };
        let Some(&name) = anchor::ANCHOR_NAMES.iter().find(|n| **n == name.as_str()) else {
            let mut d =
                Diagnostic::error("attr", a.span, format!("unknown anchor `{name}` in `pin`"));
            if let Some(s) = attrs::suggest_word(name, anchor::ANCHOR_NAMES) {
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
            Item::Box(n) => self.place_node(idx, &n.at, origin),
            Item::Group(g) if self.is_fitted(idx) => self.place_fitted(idx, g),
            Item::Group(g) => self.place_node(idx, &g.node.at, origin),
            Item::Image(i) => self.place_node(idx, &i.at, origin),
            Item::Junction(j) => self
                .eval_point(&j.at, origin)
                .map(|p| Rect::new(p.x, p.y, 0.0, 0.0)),
            Item::Text(_) | Item::Line(_) => None,
        }
    }

    pub(super) fn place_node(&mut self, idx: usize, at: &'a Point, origin: Pt) -> Option<Rect> {
        let (w, h) = self.nodes[&idx].size?;
        let at = self.eval_point(at, origin)?;

        let p = &self.nodes[&idx];
        let local = Rect::new(0.0, 0.0, w, h);
        let (name, t) = p.pin;
        let radius = p.style.radius;
        let shape = p.drawn_shape();
        let form = self.form(&shape, local);
        let off =
            form::resolve_anchor(form.as_ref(), local, radius, "pin", name, t).unwrap_or_default();
        Some(Rect::new(at.x - off.x, at.y - off.y, w, h))
    }

    /// Canvas position of the frame an entry's literal coordinates are written in.
    pub(super) fn frame_origin(&mut self, idx: usize) -> Option<Pt> {
        match self.entries[idx].parent {
            None => Some(Pt::new(0.0, 0.0)),
            Some(p) if self.is_fitted(p) => self.fitted_frame(p),
            Some(p) => self.place(p).map(Rect::origin),
        }
    }

    /// A fitted group's top-left corner, which doesn't depend on its fitted size: `at`
    /// minus the pin's bounding-box offset, which is 0 on a fitted axis.
    fn fitted_frame(&mut self, idx: usize) -> Option<Pt> {
        if let Some(&f) = self.frames.get(&idx) {
            return f;
        }
        self.frames.insert(idx, None);
        let Item::Group(g) = self.entries[idx].item else {
            return None;
        };
        let origin = self.frame_origin(idx)?;
        let at = self.eval_point(&g.node.at, origin)?;
        let p = &self.nodes[&idx];
        let (w, h) = p.size?;
        let (name, t) = p.pin;
        let unit = anchor::resolve(Rect::new(0.0, 0.0, 1.0, 1.0), 0.0, "pin", name, t).ok()?;
        let f = Pt::new(at.x - unit.x * w, at.y - unit.y * h);
        self.frames.insert(idx, Some(f));
        Some(f)
    }

    /// Places a fitted group: from its frame to the far edge of everything inside it, plus
    /// `padding`, and at least as large as its title. A fitted axis is rounded up to whole
    /// pixels, then to the `grid`.
    fn place_fitted(&mut self, idx: usize, g: &'a ast::Group) -> Option<Rect> {
        let origin = self.fitted_frame(idx)?;
        let mut reach = Reach::default();
        for d in self.descendants(idx) {
            self.ensure_built(d);
            match self.built.get(&d) {
                Some(Some(Built::Shape(s))) => reach.shape(s),
                Some(Some(Built::Junction(j))) => reach.junction(j),
                Some(Some(Built::Text(t))) => reach.text(t),
                Some(Some(Built::Line(l))) => reach.line(l),
                _ => {}
            }
        }
        let p = &self.nodes[&idx];
        let (w, h) = p.size?;
        let shape = p.drawn_shape();
        let title = textlayout::fit_size(&ShapeInput {
            id: &g.node.id.name,
            node: &g.node,
            kind: ShapeKind::Group,
            shape: &shape,
            custom_text_area: None,
            text_below: false,
            icon_inset: p.icon_inset(true),
            rect: Rect::new(0.0, 0.0, w, h),
            style: &p.style,
            props: &p.text,
            block_at: &[],
        });
        let (far_x, far_y) = reach.far();
        let what = format!("`{}`", g.node.id.name);
        if far_x == f64::NEG_INFINITY && title.is_none() {
            self.diags.push(
                Diagnostic::error(
                    "size",
                    g.node.span,
                    format!("group {what} has no size to fit: it has no children or label"),
                )
                .with_help("give it a size: `size (w,h)`"),
            );
            return None;
        }
        let pad = p.style.padding;
        let grid = self.grid;
        let fit = |far: f64, o: f64, pad: f64, title: Option<f64>| {
            let inner = if far == f64::NEG_INFINITY {
                0.0
            } else {
                (far - o).max(0.0) + pad
            };
            let v = inner.max(title.unwrap_or(0.0)).ceil();
            grid.map_or(v, |g| (v / g).ceil() * g)
        };
        let (fw, fh) = p.fitted;
        let size = (
            if fw {
                fit(far_x, origin.x, pad.x, title.map(|t| t.0))
            } else {
                w
            },
            if fh {
                fit(far_y, origin.y, pad.y, title.map(|t| t.1))
            } else {
                h
            },
        );
        if !self.size_ok(size, g.node.span, &what) {
            return None;
        }
        if let Some(p) = self.nodes.get_mut(&idx) {
            p.size = Some(size);
        }
        Some(Rect::new(origin.x, origin.y, size.0, size.1))
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
            Point::Group { inner, .. } => self.eval_point(inner, origin),
            Point::Shifted {
                base,
                offsets,
                span,
            } => {
                let mut p = self.eval_point(base, origin)?;
                for o in offsets {
                    let (dx, dy) = o.delta();
                    p = p + Pt::new(dx, dy);
                }
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
