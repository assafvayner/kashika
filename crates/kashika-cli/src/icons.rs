use std::path::{Path, PathBuf};

use clap::Args;
use kashika_syntax::ast::Stmt;
use serde::Serialize;

use crate::Format;

pub const SCHEMA: &str = "kashika.icons/1";

#[derive(Args)]
pub struct IconsArgs {
    /// Directories to scan (default: the KASHIKA_ICON_PATH entries)
    dirs: Vec<PathBuf>,
    /// Only list entries whose name, path or keyword contains this term; exits 1 when nothing matches
    #[arg(long)]
    search: Option<String>,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

pub fn run(a: &IconsArgs) -> std::process::ExitCode {
    let dirs = if a.dirs.is_empty() {
        crate::icon_path()
    } else {
        a.dirs.clone()
    };
    if let Some(d) = dirs.iter().find(|d| !d.is_dir()) {
        return crate::tool_error(
            a.format,
            SCHEMA,
            &format!("{} is not a directory", d.display()),
        );
    }
    let entries = list(&dirs, a.search.as_deref());
    print!(
        "{}",
        match a.format {
            Format::Text => crate::term::text(&text(&entries)).into_owned(),
            Format::Json => json(&entries),
        }
    );
    match (&a.search, entries.is_empty()) {
        (Some(term), true) => {
            eprintln!(
                "kashika: no icons or shapes match `{}`",
                crate::term::line(term)
            );
            std::process::ExitCode::from(1)
        }
        _ => std::process::ExitCode::SUCCESS,
    }
}

#[derive(Debug, Serialize)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub size: Option<[f64; 2]>,
    pub library: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_keyword: Option<String>,
    #[serde(skip)]
    keywords: &'static [&'static str],
}

/// Bundled icons and built-in shapes first, then every image and `.kash` shape under `dirs`, in sorted order.
pub fn list(dirs: &[PathBuf], search: Option<&str>) -> Vec<Entry> {
    let mut out: Vec<Entry> = kashika_model::BUNDLED_ICONS
        .iter()
        .map(|(name, src)| Entry {
            name: name.to_string(),
            path: format!("builtin:{name}"),
            size: kashika_model::probe(src.as_bytes(), true)
                .ok()
                .map(|(w, h)| [w, h]),
            library: "kashika".into(),
            matched_keyword: None,
            keywords: keywords(kashika_model::ICON_KEYWORDS, name),
        })
        .collect();
    out.extend(kashika_model::SHAPES.iter().map(|s| Entry {
        name: s.name.to_string(),
        path: format!("builtin:shape:{}", s.name),
        size: None,
        library: "kashika".into(),
        matched_keyword: None,
        keywords: s.keywords,
    }));
    for dir in dirs {
        let mut files = Vec::new();
        walk(dir, &mut files);
        files.sort();
        for f in files {
            entries(dir, &f, &mut out);
        }
    }
    if let Some(term) = search {
        let term = term.to_lowercase();
        out.retain_mut(|e| {
            if e.name.to_lowercase().contains(&term) || e.path.to_lowercase().contains(&term) {
                return true;
            }
            e.matched_keyword = e
                .keywords
                .iter()
                .find(|k| k.contains(&term))
                .map(|k| k.to_string());
            e.matched_keyword.is_some()
        });
    }
    out
}

fn keywords(table: &'static [(&str, &[&str])], name: &str) -> &'static [&'static str] {
    table
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(&[], |(_, k)| *k)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if e.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let Ok(kind) = e.file_type() else {
            continue;
        };
        if kind.is_dir() {
            walk(&p, out);
        } else if !kind.is_symlink() || p.is_file() {
            out.push(p);
        }
    }
}

fn entries(dir: &Path, f: &Path, out: &mut Vec<Entry>) {
    let rel = f
        .strip_prefix(dir)
        .unwrap_or(f)
        .to_string_lossy()
        .replace('\\', "/");
    let library = dir.display().to_string();
    let ext = f
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let stem = f
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    match ext.as_str() {
        "svg" | "png" | "jpg" | "jpeg" => {
            let size = std::fs::read(f)
                .ok()
                .and_then(|b| kashika_model::probe(&b, ext == "svg").ok())
                .map(|(w, h)| [w, h]);
            out.push(Entry {
                name: stem,
                path: rel,
                size,
                library,
                matched_keyword: None,
                keywords: &[],
            });
        }
        "kash" => {
            let Ok(src) = std::fs::read_to_string(f) else {
                return;
            };
            let (file, _) = kashika_syntax::parse(&src);
            for stmt in file.stmts {
                if let Stmt::Shape(s) = stmt {
                    out.push(Entry {
                        name: s.name.name,
                        path: rel.clone(),
                        size: None,
                        library: library.clone(),
                        matched_keyword: None,
                        keywords: &[],
                    });
                }
            }
        }
        _ => {}
    }
}

pub fn text(entries: &[Entry]) -> String {
    let mut out = String::new();
    for e in entries {
        let size = e
            .size
            .map_or_else(|| "-".to_string(), |[w, h]| format!("{w}x{h}"));
        let note = e
            .matched_keyword
            .as_ref()
            .map_or(String::new(), |k| format!("  (matches \"{k}\")"));
        out.push_str(&format!(
            "{:<16} {:<32} {:<9} {}{}\n",
            e.name, e.path, size, e.library, note
        ));
    }
    out
}

pub fn json(entries: &[Entry]) -> String {
    format!(
        "{}\n",
        serde_json::json!({ "schema": SCHEMA, "icons": entries })
    )
}
