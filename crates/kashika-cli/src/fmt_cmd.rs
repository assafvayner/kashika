use std::path::Path;
use std::process::ExitCode;

use crate::{Format, report};

pub fn run(file: &str, check: bool, stdin_name: Option<&str>) -> ExitCode {
    let stdin = file == "-";
    let (file, src) = match crate::read_input(file, stdin_name) {
        Ok(v) => v,
        Err(msg) => return crate::tool_error(Format::Text, report::SCHEMA, &msg),
    };
    let file = file.as_str();
    match kashika_syntax::format(&src) {
        Err(mut diags) => {
            let opts = crate::report::Opts {
                max: 50,
                show_suppressed: false,
                summary: false,
                metrics: None,
                deferred: None,
                omitted: &[],
            };
            print!(
                "{}",
                crate::report::text(file, &src, &mut diags, &opts, None)
            );
            ExitCode::from(2)
        }
        Ok(out) if stdin && !check => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        Ok(out) if out == src => ExitCode::SUCCESS,
        Ok(_) if check => {
            let file = crate::term::line(file);
            if stdin {
                println!("{file}: not formatted");
            } else {
                println!("{file}: not formatted; run `kashika fmt {file}`");
            }
            ExitCode::from(1)
        }
        Ok(out) => match crate::write_atomic(Path::new(file), out.as_bytes()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => crate::tool_error(
                Format::Text,
                report::SCHEMA,
                &format!("cannot write {file}: {e}"),
            ),
        },
    }
}
