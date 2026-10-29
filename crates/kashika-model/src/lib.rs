mod anchor;
mod assets;
mod attrs;
mod colors;
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
mod version;

use kashika_diag::Diagnostic;

pub use assets::{MAX_RASTER_PIXELS, MAX_RASTER_SIDE, probe};
pub use form::Form;
pub use icons::{BUNDLED_ICONS, ICON_KEYWORDS};
pub use kashika_text::{Font, PlacedLine, Run};
pub use loader::Options;
pub use resolve::{resolve, resolve_library_with, resolve_with};
pub use scene::*;
pub use shapes::{BUILTIN_SHAPES, SHAPE_KEYWORDS, builtin_source};
pub use style::{FontWeight, Padding, StrokeStyle, Style};
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

/// Checks a library file (§3a: no `diagram`, only `shape`, `style` and `import`) on its own,
/// resolving its definitions. `None` when `src` is not a library; compile it as a diagram.
pub fn compile_library_with(src: &str, opts: &Options) -> Option<Vec<Diagnostic>> {
    let (file, mut diags) = kashika_syntax::parse(src);
    diags.extend(resolve_library_with(&file, opts)?);
    kashika_diag::sort(&mut diags);
    Some(diags)
}
