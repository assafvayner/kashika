mod anchor;
mod attrs;
mod markup;
mod resolve;
mod scene;
mod style;

use kashika_diag::Diagnostic;

pub use resolve::resolve;
pub use scene::*;
pub use style::{FontWeight, StrokeStyle, Style};

/// Parses and resolves a source file. Always returns a (possibly partial) scene, so tools
/// like `inspect` work on files with errors.
pub fn compile(src: &str) -> (Scene, Vec<Diagnostic>) {
    let (file, mut diags) = kashika_syntax::parse(src);
    let (scene, mut more) = resolve(&file);
    diags.append(&mut more);
    kashika_diag::sort(&mut diags);
    (scene, diags)
}
