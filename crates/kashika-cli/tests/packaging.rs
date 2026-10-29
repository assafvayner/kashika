use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn every_crate_ships_the_root_license() {
    let license = std::fs::read(root().join("LICENSE")).unwrap();
    let mut crates = 0;
    for entry in std::fs::read_dir(root().join("crates")).unwrap() {
        let dir = entry.unwrap().path();
        if !dir.join("Cargo.toml").is_file() {
            continue;
        }
        crates += 1;
        assert_eq!(
            std::fs::read(dir.join("LICENSE")).ok().as_ref(),
            Some(&license),
            "copy LICENSE to {}",
            dir.display()
        );
    }
    assert!(crates > 0);
}
