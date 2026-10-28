use base64::Engine;
use kashika_geom::PathSeg;
use kashika_model::{Asset, Fit, ImageRef, Shape, ShapeKind};

use crate::svg::{n, path_data, stroke_attrs};

/// Draws a shape's body (its image, path form or rect), then its icon.
pub(crate) fn body(out: &mut String, s: &Shape, assets: &[Asset]) {
    let st = &s.style;
    if let Some(img) = &s.image {
        image(out, img, assets);
    } else if let Some(f) = &s.form {
        let d: Vec<String> = f
            .draw
            .iter()
            .filter(|sp| !sp.is_empty())
            .map(|sp| subpath(sp))
            .collect();
        if !d.is_empty() {
            out.push_str(&format!(
                "<path d=\"{}\" fill=\"{}\" stroke-linejoin=\"round\" {}/>\n",
                d.join(" "),
                st.fill,
                stroke_attrs(st)
            ));
        }
    } else if s.kind != ShapeKind::Image {
        let r = s.rect;
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" {}/>\n",
            n(r.x),
            n(r.y),
            n(r.w),
            n(r.h),
            n(s.corner_radius()),
            st.fill,
            stroke_attrs(st)
        ));
    }
    if let Some(icon) = &s.icon {
        image(out, icon, assets);
    }
}

fn subpath(sp: &[PathSeg]) -> String {
    let mut d = path_data(sp);
    if let (Some(a), Some(b)) = (sp.first(), sp.last()) {
        if a.from().dist(b.to()) <= 1e-9 {
            d.push_str(" Z");
        }
    }
    d
}

fn image(out: &mut String, img: &ImageRef, assets: &[Asset]) {
    let Some(a) = assets.get(img.asset) else {
        return;
    };
    let r = img.rect;
    let fit = match img.fit {
        Fit::Contain => "xMidYMid meet",
        Fit::Cover => "xMidYMid slice",
        Fit::Stretch => "none",
    };
    let data = base64::engine::general_purpose::STANDARD.encode(&a.bytes);
    out.push_str(&format!(
        "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"{fit}\" href=\"data:{};base64,{data}\"/>\n",
        n(r.x),
        n(r.y),
        n(r.w),
        n(r.h),
        a.mime
    ));
}
