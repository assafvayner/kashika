use kashika_diag::Diagnostic;

use crate::ctx::{fmt_num, round2, shape_el, Ctx};

const MAX_DISTORTION: f64 = 0.05;

/// `image-distort`: `fit=stretch` changes an image's aspect ratio by more than 5%.
pub fn image_distort(ctx: &Ctx) -> Vec<Diagnostic> {
    let assets = &ctx.scene.assets;
    let mut out = Vec::new();
    for s in ctx.boxes().map(|g| g.shape) {
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
                    "image '{}' is stretched {}% away from its {}x{} aspect ratio",
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
                "use fit=contain or fit=cover, or size ({}, {}) to keep the ratio",
                fmt_num(r.w),
                fmt_num((r.w * nh / nw).round())
            )),
        );
    }
    out
}
