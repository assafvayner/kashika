# Releasing

All crates in `crates/` are published to crates.io at one shared version. The CLI is the `kashika` crate (directory `crates/kashika-cli`, binary `kashika`).

1. Bump the version in the root `Cargo.toml`: `version` under `[workspace.package]` and the `=X.Y.Z` pin on every `kashika-*` entry in `[workspace.dependencies]`. Then run `cargo check --workspace` to update `Cargo.lock`.
2. Add a section for the new version to `CHANGELOG.md`.
3. Check everything on a clean tree:

   ```sh
   cargo fmt --all --check
   cargo clippy --workspace --all-targets --locked -- -D warnings
   cargo test --workspace --locked
   cargo publish --workspace --dry-run --locked
   ```

   The dry run packages and builds every crate from its packaged sources, in dependency order. The "ignoring test" warnings are expected: tests are left out of the packages because they read files elsewhere in the repository.
4. Commit the bump and changelog.
5. Publish, after `cargo login` with a crates.io API token that can publish new crates:

   ```sh
   cargo publish --workspace --locked
   ```

   Cargo uploads the crates in dependency order: `kashika-diag`, `kashika-geom`, `kashika-syntax`, `kashika-text`, `kashika-model`, `kashika-check`, `kashika-render`, `kashika-route`, `kashika`. If it stops partway, fix the cause and rerun with `-p` for the crates not yet published; a published version can't be replaced, only yanked.
6. Tag and push the tag:

   ```sh
   git tag v0.1.0-dev0
   git push origin v0.1.0-dev0
   ```

## Prereleases

A version with a `-` suffix, such as `0.1.0-dev0`, is a prerelease. Cargo never selects a prerelease unless the version requirement names one, so `cargo install kashika` skips it and fails while only prereleases are published. Install it with the exact version:

```sh
cargo install kashika --version 0.1.0-dev0
```

A prerelease tool compares as its release for `kashika=` in diagram files, so `0.1.0-dev0` accepts files that declare `kashika="0.1.0"`.
