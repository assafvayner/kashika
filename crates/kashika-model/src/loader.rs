use std::io::Read;
use std::path::{Path, PathBuf};

use kashika_diag::did_you_mean;

/// How `compile_with` finds files.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The input file. Its directory is the project root and the base for relative paths;
    /// `None` (stdin) uses the current directory.
    pub path: Option<PathBuf>,
    /// Allows paths outside the project root (`--allow-path`).
    pub allow_path: bool,
    /// Extra search directories, from `KASHIKA_ICON_PATH`.
    pub icon_path: Vec<PathBuf>,
    /// Single files outside the project root that may be read (`--include`). Only the
    /// file itself is allowed, not its directory.
    pub include: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Found {
    File(PathBuf),
    Bundled(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FileError {
    pub message: String,
    pub help: Option<String>,
}

impl FileError {
    pub fn new(message: impl Into<String>) -> Self {
        FileError {
            message: message.into(),
            help: None,
        }
    }
}

pub(crate) struct Loader {
    root: PathBuf,
    main: Option<PathBuf>,
    search: Vec<PathBuf>,
    include: Vec<PathBuf>,
    allow_path: bool,
}

impl Loader {
    pub fn new(opts: &Options) -> Self {
        let dir = opts
            .path
            .as_deref()
            .and_then(Path::parent)
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let root = dir.canonicalize().unwrap_or(dir);
        let main = opts.path.as_deref().and_then(|p| p.canonicalize().ok());
        let search = opts
            .icon_path
            .iter()
            .filter_map(|p| p.canonicalize().ok())
            .collect();
        let include = opts
            .include
            .iter()
            .filter_map(|p| p.canonicalize().ok())
            .collect();
        Loader {
            root,
            main,
            search,
            include,
            allow_path: opts.allow_path,
        }
    }

    /// The project root: the input file's directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The canonical path of the input file, if it has one.
    pub fn main(&self) -> Option<&Path> {
        self.main.as_deref()
    }

    /// Adds the `icons=` directory (relative to the project root) to the front of the search path.
    pub fn add_icons_dir(&mut self, dir: &str) -> Result<(), FileError> {
        url_check(dir)?;
        let Ok(c) = self.root.join(dir).canonicalize() else {
            return Err(FileError::new(format!("icon directory `{dir}` not found")));
        };
        if !c.is_dir() {
            return Err(FileError::new(format!("`{dir}` is not a directory")));
        }
        self.check_inside(&c, dir)?;
        self.search.insert(0, c);
        Ok(())
    }

    fn check_inside(&self, p: &Path, written: &str) -> Result<(), FileError> {
        if self.allow_path
            || p.starts_with(&self.root)
            || self.search.iter().any(|s| p.starts_with(s))
            || self.include.iter().any(|f| f == p)
        {
            return Ok(());
        }
        Err(FileError {
            message: format!("`{written}` is outside the project root"),
            help: Some(
                "move it under the diagram's directory, pass `--include <file>` for this file, add its directory to KASHIKA_ICON_PATH, or pass --allow-path"
                    .into(),
            ),
        })
    }

    /// Finds `name` relative to `from` (the referencing file's directory), then on the
    /// search path. A hit that escapes the sandbox is an error.
    pub fn find(&self, from: &Path, name: &str) -> Result<Found, FileError> {
        url_check(name)?;
        for dir in self.dirs(from) {
            let p = dir.join(name);
            if p.is_file() {
                let c = p
                    .canonicalize()
                    .map_err(|e| FileError::new(format!("cannot read `{name}`: {e}")))?;
                self.check_inside(&c, name)?;
                return Ok(Found::File(c));
            }
        }
        if !name.contains(['/', '\\', '.']) {
            if let Some((n, _)) = crate::icons::bundled(name) {
                return Ok(Found::Bundled(n));
            }
        }
        if self.dirs(from).any(|dir| {
            let p = dir.join(name);
            p.is_dir() && self.allowed(&p)
        }) {
            return Err(FileError::new(format!(
                "`{name}` is a directory, not a file"
            )));
        }
        Err(FileError {
            message: format!("file `{name}` not found"),
            help: self.suggest(from, name),
        })
    }

    /// Reads a found file, refusing one larger than `max` bytes. The file is opened once
    /// and read through a `max + 1` byte window, so a file that grows after the check
    /// still can't exceed the limit.
    pub fn read(&self, found: &Found, written: &str, max: u64) -> Result<Vec<u8>, FileError> {
        let p = match found {
            Found::File(p) => p,
            Found::Bundled(n) => {
                let (_, src) =
                    crate::icons::bundled(n).expect("the loader only returns bundled names");
                return Ok(src.as_bytes().to_vec());
            }
        };
        let cannot = |e: std::io::Error| FileError::new(format!("cannot read `{written}`: {e}"));
        let mut bytes = Vec::new();
        std::fs::File::open(p)
            .map_err(cannot)?
            .take(max + 1)
            .read_to_end(&mut bytes)
            .map_err(cannot)?;
        if bytes.len() as u64 > max {
            return Err(FileError::new(format!(
                "`{written}` is larger than {}",
                show_bytes(max)
            )));
        }
        Ok(bytes)
    }

    fn dirs<'p>(&'p self, from: &'p Path) -> impl Iterator<Item = &'p Path> {
        std::iter::once(from).chain(self.search.iter().map(PathBuf::as_path))
    }

    /// Whether `p` exists inside the sandbox.
    fn allowed(&self, p: &Path) -> bool {
        p.canonicalize()
            .is_ok_and(|c| self.check_inside(&c, "").is_ok())
    }

    /// Near names for a missing file: files `find` would accept, never `name` itself.
    fn suggest(&self, from: &Path, name: &str) -> Option<String> {
        let sub = Path::new(name).parent().unwrap_or(Path::new(""));
        let mut cands: Vec<String> = Vec::new();
        for dir in self.dirs(from) {
            let Ok(rd) = std::fs::read_dir(dir.join(sub)) else {
                continue;
            };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() && self.allowed(&p) {
                    cands.push(sub.join(e.file_name()).to_string_lossy().into_owned());
                }
            }
        }
        if !name.contains('/') {
            cands.extend(
                crate::icons::BUNDLED_ICONS
                    .iter()
                    .map(|(n, _)| n.to_string()),
            );
        }
        cands.retain(|c| c != name);
        cands.sort();
        cands.dedup();
        did_you_mean(name, cands.iter().map(String::as_str))
            .map(|s| format!("did you mean `{s}`?"))
            .or_else(|| keyword_help(name))
    }
}

/// For a bare name that is a search keyword: the bundled icon or built-in shape it finds.
fn keyword_help(name: &str) -> Option<String> {
    if name.contains(['/', '\\', '.']) {
        return None;
    }
    let w = name.to_ascii_lowercase();
    if let Some((icon, _)) = crate::icons::ICON_KEYWORDS
        .iter()
        .find(|(_, words)| words.contains(&w.as_str()))
    {
        return Some(format!("did you mean the bundled icon `{icon}`?"));
    }
    crate::shapes::SHAPES
        .iter()
        .find(|s| s.keywords.contains(&w.as_str()))
        .map(|s| {
            format!(
                "no icon is called `{name}`; the built-in shape `{}` matches it: `shape={}`",
                s.name, s.name
            )
        })
}

fn show_bytes(n: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    if n.is_multiple_of(MIB) {
        format!("{} MiB", n / MIB)
    } else {
        format!("{n} bytes")
    }
}

fn url_check(name: &str) -> Result<(), FileError> {
    if name.contains("://") || name.starts_with("data:") {
        Err(FileError {
            message: format!("`{name}` is not a local file; only local files are supported"),
            help: Some("download it and refer to it by a relative path".into()),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(allow_path: bool) -> (tempfile::TempDir, Loader) {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("proj");
        std::fs::create_dir_all(proj.join("icons")).unwrap();
        std::fs::write(proj.join("icons/s3.svg"), "<svg/>").unwrap();
        std::fs::write(tmp.path().join("secret.svg"), "<svg/>").unwrap();
        let l = Loader::new(&Options {
            path: Some(proj.join("main.kash")),
            allow_path,
            ..Options::default()
        });
        (tmp, l)
    }

    fn outside(written: &str) -> FileError {
        FileError {
            message: format!("`{written}` is outside the project root"),
            help: Some(
                "move it under the diagram's directory, pass `--include <file>` for this file, add its directory to KASHIKA_ICON_PATH, or pass --allow-path"
                    .into(),
            ),
        }
    }

    fn not_found(written: &str) -> FileError {
        FileError {
            message: format!("file `{written}` not found"),
            help: None,
        }
    }

    /// A project at `<tmp>/proj` with `same.svg` in `proj/`, `proj/icons/`, `<tmp>/env/` and
    /// `<tmp>/env2/`, and a file named `server` in `<tmp>/env/` and `<tmp>/env2/`.
    fn layered() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("proj");
        for d in ["proj/icons", "env", "env2"] {
            std::fs::create_dir_all(tmp.path().join(d)).unwrap();
        }
        for f in [
            "proj/same.svg",
            "proj/icons/same.svg",
            "env/same.svg",
            "env2/same.svg",
            "env/server",
            "env2/server",
        ] {
            std::fs::write(tmp.path().join(f), "<svg/>").unwrap();
        }
        (tmp, proj)
    }

    fn loader(proj: &Path, icon_path: Vec<PathBuf>) -> Loader {
        Loader::new(&Options {
            path: Some(proj.join("main.kash")),
            icon_path,
            ..Options::default()
        })
    }

    fn file(p: PathBuf) -> Result<Found, FileError> {
        Ok(Found::File(p.canonicalize().unwrap()))
    }

    #[test]
    fn referencing_dir_beats_icons_dir() {
        let (_t, proj) = layered();
        let mut l = loader(&proj, Vec::new());
        l.add_icons_dir("icons").unwrap();
        assert_eq!(l.find(l.root(), "same.svg"), file(proj.join("same.svg")));
    }

    #[test]
    fn icons_dir_beats_env_path() {
        let (t, proj) = layered();
        std::fs::remove_file(proj.join("same.svg")).unwrap();
        let mut l = loader(&proj, vec![t.path().join("env")]);
        l.add_icons_dir("icons").unwrap();
        assert_eq!(
            l.find(l.root(), "same.svg"),
            file(proj.join("icons/same.svg"))
        );
    }

    #[test]
    fn env_path_beats_bundled_icons() {
        let (t, proj) = layered();
        let l = loader(&proj, vec![t.path().join("env")]);
        assert_eq!(
            l.find(l.root(), "server"),
            file(t.path().join("env/server"))
        );
        let l = loader(&proj, Vec::new());
        assert_eq!(l.find(l.root(), "server"), Ok(Found::Bundled("server")));
    }

    #[test]
    fn referencing_dir_beats_every_search_dir_and_bundled_icons() {
        let (t, proj) = layered();
        std::fs::write(proj.join("server"), "<svg/>").unwrap();
        std::fs::write(proj.join("icons/server"), "<svg/>").unwrap();
        let mut l = loader(&proj, vec![t.path().join("env")]);
        l.add_icons_dir("icons").unwrap();
        assert_eq!(l.find(l.root(), "server"), file(proj.join("server")));
    }

    #[test]
    fn env_path_entries_are_searched_in_order() {
        let (t, proj) = layered();
        std::fs::remove_file(proj.join("same.svg")).unwrap();
        let (env, env2) = (t.path().join("env"), t.path().join("env2"));
        let l = loader(&proj, vec![env.clone(), env2.clone()]);
        assert_eq!(l.find(l.root(), "same.svg"), file(env.join("same.svg")));
        let l = loader(&proj, vec![env2.clone(), env]);
        assert_eq!(l.find(l.root(), "same.svg"), file(env2.join("same.svg")));
    }

    #[test]
    fn absolute_paths_are_outside_the_root() {
        let (t, l) = setup(false);
        let abs = t.path().join("secret.svg");
        let written = abs.to_str().unwrap();
        assert_eq!(l.find(l.root(), written), Err(outside(written)));
    }

    #[test]
    fn dot_dot_resolves_through_the_file_system() {
        let (_t, l) = setup(false);
        assert_eq!(
            l.find(l.root(), "sub/../icons/s3.svg"),
            Err(not_found("sub/../icons/s3.svg"))
        );
        std::fs::create_dir(l.root().join("sub")).unwrap();
        assert!(matches!(
            l.find(l.root(), "sub/../icons/s3.svg"),
            Ok(Found::File(_))
        ));
    }

    #[test]
    fn directories_outside_the_root_are_not_found() {
        let (t, l) = setup(false);
        std::fs::create_dir(t.path().join("outdir")).unwrap();
        assert_eq!(l.find(l.root(), "../outdir"), Err(not_found("../outdir")));
    }

    #[cfg(unix)]
    #[test]
    fn the_first_hit_decides_even_when_it_escapes() {
        let (t, proj) = layered();
        std::fs::create_dir_all(t.path().join("out")).unwrap();
        std::fs::write(t.path().join("out/x.svg"), "<svg/>").unwrap();
        std::os::unix::fs::symlink(t.path().join("out"), proj.join("esc")).unwrap();
        std::fs::create_dir_all(t.path().join("env/esc")).unwrap();
        std::fs::write(t.path().join("env/esc/x.svg"), "<svg/>").unwrap();
        let l = loader(&proj, vec![t.path().join("env")]);
        assert_eq!(l.find(l.root(), "esc/x.svg"), Err(outside("esc/x.svg")));
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_icons_dir_outside_the_root_is_rejected() {
        let (t, proj) = layered();
        std::os::unix::fs::symlink(t.path().join("env"), proj.join("link")).unwrap();
        let mut l = loader(&proj, Vec::new());
        assert_eq!(l.add_icons_dir("link"), Err(outside("link")));
        assert_eq!(l.find(l.root(), "server"), Ok(Found::Bundled("server")));
    }

    #[test]
    fn finds_relative_files() {
        let (_t, l) = setup(false);
        let Ok(Found::File(p)) = l.find(l.root(), "icons/s3.svg") else {
            panic!("not found")
        };
        assert!(p.ends_with("icons/s3.svg"));
    }

    #[test]
    fn bundled_icons_by_bare_name() {
        let (_t, l) = setup(false);
        assert_eq!(l.find(l.root(), "server"), Ok(Found::Bundled("server")));
        let e = l.find(l.root(), "servr").unwrap_err();
        assert_eq!(e.help.as_deref(), Some("did you mean `server`?"));
    }

    #[test]
    fn missing_files_suggest_a_near_name() {
        let (_t, l) = setup(false);
        let e = l.find(l.root(), "icons/s4.svg").unwrap_err();
        assert_eq!(e.message, "file `icons/s4.svg` not found");
        assert_eq!(e.help.as_deref(), Some("did you mean `icons/s3.svg`?"));
    }

    #[test]
    fn suggestions_stay_inside_the_sandbox() {
        let (_t, l) = setup(false);
        let e = l.find(l.root(), "../secrt.svg").unwrap_err();
        assert_eq!(e.message, "file `../secrt.svg` not found");
        assert_eq!(e.help, None);
        let (_t, l) = setup(true);
        let e = l.find(l.root(), "../secrt.svg").unwrap_err();
        assert_eq!(e.help.as_deref(), Some("did you mean `../secret.svg`?"));
    }

    #[test]
    fn directories_are_named_as_such() {
        let (_t, l) = setup(false);
        let e = l.find(l.root(), "icons").unwrap_err();
        assert_eq!(e.message, "`icons` is a directory, not a file");
        let e = l.find(l.root(), "icon").unwrap_err();
        assert_eq!(e.message, "file `icon` not found");
        assert_eq!(e.help, None);
    }

    #[test]
    fn escaping_the_root_needs_allow_path() {
        let (_t, l) = setup(false);
        let e = l.find(l.root(), "../secret.svg").unwrap_err();
        assert_eq!(e, outside("../secret.svg"));
        let (_t, l) = setup(true);
        assert!(l.find(l.root(), "../secret.svg").is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_out_of_the_root_are_rejected() {
        let (t, _) = setup(false);
        let proj = t.path().join("proj");
        std::os::unix::fs::symlink(t.path().join("secret.svg"), proj.join("link.svg")).unwrap();
        let l = Loader::new(&Options {
            path: Some(proj.join("main.kash")),
            ..Options::default()
        });
        let e = l.find(l.root(), "link.svg").unwrap_err();
        assert_eq!(e, outside("link.svg"));
        let e = l.find(l.root(), "lnk.svg").unwrap_err();
        assert!(!e.help.unwrap_or_default().contains("link"));
    }

    #[test]
    fn reads_are_capped() {
        let (_t, l) = setup(false);
        let found = l.find(l.root(), "icons/s3.svg").unwrap();
        assert_eq!(l.read(&found, "icons/s3.svg", 6).unwrap(), b"<svg/>");
        let e = l.read(&found, "icons/s3.svg", 5).unwrap_err();
        assert_eq!(e.message, "`icons/s3.svg` is larger than 5 bytes");
        let server = l.find(l.root(), "server").unwrap();
        assert!(l.read(&server, "server", 0).is_ok());
    }

    #[test]
    fn urls_are_rejected() {
        let (_t, l) = setup(false);
        for name in [
            "https://example.com/x.svg",
            "file:///etc/hosts",
            "data:image/png;base64,AAAA",
        ] {
            let e = l.find(l.root(), name).unwrap_err();
            assert_eq!(
                e,
                FileError {
                    message: format!(
                        "`{name}` is not a local file; only local files are supported"
                    ),
                    help: Some("download it and refer to it by a relative path".into()),
                },
            );
        }
    }

    #[test]
    fn search_path_from_options_and_icons_dir() {
        let (t, _) = setup(false);
        let proj = t.path().join("proj");
        let l = Loader::new(&Options {
            path: Some(proj.join("main.kash")),
            icon_path: vec![proj.join("icons")],
            ..Options::default()
        });
        assert!(l.find(l.root(), "s3.svg").is_ok());
        let mut l = Loader::new(&Options {
            path: Some(proj.join("main.kash")),
            ..Options::default()
        });
        assert!(l.find(l.root(), "s3.svg").is_err());
        l.add_icons_dir("icons").unwrap();
        assert!(l.find(l.root(), "s3.svg").is_ok());
        assert!(l.add_icons_dir("..").is_err());
        assert!(l.add_icons_dir("nope").is_err());
    }

    #[test]
    fn included_files_are_allowed_one_by_one() {
        let (t, _) = setup(false);
        std::fs::write(t.path().join("other.svg"), "<svg/>").unwrap();
        let l = Loader::new(&Options {
            path: Some(t.path().join("proj/main.kash")),
            include: vec![t.path().join("proj/../secret.svg")],
            ..Options::default()
        });
        assert!(l.find(l.root(), "../secret.svg").is_ok());
        let e = l.find(l.root(), "../other.svg").unwrap_err();
        assert_eq!(e, outside("../other.svg"));
        let e = l.find(l.root(), "../secrt.svg").unwrap_err();
        assert_eq!(e.help.as_deref(), Some("did you mean `../secret.svg`?"));
    }

    #[test]
    fn env_search_dirs_are_trusted_roots() {
        let (t, _) = setup(false);
        let l = Loader::new(&Options {
            path: Some(t.path().join("proj/main.kash")),
            icon_path: vec![t.path().to_path_buf()],
            ..Options::default()
        });
        assert!(l.find(l.root(), "secret.svg").is_ok());
    }
}
