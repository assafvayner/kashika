use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn kashika() -> Command {
    Command::cargo_bin("kashika").unwrap()
}

#[test]
fn render_writes_png() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.png");
    kashika()
        .args([
            "render",
            &fixture("basic.kash"),
            "-o",
            out.to_str().unwrap(),
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("output: written"));
    assert_eq!(&std::fs::read(&out).unwrap()[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn force_draws_red_markers_at_errors() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.svg");
    kashika()
        .args([
            "render",
            &fixture("typo.kash"),
            "-o",
            out.to_str().unwrap(),
            "--force",
        ])
        .assert()
        .code(2);
    let svg = std::fs::read_to_string(&out).unwrap();
    assert!(svg.contains("<g id=\"kashika-overlay\">"));
    assert!(svg.contains("error[unknown-anchor]"));
}

#[test]
fn clean_render_has_no_overlay_and_debug_adds_one() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("plain.svg");
    let debug = dir.path().join("debug.svg");
    kashika()
        .args([
            "render",
            &fixture("basic.kash"),
            "-o",
            plain.to_str().unwrap(),
        ])
        .assert()
        .code(0);
    kashika()
        .args([
            "render",
            &fixture("basic.kash"),
            "-o",
            debug.to_str().unwrap(),
            "--debug",
        ])
        .assert()
        .code(0);
    assert!(!std::fs::read_to_string(&plain)
        .unwrap()
        .contains("kashika-overlay"));
    assert!(std::fs::read_to_string(&debug)
        .unwrap()
        .contains("kashika-overlay"));
}

#[test]
fn debug_marks_lint_warnings_at_their_geometry() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("debug.svg");
    kashika()
        .args([
            "render",
            &fixture("crossing.kash"),
            "-o",
            out.to_str().unwrap(),
            "--debug",
        ])
        .assert()
        .code(1);
    let svg = std::fs::read_to_string(&out).unwrap();
    assert!(svg.contains("<circle cx=\"200\" cy=\"150\" r=\"6\" fill=\"none\" stroke=\"#ea580c\""));
    assert!(svg.contains("aria-label=\"warn[cross]\""));
    assert!(
        !svg.contains("<circle cx=\"240\" cy=\"150\""),
        "suppressed findings are not marked"
    );
}

fn render_to(ext: &str, extra: &[&str]) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join(format!("out.{ext}"));
    let mut args = vec![
        "render".to_string(),
        fixture("basic.kash"),
        "-o".to_string(),
        out.to_str().unwrap().to_string(),
    ];
    args.extend(extra.iter().map(|s| s.to_string()));
    kashika().args(&args).assert().code(0);
    std::fs::read(&out).unwrap()
}

fn png_size(b: &[u8]) -> (u32, u32) {
    let u = |i: usize| u32::from_be_bytes(b[i..i + 4].try_into().unwrap());
    (u(16), u(20))
}

fn jpeg_size(b: &[u8]) -> (u32, u32) {
    let mut i = 2;
    while i + 9 < b.len() {
        assert_eq!(b[i], 0xFF);
        let marker = b[i + 1];
        if matches!(marker, 0xC0..=0xC3) {
            let u = |j: usize| u32::from(u16::from_be_bytes([b[j], b[j + 1]]));
            return (u(i + 7), u(i + 5));
        }
        i += 2 + usize::from(u16::from_be_bytes([b[i + 2], b[i + 3]]));
    }
    panic!("no SOF marker");
}

fn webp_size(b: &[u8]) -> (u32, u32) {
    assert_eq!(&b[12..16], b"VP8L");
    let bits = u32::from_le_bytes(b[21..25].try_into().unwrap());
    ((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1)
}

#[test]
fn render_writes_jpeg_and_webp_with_diagram_size() {
    let expected = png_size(&render_to("png", &[]));
    for ext in ["jpg", "jpeg"] {
        let b = render_to(ext, &[]);
        assert_eq!(&b[..3], b"\xFF\xD8\xFF");
        assert_eq!(jpeg_size(&b), expected);
    }
    let b = render_to("webp", &[]);
    assert_eq!(&b[..4], b"RIFF");
    assert_eq!(&b[8..12], b"WEBP");
    assert_eq!(webp_size(&b), expected);
}

#[test]
fn jpeg_and_webp_are_deterministic() {
    for ext in ["jpg", "webp"] {
        assert_eq!(render_to(ext, &[]), render_to(ext, &[]));
    }
}

#[test]
fn debug_overlay_applies_to_jpeg_and_webp() {
    for ext in ["jpg", "webp"] {
        assert_ne!(render_to(ext, &[]), render_to(ext, &["--debug"]));
    }
}

#[test]
fn unsupported_extension_lists_supported_ones() {
    kashika()
        .args(["render", &fixture("basic.kash"), "-o", "x.gif"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "output must end in .svg, .png, .jpg, .jpeg or .webp",
        ));
}
