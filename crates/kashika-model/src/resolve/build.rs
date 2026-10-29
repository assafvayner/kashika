use kashika_geom::{Pt, Rect};
use kashika_syntax::ast::{self};

use crate::library::LibShape;
use crate::scene::*;
use crate::textlayout::{self, ShapeInput};

use super::{IconPos, Item, Resolver};

/// An entry's scene item, built before `build` adds it to the scene.
pub(super) enum Built {
    Shape(Box<Shape>),
    Junction(Junction),
    Text(Text),
    Line(Line),
}

impl<'a> Resolver<'a> {
    pub(super) fn build(&mut self, scene: &mut Scene) {
        for idx in 0..self.entries.len() {
            let entry = &self.entries[idx];
            if self.skipped.contains(&idx)
                || entry.parent.is_some_and(|p| self.skipped.contains(&p))
            {
                self.skipped.insert(idx);
                continue;
            }
            let item = entry.item;
            self.ensure_built(idx);
            let built = self.built.get_mut(&idx).and_then(Option::take);
            match item {
                Item::Box(n) | Item::Group(ast::Group { node: n, .. }) => {
                    self.record_edges(idx, &n.at);
                    if let Some(Built::Shape(s)) = built {
                        scene.shapes.push(*s);
                        self.record_node(n);
                    }
                }
                Item::Image(i) => {
                    self.record_edges(idx, &i.at);
                    if let Some(Built::Shape(s)) = built {
                        scene.shapes.push(*s);
                        self.record_point(&i.id.name, &i.at);
                        self.literal(&i.id.name, "width", i.size.0, i.span);
                        self.literal(&i.id.name, "height", i.size.1, i.span);
                    }
                }
                Item::Junction(j) => {
                    self.record_edges(idx, &j.at);
                    if let Some(Built::Junction(x)) = built {
                        scene.junctions.push(x);
                        self.record_point(&j.id.name, &j.at);
                    }
                }
                Item::Text(t) => {
                    if let Some(Built::Text(x)) = built {
                        scene.texts.push(x);
                        let owner =
                            t.id.as_ref()
                                .map_or_else(|| unnamed_text_id(t.span.line), |i| i.name.clone());
                        self.record_point(&owner, &t.at);
                    }
                }
                Item::Line(l) => {
                    if let Some(Built::Line(x)) = built {
                        let id = x.id.clone();
                        scene.lines.push(x);
                        self.record_line(&id, l);
                    }
                }
            }
        }
        scene.placement = std::mem::take(&mut self.edges);
        scene.literals = std::mem::take(&mut self.literals);
    }

    /// Builds an entry's scene item once; a fitted group builds its children early.
    pub(super) fn ensure_built(&mut self, idx: usize) {
        if self.built.contains_key(&idx) {
            return;
        }
        let built = self.make(idx);
        self.built.insert(idx, built);
    }

    fn make(&mut self, idx: usize) -> Option<Built> {
        if self.skipped.contains(&idx) {
            return None;
        }
        let entry = &self.entries[idx];
        let (item, depth) = (entry.item, entry.depth);
        let parent = entry
            .parent
            .and_then(|p| self.entries[p].item.id())
            .map(|i| i.name.clone());
        match item {
            Item::Box(n) => self
                .make_shape(idx, n, ShapeKind::Box, parent, depth)
                .map(|s| Built::Shape(Box::new(s))),
            Item::Group(g) => self
                .make_shape(idx, &g.node, ShapeKind::Group, parent, depth)
                .map(|s| Built::Shape(Box::new(s))),
            Item::Image(i) => self.make_image(idx, i).map(|s| Built::Shape(Box::new(s))),
            Item::Junction(j) => {
                let r = self.place(idx)?;
                let dot = self.entries[idx].dot();
                let origin = self.frame_origin(idx).unwrap_or_default();
                Some(Built::Junction(Junction {
                    id: j.id.name.clone(),
                    at: r.origin(),
                    dot,
                    parent,
                    origin,
                    span: j.span,
                }))
            }
            Item::Text(t) => {
                let origin = self.frame_origin(idx)?;
                let at = self.eval_point(&t.at, origin)?;
                let p = self.entries[idx].text()?;
                let block =
                    textlayout::measure_at(&t.content, p.size, false, t.span, &mut self.diags)?;
                let (lines, rect) = block.place_at(at.x, at.y, textlayout::halign(p.align));
                Some(Built::Text(Text {
                    id: t.id.as_ref().map(|i| i.name.clone()),
                    content: p.content.clone(),
                    at,
                    size: p.size,
                    color: p.color.clone(),
                    align: p.align,
                    rect,
                    lines,
                    parent,
                    origin,
                    span: t.span,
                }))
            }
            Item::Line(l) => self.build_line(idx, l, parent).map(Built::Line),
        }
    }

    fn make_shape(
        &mut self,
        idx: usize,
        n: &'a ast::Node,
        kind: ShapeKind,
        parent: Option<String>,
        depth: usize,
    ) -> Option<Shape> {
        let rect = self.place(idx)?;
        let origin = self.frame_origin(idx).unwrap_or_default();
        let block_at: Vec<Option<Pt>> = n
            .block
            .iter()
            .flatten()
            .map(|item| {
                item.at
                    .as_ref()
                    .and_then(|at| self.eval_point(at, rect.origin()))
            })
            .collect();
        let p = self.entries[idx].node()?;
        let shape = p.drawn_shape();
        let form = self.form(&shape, rect);
        let svg = self.svg_shape(&shape);
        let icon = p.icon;
        let icon_inset = p.icon_inset(kind == ShapeKind::Group);
        let text = textlayout::layout_shape(
            &ShapeInput {
                id: &n.id.name,
                node: n,
                kind,
                shape: &shape,
                custom_text_area: self.lib.shapes.get(&shape).and_then(LibShape::text_area),
                text_below: svg.is_some_and(|s| s.below),
                contain: self.contained(&shape),
                icon_inset,
                rect,
                style: &p.style,
                props: &p.text,
                block_at: &block_at,
            },
            &mut self.diags,
        );
        Some(Shape {
            id: n.id.name.clone(),
            kind,
            outline: p.outline,
            form,
            shape,
            image: svg.map(|s| ImageRef {
                asset: s.asset,
                rect,
                fit: s.fit,
            }),
            icon: icon.map(|ic| {
                let pad = p.style.padding;
                let s = ic.size;
                let lh = p.style.font_size * 1.25;
                let r = match (kind, ic.pos) {
                    (ShapeKind::Group, _) => {
                        let top = match p.text.title {
                            TitlePos::Top => rect.y + pad.y,
                            TitlePos::Bottom if text.title_band > 0.0 => {
                                rect.bottom() - text.title_band
                            }
                            TitlePos::Bottom => rect.bottom() - pad.y - lh,
                        };
                        Rect::new(rect.x + pad.x, top + (lh - s) / 2.0, s, s)
                    }
                    (_, IconPos::Left) => {
                        Rect::new(rect.x + pad.x, rect.center().y - s / 2.0, s, s)
                    }
                    (_, IconPos::Top) => Rect::new(rect.center().x - s / 2.0, rect.y + pad.y, s, s),
                };
                ImageRef {
                    asset: ic.asset,
                    rect: r,
                    fit: Fit::Contain,
                }
            }),
            label: n.label.clone(),
            rect,
            parent,
            depth,
            layer: p.layer,
            style: p.style.clone(),
            text_area: text.area,
            text_below: svg.is_some_and(|s| s.below),
            text: text.items,
            title_band: text.title_band,
            title: p.text.title,
            allow: p.allow.clone(),
            reason: p.reason.clone(),
            origin,
            span: n.span,
        })
    }

    fn make_image(&mut self, idx: usize, i: &ast::Image) -> Option<Shape> {
        let rect = self.place(idx)?;
        let origin = self.frame_origin(idx).unwrap_or_default();
        let p = self.entries[idx].node()?;
        let props = p.image?;
        Some(Shape {
            id: i.id.name.clone(),
            kind: ShapeKind::Image,
            outline: Outline::Rect,
            form: None,
            shape: "image".into(),
            image: props.asset.map(|asset| ImageRef {
                asset,
                rect,
                fit: props.fit,
            }),
            icon: None,
            label: None,
            rect,
            parent: None,
            depth: 0,
            layer: p.layer,
            style: p.style.clone(),
            text_area: rect,
            text_below: false,
            text: Vec::new(),
            title_band: 0.0,
            title: TitlePos::Top,
            allow: p.allow.clone(),
            reason: p.reason.clone(),
            origin,
            span: i.span,
        })
    }

    pub(super) fn name(&self, idx: usize) -> String {
        let item = self.entries[idx].item;
        match (item.id(), item) {
            (Some(id), _) => id.name.clone(),
            (None, Item::Text(t)) => unnamed_text_id(t.span.line),
            (None, _) => format!("line@L{}", item.span().line),
        }
    }
}
