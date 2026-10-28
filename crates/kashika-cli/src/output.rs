use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Svg,
    Png,
    Jpeg,
    Webp,
}

pub const SUPPORTED: &str = ".svg, .png, .jpg, .jpeg or .webp";

pub fn kind(path: &Path) -> Option<Kind> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("svg") => Some(Kind::Svg),
        Some("png") => Some(Kind::Png),
        Some("jpg" | "jpeg") => Some(Kind::Jpeg),
        Some("webp") => Some(Kind::Webp),
        _ => None,
    }
}

pub fn encode(kind: Kind, svg: String) -> Result<Vec<u8>, String> {
    match kind {
        Kind::Svg => Ok(svg.into_bytes()),
        Kind::Png => kashika_render::svg_to_png(&svg),
        Kind::Jpeg => kashika_render::svg_to_jpeg(&svg),
        Kind::Webp => kashika_render::svg_to_webp(&svg),
    }
}
