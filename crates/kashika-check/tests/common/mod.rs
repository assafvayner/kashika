#![allow(dead_code)]

use kashika_diag::Diagnostic;
use kashika_model::{compile, Scene};

pub const HEAD: &str = "diagram \"t\" size (800,600)\n";

/// Compiles a full source file, asserting that it has no errors.
pub fn scene_src(src: &str) -> Scene {
    let (s, d) = compile(src);
    assert!(d.is_empty(), "{d:#?}");
    s
}

pub fn scene(body: &str) -> Scene {
    scene_src(&format!("{HEAD}{body}"))
}

/// The findings with `code` that `kashika_check::check` reports for `body`.
pub fn lint(code: &str, body: &str) -> Vec<Diagnostic> {
    kashika_check::check(&scene(body))
        .into_iter()
        .filter(|d| d.code == code)
        .collect()
}
