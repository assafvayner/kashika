use std::path::{Path, PathBuf};

/// Every file name the harness writes into a case directory.
pub const OUTPUTS: &[&str] = &[
    "ast.txt",
    "scene.json",
    "diagnostics.txt",
    "out.svg",
    "out.png",
    "out.jpg",
    "out.webp",
];

const RASTERS: &[&str] = &["png", "jpg", "webp"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suite {
    Parse,
    Render,
}

#[derive(Debug, Clone)]
pub struct Case {
    /// Path relative to `tests/golden`, with `/` separators, e.g. `parse/lexical/bom`.
    pub name: String,
    pub dir: PathBuf,
    pub suite: Suite,
    pub src: String,
    pub spec: Vec<String>,
    pub dump_ast: bool,
    pub dump_scene: bool,
    pub force: bool,
    pub debug: bool,
    pub allow_path: bool,
    pub raster: Vec<String>,
    pub icon_path: Option<String>,
    /// `--include` files, relative to the case directory.
    pub include: Vec<String>,
}

pub fn golden_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

/// Every directory under `root` holding an `input.kash`, sorted by name. Case directories
/// are not searched further, so auxiliary subdirectories (`lib/`, `icons/`) are never cases.
pub fn discover(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if dir.join("input.kash").is_file() {
            found.push(dir);
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            if e.path().is_dir() {
                stack.push(e.path());
            }
        }
    }
    found.sort();
    found
}

impl Case {
    pub fn load(root: &Path, dir: &Path) -> Result<Case, String> {
        let name = dir
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let suite = match name.split('/').next() {
            Some("parse") => Suite::Parse,
            Some("render") => Suite::Render,
            _ => return Err(format!("{name}: cases live under parse/ or render/")),
        };
        let src = std::fs::read_to_string(dir.join("input.kash"))
            .map_err(|e| format!("{name}: cannot read input.kash: {e}"))?;
        let mut case = Case {
            name,
            dir: dir.to_path_buf(),
            suite,
            src,
            spec: Vec::new(),
            dump_ast: false,
            dump_scene: false,
            force: false,
            debug: false,
            allow_path: false,
            raster: Vec::new(),
            icon_path: None,
            include: Vec::new(),
        };
        case.read_directives()?;
        Ok(case)
    }

    /// Reads the `// key: value` lines at the top of `input.kash`. A leading comment line
    /// whose text before `:` is one lowercase word is a directive and must be known.
    fn read_directives(&mut self) -> Result<(), String> {
        let src = self.src.trim_start_matches('\u{feff}').to_string();
        for line in src.lines() {
            let Some(text) = line.strip_prefix("// ") else {
                break;
            };
            let Some((key, value)) = text.split_once(": ") else {
                continue;
            };
            if !key.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
                continue;
            }
            let values: Vec<String> = value
                .split([',', ' '])
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .collect();
            match (key, self.suite) {
                ("spec", _) => self.spec = values,
                ("dump", Suite::Parse) => {
                    for v in values {
                        match v.as_str() {
                            "ast" => self.dump_ast = true,
                            "scene" => self.dump_scene = true,
                            _ => return Err(format!("{}: unknown dump `{v}`", self.name)),
                        }
                    }
                }
                ("flags", _) => {
                    for v in values {
                        match (v.as_str(), self.suite) {
                            ("allow-path", _) => self.allow_path = true,
                            ("force", Suite::Render) => self.force = true,
                            ("debug", Suite::Render) => self.debug = true,
                            _ => return Err(format!("{}: unknown flag `{v}`", self.name)),
                        }
                    }
                }
                ("raster", Suite::Render) => {
                    for v in &values {
                        if !RASTERS.contains(&v.as_str()) {
                            return Err(format!("{}: unknown raster `{v}`", self.name));
                        }
                    }
                    self.raster = values;
                }
                ("icon-path", _) => self.icon_path = values.into_iter().next(),
                ("include", _) => self.include.extend(values),
                _ => {
                    return Err(format!(
                        "{}: unknown directive `{key}` for this suite",
                        self.name
                    ));
                }
            }
        }
        if self.spec.is_empty() {
            return Err(format!(
                "{}: input.kash must start with `// spec: <sections>`",
                self.name
            ));
        }
        Ok(())
    }
}
