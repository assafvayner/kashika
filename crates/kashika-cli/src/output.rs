use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Svg,
    Png,
    Jpeg,
    Webp,
}

/// Writes to stdout; a failed write (e.g. a closed pipe) is ignored and the command keeps the exit code it already decided.
pub fn write_out(args: std::fmt::Arguments) {
    use std::io::Write;
    let _ = std::io::stdout().lock().write_fmt(args);
}

/// Writes to stderr; a failed write is ignored, as in `write_out`.
pub fn write_err(args: std::fmt::Arguments) {
    use std::io::Write;
    let _ = std::io::stderr().lock().write_fmt(args);
}

#[macro_export]
macro_rules! out {
    ($($arg:tt)*) => { $crate::output::write_out(format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! outln {
    ($($arg:tt)*) => { $crate::output::write_out(format_args!("{}\n", format_args!($($arg)*))) };
}

#[macro_export]
macro_rules! errln {
    ($($arg:tt)*) => { $crate::output::write_err(format_args!("{}\n", format_args!($($arg)*))) };
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
