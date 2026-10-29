use std::collections::{BTreeMap, BTreeSet};

use base64::Engine;
use kashika_geom::PathSeg;
use kashika_model::{Asset, Fit, ImageRef, Scene, Shape, ShapeKind};

use crate::svg::{n, path_data, stroke_attrs};

fn aspect(fit: Fit) -> &'static str {
    match fit {
        Fit::Contain => "xMidYMid meet",
        Fit::Cover => "xMidYMid slice",
        Fit::Stretch => "none",
    }
}

fn fit_name(fit: Fit) -> &'static str {
    match fit {
        Fit::Contain => "contain",
        Fit::Cover => "cover",
        Fit::Stretch => "stretch",
    }
}

fn data_uri(a: &Asset) -> String {
    let data = base64::engine::general_purpose::STANDARD.encode(&a.bytes);
    format!("data:{};base64,{data}", a.mime)
}

/// Assets drawn more than once are embedded a single time in `<defs>`, with one `<symbol>`
/// per fit used, and drawn with `<use>`.
pub(crate) struct Defs<'a> {
    id: String,
    assets: &'a [Asset],
    shared: BTreeMap<usize, BTreeSet<&'static str>>,
}

impl<'a> Defs<'a> {
    pub(crate) fn new(scene: &'a Scene, id: String) -> Self {
        let mut uses: BTreeMap<usize, Vec<Fit>> = BTreeMap::new();
        for s in &scene.shapes {
            for img in s.image.iter().chain(s.icon.iter()) {
                uses.entry(img.asset).or_default().push(img.fit);
            }
        }
        let shared = uses
            .into_iter()
            .filter(|(i, v)| {
                v.len() > 1
                    && scene
                        .assets
                        .get(*i)
                        .is_some_and(|a| a.natural.0 > 0.0 && a.natural.1 > 0.0)
            })
            .map(|(i, v)| (i, v.into_iter().map(fit_name).collect()))
            .collect();
        Defs {
            id,
            assets: &scene.assets,
            shared,
        }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    fn symbol_id(&self, asset: usize, fit: Fit) -> String {
        format!("kashika-{}-asset-{asset}-{}", self.id, fit_name(fit))
    }

    pub(crate) fn write(&self, out: &mut String) {
        if self.shared.is_empty() {
            return;
        }
        out.push_str("<defs>\n");
        for (&i, fits) in &self.shared {
            let a = &self.assets[i];
            let (w, h) = (n(a.natural.0), n(a.natural.1));
            out.push_str(&format!(
                "<image id=\"kashika-{}-asset-{i}\" width=\"{w}\" height=\"{h}\" href=\"{}\"/>\n",
                self.id,
                data_uri(a)
            ));
            for fit in [Fit::Contain, Fit::Cover, Fit::Stretch] {
                if fits.contains(fit_name(fit)) {
                    out.push_str(&format!(
                        "<symbol id=\"{}\" viewBox=\"0 0 {w} {h}\" preserveAspectRatio=\"{}\"><use href=\"#kashika-{}-asset-{i}\"/></symbol>\n",
                        self.symbol_id(i, fit),
                        aspect(fit),
                        self.id
                    ));
                }
            }
        }
        out.push_str("</defs>\n");
    }
}

/// Draws a shape's body (its image, path form or rect), then its icon.
pub(crate) fn body(out: &mut String, s: &Shape, defs: &Defs) {
    let st = &s.style;
    if let Some(img) = &s.image {
        image(out, img, defs);
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
        image(out, icon, defs);
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

fn image(out: &mut String, img: &ImageRef, defs: &Defs) {
    let Some(a) = defs.assets.get(img.asset) else {
        return;
    };
    let r = img.rect;
    let (x, y, w, h) = (n(r.x), n(r.y), n(r.w), n(r.h));
    if defs.shared.contains_key(&img.asset) {
        out.push_str(&format!(
            "<use href=\"#{}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\"/>\n",
            defs.symbol_id(img.asset, img.fit)
        ));
        return;
    }
    out.push_str(&format!(
        "<image x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" preserveAspectRatio=\"{}\" href=\"{}\"/>\n",
        aspect(img.fit),
        data_uri(a)
    ));
}
