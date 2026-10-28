use resvg::{tiny_skia, usvg};

/// Rasterizes an SVG document at 1 px per diagram unit. Kashika draws text as glyph paths,
/// so no fonts are loaded and the bytes are the same on every machine.
pub fn svg_to_png(svg: &str) -> Result<Vec<u8>, String> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let size = tree.size().to_int_size();
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("the canvas is empty")?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| e.to_string())
}
