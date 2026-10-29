use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use kashika_geom::{Pt, convex_hull};

use crate::loader::{FileError, Found, Loader};
use crate::scene::Asset;
use crate::style::ICON_COLOR;

/// Largest image or SVG file read.
const MAX_BYTES: u64 = 16 * 1024 * 1024;
/// Deepest element nesting accepted in an SVG, judged from the source before parsing.
const MAX_SVG_DEPTH: usize = 256;

/// Raster output (PNG, JPEG, WebP) is 1 px per diagram unit; a canvas beyond either limit is refused.
pub const MAX_RASTER_SIDE: u32 = 16_384;
pub const MAX_RASTER_PIXELS: u64 = 100_000_000;
/// Attached PNG/JPEG images, judged by their header before any pixel is decoded.
pub const MAX_IMAGE_SIDE: u32 = 8_192;
pub const MAX_IMAGE_PIXELS: u64 = 32_000_000;

fn check_image_size(width: usize, height: usize) -> Result<(), String> {
    let (w, h) = (width as u64, height as u64);
    if w > u64::from(MAX_IMAGE_SIDE) || h > u64::from(MAX_IMAGE_SIDE) || w * h > MAX_IMAGE_PIXELS {
        return Err(format!(
            "the {w}x{h} image is too large (at most {MAX_IMAGE_SIDE} px per side and {MAX_IMAGE_PIXELS} px in total)"
        ));
    }
    Ok(())
}

/// Loaded assets, deduplicated by canonical path, in first-use order.
#[derive(Default)]
pub(crate) struct Store {
    assets: Vec<Asset>,
    keys: Vec<String>,
}

impl Store {
    /// Finds `name` from the directory `from`, loads it once and returns its index.
    pub fn load(&mut self, loader: &Loader, from: &Path, name: &str) -> Result<usize, FileError> {
        let found = loader.find(from, name)?;
        self.load_found(loader, found, name, None)
    }

    /// Like `load`, for an icon: an SVG that uses `currentColor` and sets no `color` on its root
    /// is drawn in `color`, and is kept apart from the same file loaded in another color.
    pub fn load_icon(
        &mut self,
        loader: &Loader,
        from: &Path,
        name: &str,
        color: &str,
    ) -> Result<usize, FileError> {
        let found = loader.find(from, name)?;
        self.load_found(loader, found, name, Some(color))
    }

    /// Like `load`, but only accepts SVG files; anything else is refused before it is read.
    pub fn load_svg(
        &mut self,
        loader: &Loader,
        from: &Path,
        name: &str,
        what: &str,
    ) -> Result<usize, FileError> {
        let found = loader.find(from, name)?;
        if let Found::File(p) = &found {
            if !is_svg_path(p) {
                return Err(FileError::new(format!(
                    "{what} needs an SVG file; `{name}` is not one"
                )));
            }
        }
        self.load_found(loader, found, name, None)
    }

    fn load_found(
        &mut self,
        loader: &Loader,
        found: Found,
        name: &str,
        color: Option<&str>,
    ) -> Result<usize, FileError> {
        let (key, color) = match &found {
            Found::File(p) => (p.to_string_lossy().into_owned(), color),
            Found::Bundled(n) => (format!("builtin:{n}"), color.or(Some(ICON_COLOR))),
        };
        let colored = format!("{key}\u{0}{}", color.unwrap_or_default());
        if let Some(i) = self.keys.iter().position(|k| *k == key || *k == colored) {
            return Ok(i);
        }
        let bytes = loader.read(&found, name, MAX_BYTES)?;
        let (is_svg, shown) = match &found {
            Found::File(p) => (is_svg_path(p), name.to_string()),
            Found::Bundled(_) => (true, key.clone()),
        };
        let asset = decode(shown, &bytes, is_svg, color)
            .map_err(|m| FileError::new(format!("cannot use `{name}`: {m}")))?;
        let recolorable = is_svg && uses_current_color(&bytes);
        self.keys.push(if recolorable { colored } else { key });
        self.assets.push(asset);
        Ok(self.assets.len() - 1)
    }

    pub fn get(&self, i: usize) -> &Asset {
        &self.assets[i]
    }

    pub fn into_vec(self) -> Vec<Asset> {
        self.assets
    }
}

fn is_svg_path(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"))
}

/// The natural size of an image file's contents, for `kashika icons`.
pub fn probe(bytes: &[u8], is_svg: bool) -> Result<(f64, f64), String> {
    decode(String::new(), bytes, is_svg, None).map(|a| a.natural)
}

fn decode(path: String, bytes: &[u8], is_svg: bool, color: Option<&str>) -> Result<Asset, String> {
    if is_svg {
        svg(path, bytes, color)
    } else {
        raster(path, bytes)
    }
}

fn unit_square() -> Vec<Pt> {
    vec![
        Pt::new(0.0, 0.0),
        Pt::new(1.0, 0.0),
        Pt::new(1.0, 1.0),
        Pt::new(0.0, 1.0),
    ]
}

fn raster(path: String, bytes: &[u8]) -> Result<Asset, String> {
    let mime = match imagesize::image_type(bytes) {
        Ok(imagesize::ImageType::Png) => "image/png",
        Ok(imagesize::ImageType::Jpeg) => "image/jpeg",
        _ => return Err("only SVG, PNG and JPEG images are supported".into()),
    };
    let size =
        imagesize::blob_size(bytes).map_err(|e| format!("cannot read the image size: {e}"))?;
    check_image_size(size.width, size.height)?;
    Ok(Asset {
        path,
        mime,
        natural: (size.width as f64, size.height as f64),
        bytes: bytes.to_vec(),
        unit_hull: unit_square(),
        current_color: None,
        dropped: Vec::new(),
    })
}

/// The bundled fonts, for `<text>` in SVG files. Every generic family resolves to Inter
/// but `monospace`, which is the mono face.
fn fonts() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        for data in kashika_text::font_files() {
            db.load_font_source(usvg::fontdb::Source::Binary(Arc::new(data)));
        }
        let family = |mono: bool| {
            db.faces()
                .find(|f| f.monospaced == mono)
                .and_then(|f| f.families.first())
                .map(|(name, _)| name.clone())
                .unwrap_or_default()
        };
        let (sans, mono) = (family(false), family(true));
        db.set_serif_family(sans.clone());
        db.set_sans_serif_family(sans.clone());
        db.set_cursive_family(sans.clone());
        db.set_fantasy_family(sans);
        db.set_monospace_family(mono);
        Arc::new(db)
    })
    .clone()
}

/// Parses with usvg and re-serializes. The round trip is the sanitizer: scripts, event
/// handlers, `foreignObject` and external references don't survive it, and the href
/// resolver refuses everything but `data:` URIs. `<text>` is set in the bundled fonts and
/// written as paths. With `color`, an SVG that uses `currentColor` and sets no `color` on
/// its root resolves it to `color`.
fn svg(path: String, bytes: &[u8], color: Option<&str>) -> Result<Asset, String> {
    check_depth(bytes)?;
    let current_color = color.filter(|_| uses_current_color(bytes));
    let too_deep = AtomicBool::new(false);
    let data = usvg::ImageHrefResolver::default_data_resolver();
    let fontdb = fonts();
    let opt = usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|mime, bytes, opts| {
                let svg = mime == "image/svg+xml"
                    || (mime == "text/plain" && imagesize::image_type(&bytes).is_err());
                if svg && check_depth(&bytes).is_err() {
                    too_deep.store(true, Ordering::Relaxed);
                    return None;
                }
                data(mime, bytes, opts)
            }),
            resolve_string: Box::new(|_, _| None),
        },
        style_sheet: current_color.map(|c| format!("svg {{ color: {c} }}")),
        font_family: fontdb
            .family_name(&usvg::fontdb::Family::SansSerif)
            .to_string(),
        fontdb,
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(bytes, &opt).map_err(|e| format!("not a valid SVG: {e}"))?;
    if too_deep.load(Ordering::Relaxed) {
        return Err(format!(
            "an embedded SVG nests elements more than {MAX_SVG_DEPTH} deep"
        ));
    }
    check_nested_images(tree.root())?;
    let size = tree.size();
    let (w, h) = (f64::from(size.width()), f64::from(size.height()));
    let mut pts = Vec::new();
    collect_points(tree.root(), &mut pts);
    let unit: Vec<Pt> = pts.iter().map(|p| Pt::new(p.x / w, p.y / h)).collect();
    let hull = convex_hull(&unit);
    Ok(Asset {
        path,
        mime: "image/svg+xml",
        natural: (w, h),
        bytes: tree.to_string(&usvg::WriteOptions::default()).into_bytes(),
        unit_hull: if hull.len() >= 3 { hull } else { unit_square() },
        current_color: current_color.map(str::to_string),
        dropped: dropped(bytes, &tree),
    })
}

/// The elements of the SVG source that its usvg tree lost, as (element name, count):
/// `<image>`s that reference anything but a `data:` URI or fail to decode, and
/// `foreignObject`s.
fn dropped(bytes: &[u8], tree: &usvg::Tree) -> Vec<(&'static str, usize)> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Vec::new();
    };
    let Ok(doc) = usvg::roxmltree::Document::parse_with_options(
        text,
        usvg::roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    ) else {
        return Vec::new();
    };
    let count = |name: &str| {
        doc.descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == name)
            .count()
    };
    [
        (
            "image",
            count("image").saturating_sub(count_images(tree.root())),
        ),
        ("foreignObject", count("foreignObject")),
    ]
    .into_iter()
    .filter(|&(_, n)| n > 0)
    .collect()
}

/// The image nodes of a usvg tree: in groups, masks and pattern fills or strokes, but not
/// inside embedded SVG images.
fn count_images(g: &usvg::Group) -> usize {
    let mut n = g.mask().map_or(0, |m| count_images(m.root()));
    for node in g.children() {
        n += match node {
            usvg::Node::Group(g) => count_images(g),
            usvg::Node::Image(_) => 1,
            usvg::Node::Path(p) => [p.fill().map(|f| f.paint()), p.stroke().map(|s| s.paint())]
                .into_iter()
                .flatten()
                .map(|paint| match paint {
                    usvg::Paint::Pattern(pat) => count_images(pat.root()),
                    _ => 0,
                })
                .sum(),
            usvg::Node::Text(_) => 0,
        };
    }
    n
}

/// Whether an SVG paints with `currentColor` and leaves its root's `color` unset.
fn uses_current_color(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    if !text.to_ascii_lowercase().contains("currentcolor") {
        return false;
    }
    usvg::roxmltree::Document::parse_with_options(
        text,
        usvg::roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .is_ok_and(|d| d.root_element().attribute("color").is_none())
}

/// Refuses markup nested more than `MAX_SVG_DEPTH` deep before the recursive parsers see
/// it. Comments, CDATA and processing instructions are skipped; declarations (`<!...>`)
/// are scanned through, so element markup in entity values counts too.
fn check_depth(bytes: &[u8]) -> Result<(), String> {
    let after = |from: usize, pat: &[u8]| {
        bytes
            .get(from..)
            .and_then(|rest| rest.windows(pat.len()).position(|w| w == pat))
            .map_or(bytes.len(), |i| from + i + pat.len())
    };
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &bytes[i..];
        if rest.starts_with(b"<!--") {
            i = after(i + 4, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            i = after(i + 9, b"]]>");
        } else if rest.starts_with(b"<?") {
            i = after(i + 2, b"?>");
        } else if rest.starts_with(b"<!") {
            i += 2;
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            i = after(i + 2, b">");
        } else {
            let mut j = i + 1;
            let mut quote = None;
            while j < bytes.len() {
                match (quote, bytes[j]) {
                    (None, b'"' | b'\'') => quote = Some(bytes[j]),
                    (Some(q), c) if c == q => quote = None,
                    (None, b'>') => break,
                    _ => {}
                }
                j += 1;
            }
            if bytes[j - 1] != b'/' {
                depth += 1;
                if depth > MAX_SVG_DEPTH {
                    return Err(format!(
                        "elements are nested more than {MAX_SVG_DEPTH} deep"
                    ));
                }
            }
            i = j + 1;
        }
    }
    Ok(())
}

/// Raster images embedded in an SVG are held to the same limit as attached ones, wherever
/// they sit: groups, nested SVG images and pattern fills or strokes.
fn check_nested_images(g: &usvg::Group) -> Result<(), String> {
    for node in g.children() {
        match node {
            usvg::Node::Group(g) => check_nested_images(g)?,
            usvg::Node::Image(img) => match img.kind() {
                usvg::ImageKind::SVG(t) => check_nested_images(t.root())?,
                usvg::ImageKind::JPEG(d)
                | usvg::ImageKind::PNG(d)
                | usvg::ImageKind::GIF(d)
                | usvg::ImageKind::WEBP(d) => {
                    let size = imagesize::blob_size(d)
                        .map_err(|e| format!("cannot read an embedded image's size: {e}"))?;
                    check_image_size(size.width, size.height)
                        .map_err(|m| format!("an embedded image: {m}"))?;
                }
            },
            usvg::Node::Path(p) => {
                let paints = [p.fill().map(|f| f.paint()), p.stroke().map(|s| s.paint())];
                for paint in paints.into_iter().flatten() {
                    if let usvg::Paint::Pattern(pat) = paint {
                        check_nested_images(pat.root())?;
                    }
                }
            }
            usvg::Node::Text(_) => {}
        }
    }
    Ok(())
}

/// Every path point in canvas coordinates of the SVG, control points included.
fn collect_points(g: &usvg::Group, out: &mut Vec<Pt>) {
    for node in g.children() {
        match node {
            usvg::Node::Group(g) => collect_points(g, out),
            usvg::Node::Path(p) => {
                let ts = p.abs_transform();
                for pt in p.data().points() {
                    let mut q = *pt;
                    ts.map_point(&mut q);
                    out.push(Pt::new(f64::from(q.x), f64::from(q.y)));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;
    use crate::icons::BUNDLED_ICONS;

    const EVIL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="10" height="10" onload="alert(1)"><script>alert(2)</script><foreignObject width="10" height="10"><div xmlns="http://www.w3.org/1999/xhtml">x</div></foreignObject><image width="5" height="5" xlink:href="https://example.com/x.png"/><image width="5" height="5" href="secret.png"/><rect width="10" height="10" fill="#f00"/></svg>"##;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut b = vec![
            0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
        ];
        b.extend(w.to_be_bytes());
        b.extend(h.to_be_bytes());
        b.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
        b
    }

    #[test]
    fn svg_is_sanitized() {
        let a = decode("evil.svg".into(), EVIL.as_bytes(), true, None).unwrap();
        let out = String::from_utf8(a.bytes).unwrap();
        for bad in [
            "script",
            "alert",
            "foreignObject",
            "example.com",
            "secret.png",
            "onload",
        ] {
            assert!(!out.contains(bad), "`{bad}` survived: {out}");
        }
        assert!(out.contains("<path"), "{out}");
        assert_eq!(a.mime, "image/svg+xml");
        assert_eq!(a.natural, (10.0, 10.0));
        assert_eq!(a.dropped, [("image", 2), ("foreignObject", 1)]);
    }

    #[test]
    fn svg_text_is_drawn_as_paths_in_the_bundled_fonts() {
        let text = |family: &str| {
            let src = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="20"><text x="2" y="15" font-size="12"{family}>Hello</text></svg>"#
            );
            let a = decode("t.svg".into(), src.as_bytes(), true, None).unwrap();
            assert!(a.dropped.is_empty());
            String::from_utf8(a.bytes).unwrap()
        };
        let inter = text("");
        assert!(!inter.contains("<text"), "{inter}");
        assert!(inter.contains("<path"), "{inter}");
        assert_eq!(inter, text(r#" font-family="Inter""#));
        assert_eq!(inter, text(r#" font-family="Helvetica, sans-serif""#));
        assert_eq!(inter, text(r#" font-family="Times""#));
        let mono = text(r#" font-family="monospace""#);
        assert_ne!(inter, mono);
        assert_eq!(mono, text(r#" font-family="JetBrains Mono""#));
        assert_eq!(mono, text(r#" font-family="Courier New, monospace""#));
    }

    #[test]
    fn svg_hull_is_in_unit_coordinates() {
        let src = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><path d="M 10 0 L 30 0 L 40 20 L 0 20 Z"/></svg>"##;
        let a = decode("t.svg".into(), src.as_bytes(), true, None).unwrap();
        assert_eq!(a.natural, (40.0, 20.0));
        let mut h = a.unit_hull.clone();
        h.sort_by(|p, q| p.x.total_cmp(&q.x).then(p.y.total_cmp(&q.y)));
        assert_eq!(
            h,
            vec![
                Pt::new(0.0, 1.0),
                Pt::new(0.25, 0.0),
                Pt::new(0.75, 0.0),
                Pt::new(1.0, 1.0)
            ]
        );
    }

    #[test]
    fn current_color_resolves_to_the_given_color() {
        let svg = |root: &str, paint: &str| {
            format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"{root}><rect width="10" height="10" fill="{paint}"/></svg>"##
            )
        };
        let a = decode(
            "c.svg".into(),
            svg("", "currentColor").as_bytes(),
            true,
            Some("#ffffff"),
        )
        .unwrap();
        assert_eq!(a.current_color.as_deref(), Some("#ffffff"));
        let out = String::from_utf8(a.bytes).unwrap();
        assert!(out.contains("fill=\"#ffffff\""), "{out}");
        for (root, paint) in [("", "#123456"), (" color=\"#123456\"", "currentColor")] {
            let a = decode(
                "c.svg".into(),
                svg(root, paint).as_bytes(),
                true,
                Some("#ffffff"),
            )
            .unwrap();
            assert_eq!(a.current_color, None, "{root} {paint}");
            assert!(String::from_utf8(a.bytes).unwrap().contains("#123456"));
        }
        let a = decode(
            "c.svg".into(),
            svg("", "currentColor").as_bytes(),
            true,
            None,
        )
        .unwrap();
        assert_eq!(a.current_color, None);
        assert!(String::from_utf8(a.bytes).unwrap().contains("#000000"));
    }

    #[test]
    fn raster_sizes_come_from_the_header() {
        let a = decode("x.png".into(), &png(200, 100), false, None).unwrap();
        assert_eq!((a.mime, a.natural), ("image/png", (200.0, 100.0)));
        assert_eq!(a.bytes, png(200, 100));
        assert_eq!(a.unit_hull.len(), 4);
        let e = decode("x.gif".into(), b"GIF89a\x01\x00\x01\x00", false, None).unwrap_err();
        assert!(e.contains("only SVG, PNG and JPEG"), "{e}");
        assert!(decode("bad.svg".into(), b"<svg", true, None).is_err());
    }

    #[test]
    fn bundled_icons_decode() {
        for (name, src) in BUNDLED_ICONS {
            let a = decode(format!("builtin:{name}"), src.as_bytes(), true, None).unwrap();
            assert_eq!(a.natural, (24.0, 24.0), "{name}");
            assert!(a.unit_hull.len() >= 3, "{name}");
        }
    }

    #[test]
    fn oversized_images_are_refused_from_the_header() {
        let e = decode("x.png".into(), &png(100_000, 10), false, None).unwrap_err();
        assert!(e.contains("100000x10") && e.contains("too large"), "{e}");
        let e = decode("x.png".into(), &png(8_000, 8_000), false, None).unwrap_err();
        assert!(e.contains("too large"), "{e}");
        assert!(decode("x.png".into(), &png(8_192, 3_906), false, None).is_ok());
    }

    fn b64(b: &[u8]) -> String {
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut o = String::new();
        for c in b.chunks(3) {
            let n = (u32::from(c[0]) << 16)
                | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
                | u32::from(*c.get(2).unwrap_or(&0));
            for i in 0..4 {
                if i <= c.len() {
                    o.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
                } else {
                    o.push('=');
                }
            }
        }
        o
    }

    fn nested(depth: usize) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">{}<rect width="10" height="10"/>{}</svg>"#,
            "<g>".repeat(depth - 1),
            "</g>".repeat(depth - 1)
        )
    }

    #[test]
    fn deep_nesting_is_refused_before_parsing() {
        let e = decode("d.svg".into(), nested(20_000).as_bytes(), true, None).unwrap_err();
        assert!(e.contains("nested more than 256 deep"), "{e}");
        assert!(decode("d.svg".into(), nested(256).as_bytes(), true, None).is_ok());
        let e = decode("d.svg".into(), nested(257).as_bytes(), true, None).unwrap_err();
        assert!(e.contains("nested more than 256 deep"), "{e}");
        let flat = format!(
            r#"<?xml version="1.0"?><!-- <g><g> --><svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">{}<![CDATA[<g>]]></svg>"#,
            "<g a='>'><rect width=\"1\" height=\"1\"/></g>".repeat(300)
        );
        assert!(check_depth(flat.as_bytes()).is_ok());
    }

    #[test]
    fn deep_nesting_in_an_embedded_svg_is_refused() {
        let src = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image width="10" height="10" href="data:image/svg+xml;base64,{}"/></svg>"#,
            b64(nested(20_000).as_bytes())
        );
        let e = decode("n.svg".into(), src.as_bytes(), true, None).unwrap_err();
        assert!(
            e.contains("embedded SVG nests elements more than 256 deep"),
            "{e}"
        );
    }

    #[test]
    fn oversized_images_nested_in_svg_are_refused() {
        let mut src = String::new();
        write!(
            src,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image width="10" height="10" href="data:image/png;base64,{}"/></svg>"#,
            b64(&png(100_000, 10))
        )
        .unwrap();
        let e = decode("n.svg".into(), src.as_bytes(), true, None).unwrap_err();
        assert!(e.contains("too large"), "{e}");
    }

    #[test]
    fn probe_reports_natural_size() {
        assert_eq!(probe(&png(3, 7), false), Ok((3.0, 7.0)));
    }
}
