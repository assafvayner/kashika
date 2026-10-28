mod anchor;
mod assets;
mod attrs;
mod form;
mod icons;
mod library;
mod loader;
mod markup;
mod resolve;
mod scene;
mod shapes;
mod style;
mod textlayout;

use kashika_diag::Diagnostic;

pub use assets::probe;
pub use form::Form;
pub use icons::BUNDLED_ICONS;
pub use kashika_text::{Font, PlacedLine, Run};
pub use loader::Options;
pub use resolve::{resolve, resolve_with};
pub use scene::*;
pub use shapes::{builtin_source, BUILTIN_SHAPES};
pub use style::{FontWeight, StrokeStyle, Style};
pub use textlayout::{TextItem, TextRole};

/// Parses and resolves a source file. Always returns a (possibly partial) scene, so tools
/// like `inspect` work on files with errors. Relative paths resolve from the current directory.
pub fn compile(src: &str) -> (Scene, Vec<Diagnostic>) {
    compile_with(src, &Options::default())
}

/// Like `compile`, with the input's path, sandbox and search path from `opts`.
pub fn compile_with(src: &str, opts: &Options) -> (Scene, Vec<Diagnostic>) {
    let (file, mut diags) = kashika_syntax::parse(src);
    let (scene, mut more) = resolve_with(&file, opts);
    diags.append(&mut more);
    kashika_diag::sort(&mut diags);
    (scene, diags)
}
