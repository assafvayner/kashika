use std::path::Path;
use std::process::ExitCode;

use crate::Format;

pub fn run(file: &str, check: bool) -> ExitCode {
    let src = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => return crate::tool_error(Format::Text, &format!("cannot read {file}: {e}")),
    };
    match kashika_syntax::format(&src) {
        Err(diags) => {
            print!("{}", crate::report::text(file, &src, &diags, 50, None));
            ExitCode::from(2)
        }
        Ok(out) if out == src => ExitCode::SUCCESS,
        Ok(_) if check => {
            println!("{file}: not formatted; run `kashika fmt {file}`");
            ExitCode::from(1)
        }
        Ok(out) => match crate::write_atomic(Path::new(file), out.as_bytes()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => crate::tool_error(Format::Text, &format!("cannot write {file}: {e}")),
        },
    }
}
