use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn kashika() -> Command {
    Command::cargo_bin("kashika").unwrap()
}

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// Extensions allowed in an example directory: source, renders and imported .kash files.
const ALLOWED_EXTENSIONS: &[&str] = &["svg", "png", "kash"];

#[test]
fn example_renders_are_committed_and_current() {
    let mut names = Vec::new();
    for e in std::fs::read_dir(examples_dir()).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            names.push(p.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    names.sort();
    assert!(!names.is_empty());

    let readme = std::fs::read_to_string(examples_dir().join("README.md")).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    for name in names {
        assert!(
            readme.lines().any(|l| l == format!("## {name}")),
            "examples/README.md has no `## {name}` section"
        );
        let dir = examples_dir().join(&name);
        let src = dir.join(format!("{name}.kash"));
        let regen = format!(
            "regenerate with: for ext in svg png; do cargo run -p kashika -- render examples/{name}/{name}.kash -o examples/{name}/{name}.$ext; done"
        );

        let files: BTreeSet<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        for required in [
            format!("{name}.kash"),
            format!("{name}.svg"),
            format!("{name}.png"),
        ] {
            assert!(
                files.contains(&required),
                "examples/{name}/ is missing {required}; {regen}"
            );
        }
        for f in &files {
            let ext = Path::new(f).extension().and_then(|x| x.to_str());
            assert!(
                ext.is_some_and(|x| ALLOWED_EXTENSIONS.contains(&x)),
                "unexpected file examples/{name}/{f}"
            );
        }

        kashika()
            .args(["check", src.to_str().unwrap()])
            .assert()
            .code(0)
            .stdout(predicates::str::contains("0 errors, 0 warnings"));

        kashika()
            .args(["fmt", "--check", src.to_str().unwrap()])
            .assert()
            .code(0);

        for ext in ["svg", "png"] {
            let out = tmp.path().join(format!("{name}.{ext}"));
            kashika()
                .args(["render", src.to_str().unwrap(), "-o", out.to_str().unwrap()])
                .assert()
                .code(0);
            let fresh = std::fs::read(&out).unwrap();
            let committed = std::fs::read(dir.join(format!("{name}.{ext}"))).unwrap();
            assert!(
                fresh == committed,
                "examples/{name}/{name}.{ext} is stale; {regen}"
            );
        }
    }
}
