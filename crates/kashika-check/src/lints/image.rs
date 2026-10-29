use kashika_diag::Diagnostic;
use kashika_geom::{fmt_num, round2};
use kashika_model::ShapeKind;

use crate::ctx::Ctx;
use crate::element::shape_el;
use crate::sink::Sink;

const MAX_DISTORTION: f64 = 0.05;

/// `image-distort`: `fit=stretch` changes the aspect ratio of an image or SVG-file shape by
/// more than 5%.
pub fn image_distort(ctx: &Ctx, out: &mut Sink) {
    let assets = &ctx.scene.assets;
    for s in ctx
        .scene
        .shapes
        .iter()
        .filter(|s| s.kind != ShapeKind::Group)
    {
        let (Some(d), Some(img)) = (s.image_distortion(assets), s.image.as_ref()) else {
            continue;
        };
        if d <= MAX_DISTORTION {
            continue;
        }
        let (nw, nh) = assets[img.asset].natural;
        let r = img.rect;
        out.push(
            Diagnostic::warn(
                "image-distort",
                s.span,
                format!(
                    "{} '{}' is stretched {}% away from its {}x{} aspect ratio",
                    s.kind.name(),
                    s.id,
                    (d * 100.0).round(),
                    fmt_num(nw),
                    fmt_num(nh)
                ),
            )
            .with_element(shape_el(s))
            .with_fact("natural", [round2(nw), round2(nh)])
            .with_fact("size", [round2(r.w), round2(r.h)])
            .with_fact("distortion", round2(d))
            .with_help(format!(
                "use fit=contain or fit=cover{}, or size ({}, {}) to keep the ratio",
                if s.kind == ShapeKind::Image {
                    String::new()
                } else {
                    format!(" on shape `{}`", s.shape)
                },
                fmt_num(r.w),
                fmt_num((r.w * nh / nw).round())
            )),
        );
    }
}

/// `svg-dropped`: an SVG file lost elements when it was embedded (`<image>`s that are not
/// `data:` URIs, `foreignObject`). Reported once per file, at its first use.
pub fn svg_dropped(ctx: &Ctx, out: &mut Sink) {
    let assets = &ctx.scene.assets;
    let mut seen = Vec::new();
    for s in &ctx.scene.shapes {
        for img in [&s.image, &s.icon].into_iter().flatten() {
            let a = &assets[img.asset];
            if a.dropped.is_empty() || seen.contains(&img.asset) {
                continue;
            }
            seen.push(img.asset);
            let lost = a
                .dropped
                .iter()
                .map(|&(name, n)| {
                    format!("{n} `<{name}>` element{}", if n == 1 { "" } else { "s" })
                })
                .collect::<Vec<_>>()
                .join(" and ");
            out.push(
                Diagnostic::warn(
                    "svg-dropped",
                    s.span,
                    format!("'{}' drops {lost} when embedded", a.path),
                )
                .with_element(shape_el(s))
                .with_fact("file", a.path.clone())
                .with_fact(
                    "dropped",
                    a.dropped
                        .iter()
                        .map(|&(name, n)| (name.to_string(), n))
                        .collect::<std::collections::BTreeMap<_, _>>(),
                )
                .with_help("embed it as a path, or use a separate `image`"),
            );
        }
    }
}
