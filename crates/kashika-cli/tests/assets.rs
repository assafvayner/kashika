use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><path d="M 10 0 L 30 0 L 40 20 L 0 20 Z"/></svg>"##;

fn kashika() -> Command {
    Command::cargo_bin("kashika").unwrap()
}

fn write(dir: &Path, name: &str, body: &str) {
    let p = dir.join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

#[test]
fn render_resolves_files_next_to_the_input_and_embeds_them() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "lib/net.kash",
        "shape lb from \"../icons/lb.svg\"\n",
    );
    write(dir.path(), "icons/lb.svg", SVG);
    write(
        dir.path(),
        "arch.kash",
        "diagram \"a\" size (300,200)\nimport \"lib/net.kash\" as net\nbox lb \"LB\" at (20,20) size (80,40) shape=net.lb\n",
    );
    let out = dir.path().join("arch.svg");
    kashika()
        .current_dir(std::env::temp_dir())
        .args([
            "render",
            dir.path().join("arch.kash").to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("output: written"));
    let svg = std::fs::read_to_string(&out).unwrap();
    assert!(svg.contains("href=\"data:image/svg+xml;base64,"), "{svg}");
}

#[test]
fn escaping_paths_need_allow_path() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "shared/x.svg", SVG);
    write(
        tmp.path(),
        "proj/a.kash",
        "diagram \"a\" size (300,200)\nbox a at (0,0) size (40,40) icon=\"../shared/x.svg\"\n",
    );
    let file = tmp.path().join("proj/a.kash");
    kashika()
        .args(["check", file.to_str().unwrap()])
        .assert()
        .code(2)
        .stdout(
            predicate::str::contains("error[file]").and(predicate::str::contains("--allow-path")),
        );
    kashika()
        .args(["check", file.to_str().unwrap(), "--allow-path"])
        .assert()
        .code(0);
}

#[test]
fn icon_path_comes_from_the_environment() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "vendor/aws/s3.svg", SVG);
    write(
        tmp.path(),
        "proj/a.kash",
        "diagram \"a\" size (300,200)\nbox a at (0,0) size (40,40) icon=\"aws/s3.svg\"\n",
    );
    let file = tmp.path().join("proj/a.kash");
    kashika()
        .env_remove("KASHIKA_ICON_PATH")
        .args(["check", file.to_str().unwrap()])
        .assert()
        .code(2);
    kashika()
        .env("KASHIKA_ICON_PATH", tmp.path().join("vendor"))
        .args(["check", file.to_str().unwrap()])
        .assert()
        .code(0);
}

#[test]
fn stdin_resolves_from_the_current_directory() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "x.svg", SVG);
    kashika()
        .current_dir(dir.path())
        .args(["check", "-"])
        .write_stdin("diagram \"a\" size (100,100)\nbox a at (0,0) size (40,40) icon=\"x.svg\"\n")
        .assert()
        .code(0);
}

#[test]
fn icons_lists_bundled_icons_and_directory_contents() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "aws/s3.svg", SVG);
    write(
        dir.path(),
        "lib.kash",
        "shape lb {\n  path \"M 0 0 L 1 1\"\n}\n",
    );
    write(dir.path(), "notes.txt", "x");
    let d = dir.path().to_str().unwrap();
    kashika().args(["icons", d]).assert().code(0).stdout(
        predicate::str::contains("server")
            .and(predicate::str::contains("builtin:server"))
            .and(predicate::str::contains("aws/s3.svg"))
            .and(predicate::str::contains("40x20"))
            .and(predicate::str::contains("lb "))
            .and(predicate::str::contains("notes").not()),
    );
    kashika()
        .args(["icons", d, "--search", "S3"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("s3").and(predicate::str::contains("server").not()));
    let out = kashika()
        .args(["icons", d, "--format", "json"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["schema"], "kashika.icons/1");
    let s3 = v["icons"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "s3")
        .unwrap();
    assert_eq!(s3["size"], serde_json::json!([40.0, 20.0]));
    assert_eq!(s3["path"], "aws/s3.svg");
}

#[test]
fn icons_defaults_to_the_icon_path() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "db.svg", SVG);
    kashika()
        .env("KASHIKA_ICON_PATH", dir.path())
        .args(["icons"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("db.svg"));
}

#[test]
fn inspect_reports_image_natural_size_and_fit() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "logo.svg", SVG);
    write(
        dir.path(),
        "a.kash",
        "diagram \"a\" size (300,200)\nimage logo \"logo.svg\" at (20,20) size (80,80) fit=stretch\n",
    );
    let file = dir.path().join("a.kash");
    kashika()
        .args(["inspect", file.to_str().unwrap()])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("image logo (line 2)"))
        .stdout(predicate::str::contains(
            "  image \"logo.svg\" natural 40x20 aspect 2 fit=stretch",
        ));
    let out = kashika()
        .args(["inspect", file.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let img = &v["shapes"][0]["image"];
    assert_eq!(img["natural"], serde_json::json!([40.0, 20.0]));
    assert_eq!(img["aspect"], serde_json::json!(2.0));
    assert_eq!(img["fit"], "stretch");
}
