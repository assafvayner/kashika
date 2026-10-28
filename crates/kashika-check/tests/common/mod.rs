#![allow(dead_code)]

use kashika_diag::Diagnostic;
use kashika_model::{Scene, compile};

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

/// A PNG header with the given natural size; enough for size probing.
pub fn png(w: u32, h: u32) -> Vec<u8> {
    let mut b = vec![
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
    ];
    b.extend(w.to_be_bytes());
    b.extend(h.to_be_bytes());
    b.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    b
}

/// Like `lint`, for a body whose files live in `dir`. Writes `logo.png` (200x100) there.
pub fn lint_in(dir: &std::path::Path, code: &str, body: &str) -> Vec<Diagnostic> {
    std::fs::write(dir.join("logo.png"), png(200, 100)).unwrap();
    let opts = kashika_model::Options {
        path: Some(dir.join("t.kash")),
        ..Default::default()
    };
    let (s, d) = kashika_model::compile_with(&format!("{HEAD}{body}"), &opts);
    assert!(d.is_empty(), "{d:#?}");
    kashika_check::check(&s)
        .into_iter()
        .filter(|d| d.code == code)
        .collect()
}
