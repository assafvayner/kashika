use std::process::ExitCode;

use kashika_diag::did_you_mean;

use crate::Format;

const RULES: &str = include_str!("../ref/rules.md");
const SYNTAX: &str = include_str!("../ref/syntax.md");
const ANCHORS: &str = include_str!("../ref/anchors.md");
const PATHS: &str = include_str!("../ref/paths.md");
const LINTS: &str = include_str!("../ref/lints.md");
const RECIPES: &str = include_str!("../ref/recipes.md");
const EXAMPLE: &str = include_str!("../ref/example.md");
const UPLOAD: &str = include_str!("../../../examples/upload.kash");
const LIMITS: &str = include_str!("../ref/limits.md");
const SHAPES: &str = include_str!("../ref/shapes.md");
const STYLES: &str = include_str!("../ref/styles.md");
const METRICS: &str = include_str!("../ref/metrics.md");
const CLI: &str = include_str!("../ref/cli.md");

pub const TOPICS: &[&str] = &[
    "syntax", "anchors", "paths", "shapes", "styles", "metrics", "lints", "examples", "cli",
];

fn examples() -> String {
    format!("{RECIPES}\n{EXAMPLE}\n```kash\n{UPLOAD}```\n")
}

fn shapes() -> String {
    let mut t = String::from(
        "| shape | unit path (120x80 box) | outline | text area |\n|---|---|---|---|\n",
    );
    for name in kashika_model::BUILTIN_SHAPES {
        if let Some((d, outline, a)) = kashika_model::builtin_source(name, 120.0, 80.0) {
            let n = |v: f64| format!("{}", (v * 1000.0).round() / 1000.0);
            let d: Vec<String> = d
                .split_whitespace()
                .map(|w| w.parse().map_or(w.to_string(), n))
                .collect();
            t.push_str(&format!(
                "| `{name}` | `{}` | {outline} | ({},{},{},{}) |\n",
                d.join(" "),
                n(a.x),
                n(a.y),
                n(a.w),
                n(a.h)
            ));
        }
    }
    SHAPES.replace("{builtins}", t.trim_end())
}

fn metrics() -> String {
    const SAMPLE: &str = "The quick brown fox jumps over the lazy dog 0123456789";
    let n = SAMPLE.chars().count() as f64;
    let width = |size: f64, bold: bool| {
        kashika_text::measure(SAMPLE, size, bold).map_or(0.0, |b| b.width) / n
    };
    let mut s = String::from(METRICS);
    s.push_str("\n| font-size | regular | bold | line height |\n|---|---|---|---|\n");
    for size in [10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0] {
        s.push_str(&format!(
            "| {size} | {:.2} | {:.2} | {:.2} |\n",
            width(size, false),
            width(size, true),
            size * kashika_text::LINE_HEIGHT
        ));
    }
    s
}

pub fn page(topic: Option<&str>) -> Result<String, String> {
    let Some(topic) = topic else {
        let ex = examples();
        return Ok([RULES, SYNTAX, ANCHORS, PATHS, LINTS, ex.as_str(), LIMITS].join("\n"));
    };
    Ok(match topic {
        "syntax" => SYNTAX.into(),
        "anchors" => ANCHORS.into(),
        "paths" => PATHS.into(),
        "shapes" => shapes(),
        "styles" => STYLES.into(),
        "metrics" => metrics(),
        "lints" => LINTS.into(),
        "examples" => examples(),
        "cli" => CLI.into(),
        code => return lint(code).ok_or_else(|| unknown(code)),
    })
}

/// The cells of each row of the diagnostics table: code, severity, trigger, fix.
fn rows() -> impl Iterator<Item = Vec<&'static str>> {
    LINTS
        .lines()
        .filter(|l| l.starts_with("| `"))
        .map(|l| l.trim_matches('|').split('|').map(str::trim).collect())
}

fn lint(code: &str) -> Option<String> {
    let row = rows().find(|r| r[0].trim_matches('`') == code)?;
    Some(format!(
        "# {code} ({})\n\nTrigger: {}.\n\nFix: {}.\n",
        row[1], row[2], row[3]
    ))
}

fn unknown(topic: &str) -> String {
    let codes: Vec<&str> = rows().map(|r| r[0].trim_matches('`')).collect();
    let mut msg = format!("unknown ref topic `{topic}`");
    let all = TOPICS.iter().copied().chain(codes.iter().copied());
    if let Some(s) = did_you_mean(topic, all) {
        msg.push_str(&format!("; did you mean `{s}`?"));
    }
    msg.push_str(&format!(
        " Topics: {}, or a diagnostic code",
        TOPICS.join(", ")
    ));
    msg
}

pub fn run(topic: Option<&str>, json: bool) -> ExitCode {
    let format = if json { Format::Json } else { Format::Text };
    match page(topic) {
        Err(msg) => crate::tool_error(format, &msg),
        Ok(md) if json => {
            println!(
                "{}",
                serde_json::json!({
                    "schema": "kashika.ref/1",
                    "tool": env!("CARGO_PKG_VERSION"),
                    "topic": topic,
                    "topics": TOPICS,
                    "markdown": md,
                })
            );
            ExitCode::SUCCESS
        }
        Ok(md) => {
            print!("{md}");
            ExitCode::SUCCESS
        }
    }
}
