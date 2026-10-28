pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

const FORM_HELP: &str = "write kashika=\"0.1.0\"";

pub struct Problem {
    pub message: String,
    pub help: String,
}

fn parse(s: &str) -> Option<(u64, u64, u64)> {
    let part = |p: &str| {
        let ok = !p.is_empty()
            && p.bytes().all(|b| b.is_ascii_digit())
            && (p == "0" || !p.starts_with('0'));
        if ok {
            p.parse::<u64>().ok()
        } else {
            None
        }
    };
    let mut it = s.split('.');
    let major = part(it.next()?)?;
    let minor = part(it.next()?)?;
    let patch = match it.next() {
        Some(p) => part(p)?,
        None => 0,
    };
    if it.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn check(file: &str, tool: &str) -> Result<(), Problem> {
    let Some(want) = parse(file) else {
        return Err(Problem {
            message: format!("`{file}` is not a valid kashika version"),
            help: FORM_HELP.to_string(),
        });
    };
    let have = parse(tool).unwrap_or((0, 0, 0));
    if want.0 == have.0 && want <= have {
        return Ok(());
    }
    let (maj, min, pat) = want;
    let help = if want.0 == have.0 {
        "upgrade kashika, or lower `kashika=` if the file uses nothing newer".to_string()
    } else {
        format!(
            "use a kashika {maj}.x release, or set `kashika=` to a {}.x version",
            have.0
        )
    };
    Err(Problem {
        message: format!(
            "this file needs kashika {maj}.{min}.{pat} or newer (same major version); this is kashika {tool}"
        ),
        help,
    })
}

pub fn form_help() -> &'static str {
    FORM_HELP
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(f: &str, t: &str) -> bool {
        check(f, t).is_ok()
    }

    #[test]
    fn accepts_equal_older_and_shorthand() {
        assert!(ok("0.1.0", "0.1.0"));
        assert!(ok("0.0.5", "0.1.0"));
        assert!(ok("0.1", "0.1.0"));
        assert!(ok("1.2.0", "1.4.3"));
        assert!(ok("1.4", "1.4.3"));
    }

    #[test]
    fn rejects_newer_and_other_major() {
        assert!(!ok("0.2.0", "0.1.0"));
        assert!(!ok("0.1.1", "0.1.0"));
        assert!(!ok("1.0.0", "0.1.0"));
        assert!(!ok("1.5.0", "1.4.3"));
        assert!(!ok("1.0.0", "2.0.0"));
        assert!(!ok("0.9", "1.0.0"));
    }

    #[test]
    fn minor_not_enforced_across_older_minor_in_0x() {
        assert!(ok("0.1.0", "0.7.2"));
    }

    #[test]
    fn rejects_malformed() {
        for s in [
            "1",
            "",
            "0.1.0-beta",
            "0.1.0+b",
            "1.2.3.4",
            "a.b",
            "0.1.",
            ".1",
            "01.0",
            "v0.1",
            "0.-1",
        ] {
            let p = check(s, "0.1.0").err().unwrap_or_else(|| panic!("{s}"));
            assert!(p.help.contains("kashika=\"0.1.0\""), "{s}");
        }
    }

    #[test]
    fn message_text() {
        let p = check("0.4", "0.1.0").err().unwrap();
        assert_eq!(
            p.message,
            "this file needs kashika 0.4.0 or newer (same major version); this is kashika 0.1.0"
        );
    }
}
