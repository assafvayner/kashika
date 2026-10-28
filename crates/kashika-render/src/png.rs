use resvg::{tiny_skia, usvg};

fn rasterize(svg: &str) -> Result<tiny_skia::Pixmap, String> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let size = tree.size().to_int_size();
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("the canvas is empty")?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    Ok(pixmap)
}

/// Rasterizes an SVG document at 1 px per diagram unit. Kashika draws text as glyph paths,
/// so no fonts are loaded and the bytes are the same on every machine.
pub fn svg_to_png(svg: &str) -> Result<Vec<u8>, String> {
    rasterize(svg)?.encode_png().map_err(|e| e.to_string())
}

const JPEG_QUALITY: u8 = 90;

/// Rasterizes like [`svg_to_png`], composited onto white because JPEG has no alpha.
pub fn svg_to_jpeg(svg: &str) -> Result<Vec<u8>, String> {
    let pixmap = rasterize(svg)?;
    let (w, h) = (pixmap.width(), pixmap.height());
    let too_big = || "the canvas is too large for JPEG".to_string();
    let (jw, jh) = (
        u16::try_from(w).map_err(|_| too_big())?,
        u16::try_from(h).map_err(|_| too_big())?,
    );
    let rgb: Vec<u8> = pixmap
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, a]| {
            let inv = 255 - u16::from(a);
            [r, g, b].map(|c| (u16::from(c) + inv).min(255) as u8)
        })
        .collect();
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, JPEG_QUALITY)
        .encode(&rgb, jw, jh, jpeg_encoder::ColorType::Rgb)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

/// Rasterizes like [`svg_to_png`] and encodes lossless WebP.
pub fn svg_to_webp(svg: &str) -> Result<Vec<u8>, String> {
    let pixmap = rasterize(svg)?;
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect::<Vec<u8>>();
    let mut out = Vec::new();
    image_webp::WebPEncoder::new(&mut out)
        .encode(
            &rgba,
            pixmap.width(),
            pixmap.height(),
            image_webp::ColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
    Ok(out)
}
