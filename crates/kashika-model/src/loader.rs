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
        Loader {
            root,
            main,
            search,
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
        {
            return Ok(());
        }
        Err(FileError {
            message: format!("`{written}` is outside the project root"),
            help: Some(
                "move it under the diagram's directory, add its directory to KASHIKA_ICON_PATH, or pass --allow-path"
                    .into(),
            ),
        })
    }

    /// Finds `name` relative to `from` (the referencing file's directory), then on the
    /// search path. A hit that escapes the sandbox is an error.
    pub fn find(&self, from: &Path, name: &str) -> Result<Found, FileError> {
        url_check(name)?;
        for dir in std::iter::once(from).chain(self.search.iter().map(PathBuf::as_path)) {
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
        Err(FileError {
            message: format!("file `{name}` not found"),
            help: self.suggest(from, name),
        })
    }

    fn suggest(&self, from: &Path, name: &str) -> Option<String> {
        let sub = Path::new(name).parent().unwrap_or(Path::new(""));
        let mut cands: Vec<String> = Vec::new();
        for dir in std::iter::once(from).chain(self.search.iter().map(PathBuf::as_path)) {
            let Ok(rd) = std::fs::read_dir(dir.join(sub)) else {
                continue;
            };
            for e in rd.flatten() {
                cands.push(sub.join(e.file_name()).to_string_lossy().into_owned());
            }
        }
        if !name.contains('/') {
            cands.extend(
                crate::icons::BUNDLED_ICONS
                    .iter()
                    .map(|(n, _)| n.to_string()),
            );
        }
        cands.sort();
        cands.dedup();
        did_you_mean(name, cands.iter().map(String::as_str)).map(|s| format!("did you mean `{s}`?"))
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
    fn escaping_the_root_needs_allow_path() {
        let (_t, l) = setup(false);
        let e = l.find(l.root(), "../secret.svg").unwrap_err();
        assert!(
            e.message.contains("outside the project root"),
            "{}",
            e.message
        );
        let (_t, l) = setup(true);
        assert!(l.find(l.root(), "../secret.svg").is_ok());
    }

    #[test]
    fn urls_are_rejected() {
        let (_t, l) = setup(false);
        let e = l.find(l.root(), "https://example.com/x.svg").unwrap_err();
        assert!(e.message.contains("not a local file"), "{}", e.message);
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
