use std::path::Path;

use kashika_geom::{Pt, convex_hull};

use crate::icons;
use crate::loader::{FileError, Found, Loader};
use crate::scene::Asset;

const MAX_BYTES: u64 = 16 * 1024 * 1024;

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
        self.load_found(loader.find(from, name)?, name)
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
        self.load_found(found, name)
    }

    fn load_found(&mut self, found: Found, name: &str) -> Result<usize, FileError> {
        let key = match &found {
            Found::File(p) => p.to_string_lossy().into_owned(),
            Found::Bundled(n) => format!("builtin:{n}"),
        };
        if let Some(i) = self.keys.iter().position(|k| *k == key) {
            return Ok(i);
        }
        let (bytes, is_svg, shown) = match found {
            Found::File(p) => {
                let len = std::fs::metadata(&p)
                    .map_err(|e| FileError::new(format!("cannot read `{name}`: {e}")))?
                    .len();
                if len > MAX_BYTES {
                    return Err(FileError::new(format!("`{name}` is larger than 16 MiB")));
                }
                let bytes = std::fs::read(&p)
                    .map_err(|e| FileError::new(format!("cannot read `{name}`: {e}")))?;
                (bytes, is_svg_path(&p), name.to_string())
            }
            Found::Bundled(n) => {
                let (_, src) = icons::bundled(n).expect("the loader only returns bundled names");
                (src.as_bytes().to_vec(), true, key.clone())
            }
        };
        let asset = decode(shown, &bytes, is_svg)
            .map_err(|m| FileError::new(format!("cannot use `{name}`: {m}")))?;
        self.keys.push(key);
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
    decode(String::new(), bytes, is_svg).map(|a| a.natural)
}

fn decode(path: String, bytes: &[u8], is_svg: bool) -> Result<Asset, String> {
    if is_svg {
        svg(path, bytes)
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
    })
}

/// Parses with usvg and re-serializes. The round trip is the sanitizer: scripts, event
/// handlers, `foreignObject` and external references don't survive it, and the href
/// resolver refuses everything but `data:` URIs.
fn svg(path: String, bytes: &[u8]) -> Result<Asset, String> {
    let opt = usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(bytes, &opt).map_err(|e| format!("not a valid SVG: {e}"))?;
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
    })
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
        let a = decode("evil.svg".into(), EVIL.as_bytes(), true).unwrap();
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
    }

    #[test]
    fn svg_hull_is_in_unit_coordinates() {
        let src = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><path d="M 10 0 L 30 0 L 40 20 L 0 20 Z"/></svg>"##;
        let a = decode("t.svg".into(), src.as_bytes(), true).unwrap();
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
    fn raster_sizes_come_from_the_header() {
        let a = decode("x.png".into(), &png(200, 100), false).unwrap();
        assert_eq!((a.mime, a.natural), ("image/png", (200.0, 100.0)));
        assert_eq!(a.bytes, png(200, 100));
        assert_eq!(a.unit_hull.len(), 4);
        let e = decode("x.gif".into(), b"GIF89a\x01\x00\x01\x00", false).unwrap_err();
        assert!(e.contains("only SVG, PNG and JPEG"), "{e}");
        assert!(decode("bad.svg".into(), b"<svg", true).is_err());
    }

    #[test]
    fn bundled_icons_decode() {
        for (name, src) in BUNDLED_ICONS {
            let a = decode(format!("builtin:{name}"), src.as_bytes(), true).unwrap();
            assert_eq!(a.natural, (24.0, 24.0), "{name}");
            assert!(a.unit_hull.len() >= 3, "{name}");
        }
    }

    #[test]
    fn oversized_images_are_refused_from_the_header() {
        let e = decode("x.png".into(), &png(100_000, 10), false).unwrap_err();
        assert!(e.contains("100000x10") && e.contains("too large"), "{e}");
        let e = decode("x.png".into(), &png(8_000, 8_000), false).unwrap_err();
        assert!(e.contains("too large"), "{e}");
        assert!(decode("x.png".into(), &png(8_192, 3_906), false).is_ok());
    }

    #[test]
    fn oversized_images_nested_in_svg_are_refused() {
        let b64 = |b: &[u8]| {
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
        };
        let mut src = String::new();
        write!(
            src,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image width="10" height="10" href="data:image/png;base64,{}"/></svg>"#,
            b64(&png(100_000, 10))
        )
        .unwrap();
        let e = decode("n.svg".into(), src.as_bytes(), true).unwrap_err();
        assert!(e.contains("too large"), "{e}");
    }

    #[test]
    fn probe_reports_natural_size() {
        assert_eq!(probe(&png(3, 7), false), Ok((3.0, 7.0)));
    }
}
