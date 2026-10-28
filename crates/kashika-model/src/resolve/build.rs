use kashika_geom::{Pt, Rect};
use kashika_syntax::ast::{self};

use crate::library::LibShape;
use crate::scene::*;
use crate::textlayout::{self, ShapeInput};

use super::{IconPos, Item, Resolver};

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

            let (item, depth) = (entry.item, entry.depth);
            let parent = entry
                .parent
                .and_then(|p| self.entries[p].item.id())
                .map(|i| i.name.clone());
            match item {
                Item::Box(n) => {
                    self.record_edges(idx, &n.at);
                    self.push_shape(scene, idx, n, ShapeKind::Box, parent, depth)
                }
                Item::Group(g) => {
                    self.record_edges(idx, &g.node.at);
                    self.push_shape(scene, idx, &g.node, ShapeKind::Group, parent, depth)
                }
                Item::Image(i) => {
                    self.record_edges(idx, &i.at);
                    self.push_image(scene, idx, i)
                }
                Item::Junction(j) => {
                    self.record_edges(idx, &j.at);
                    if let Some(r) = self.place(idx) {
                        let dot = self.dots.get(&idx).copied().unwrap_or(false);
                        let origin = self.frame_origin(idx).unwrap_or_default();
                        scene.junctions.push(Junction {
                            id: j.id.name.clone(),
                            at: r.origin(),
                            dot,
                            parent,
                            origin,
                            span: j.span,
                        });
                        self.record_point(&j.id.name, &j.at);
                    }
                }
                Item::Text(t) => {
                    let Some(origin) = self.frame_origin(idx) else {
                        continue;
                    };
                    let Some(at) = self.eval_point(&t.at, origin) else {
                        continue;
                    };
                    let p = &self.texts[&idx];
                    let Some(block) =
                        textlayout::measure_at(&t.content, p.size, false, t.span, &mut self.diags)
                    else {
                        continue;
                    };
                    let (lines, rect) = block.place_at(at.x, at.y, textlayout::halign(p.align));
                    scene.texts.push(Text {
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
                    });
                    let owner =
                        t.id.as_ref()
                            .map_or_else(|| format!("text@L{}", t.span.line), |i| i.name.clone());
                    self.record_point(&owner, &t.at);
                }
                Item::Line(l) => {
                    if let Some(line) = self.build_line(idx, l, parent) {
                        scene.lines.push(line);
                    }
                }
            }
        }
        scene.placement = std::mem::take(&mut self.edges);
        scene.literals = std::mem::take(&mut self.literals);
    }

    pub(super) fn push_shape(
        &mut self,
        scene: &mut Scene,
        idx: usize,
        n: &'a ast::Node,
        kind: ShapeKind,
        parent: Option<String>,
        depth: usize,
    ) {
        let Some(rect) = self.place(idx) else { return };
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
        let p = &self.nodes[&idx];
        let shape = p.drawn_shape();
        let form = self.form(&shape, rect);
        let svg = self.svg_shape(&shape);
        let icon = p.icon;
        let icon_inset = icon.map_or((0.0, 0.0), |ic| {
            let d = ic.size + p.style.padding;
            if kind == ShapeKind::Group || ic.pos == IconPos::Left {
                (d, 0.0)
            } else {
                (0.0, d)
            }
        });
        let text = textlayout::layout_shape(
            &ShapeInput {
                id: &n.id.name,
                node: n,
                kind,
                shape: &shape,
                custom_text_area: self.lib.shapes.get(&shape).and_then(LibShape::text_area),
                text_below: svg.is_some_and(|s| s.below),
                icon_inset,
                rect,
                style: &p.style,
                props: &p.text,
                block_at: &block_at,
            },
            &mut self.diags,
        );
        scene.shapes.push(Shape {
            id: n.id.name.clone(),
            kind,
            outline: p.outline,
            form,
            shape,
            image: svg.map(|s| ImageRef {
                asset: s.asset,
                rect,
                fit: Fit::Stretch,
            }),
            icon: icon.map(|ic| {
                let pad = p.style.padding;
                let s = ic.size;
                let lh = p.style.font_size * 1.25;
                let r = match (kind, ic.pos) {
                    (ShapeKind::Group, _) => {
                        Rect::new(rect.x + pad, rect.y + pad + (lh - s) / 2.0, s, s)
                    }
                    (_, IconPos::Left) => Rect::new(rect.x + pad, rect.center().y - s / 2.0, s, s),
                    (_, IconPos::Top) => Rect::new(rect.center().x - s / 2.0, rect.y + pad, s, s),
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
            allow: p.allow.clone(),
            reason: p.reason.clone(),
            origin,
            span: n.span,
        });
        self.record_node(n);
    }

    pub(super) fn push_image(&mut self, scene: &mut Scene, idx: usize, i: &ast::Image) {
        let Some(rect) = self.place(idx) else { return };
        let origin = self.frame_origin(idx).unwrap_or_default();
        let p = &self.nodes[&idx];
        let props = p.image.expect("image props");
        scene.shapes.push(Shape {
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
            allow: p.allow.clone(),
            reason: p.reason.clone(),
            origin,
            span: i.span,
        });
        self.record_point(&i.id.name, &i.at);
        self.literal(&i.id.name, "width", i.size.0, i.span);
        self.literal(&i.id.name, "height", i.size.1, i.span);
    }

    pub(super) fn name(&self, idx: usize) -> &str {
        self.entries[idx]
            .item
            .id()
            .map_or("?", |id| id.name.as_str())
    }
}
