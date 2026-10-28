use resvg::{tiny_skia, usvg};

use kashika_model::{MAX_RASTER_PIXELS, MAX_RASTER_SIDE};

fn check_raster_size(width: u32, height: u32) -> Result<(), String> {
    if width > MAX_RASTER_SIDE
        || height > MAX_RASTER_SIDE
        || u64::from(width) * u64::from(height) > MAX_RASTER_PIXELS
    {
        return Err(format!(
            "the {width}x{height} canvas is too large for raster output (at most {MAX_RASTER_SIDE} px per side and {MAX_RASTER_PIXELS} px in total); use .svg output or a smaller `size`"
        ));
    }
    Ok(())
}

fn rasterize(svg: &str) -> Result<tiny_skia::Pixmap, String> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let size = tree.size().to_int_size();
    check_raster_size(size.width(), size.height())?;
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
    let (jw, jh) = (w as u16, h as u16);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_cap() {
        assert!(check_raster_size(MAX_RASTER_SIDE, 6103).is_ok());
        assert!(check_raster_size(10_000, 10_000).is_ok());
        assert!(check_raster_size(1, 1).is_ok());
    }

    #[test]
    fn rejects_an_oversized_side() {
        let e = check_raster_size(MAX_RASTER_SIDE + 1, 10).unwrap_err();
        assert!(e.contains("16384") && e.contains("svg"), "{e}");
        assert!(check_raster_size(10, MAX_RASTER_SIDE + 1).is_err());
    }

    #[test]
    fn rejects_an_oversized_area() {
        assert!(check_raster_size(MAX_RASTER_SIDE, MAX_RASTER_SIDE).is_err());
        assert!(check_raster_size(10_001, 10_000).is_err());
    }

    #[test]
    fn rasterize_rejects_before_allocating() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000000" height="1000000"/>"#;
        assert!(svg_to_png(svg).unwrap_err().contains("too large"));
    }
}
