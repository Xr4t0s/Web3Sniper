//! Minimal ANSI styling. Disabled automatically when stdout is not a TTY or
//! `NO_COLOR` is set, so piped output stays clean.

use std::io::IsTerminal;
use std::sync::OnceLock;

fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED
        .get_or_init(|| std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none())
}

fn paint(code: &str, s: &str) -> String {
    if enabled() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn dim(s: &str) -> String {
    paint("2", s)
}
pub fn bold(s: &str) -> String {
    paint("1", s)
}
pub fn red(s: &str) -> String {
    paint("31", s)
}
pub fn green(s: &str) -> String {
    paint("32", s)
}
pub fn yellow(s: &str) -> String {
    paint("33", s)
}
pub fn cyan(s: &str) -> String {
    paint("36", s)
}
pub fn bold_cyan(s: &str) -> String {
    paint("1;36", s)
}

/// `0x1234…abcd` — a hex string trimmed to head and tail.
pub fn short_hex(s: &str) -> String {
    match s.strip_prefix("0x") {
        Some(body) if body.len() > 12 => {
            format!("0x{}…{}", &body[..4], &body[body.len() - 4..])
        }
        _ => s.to_string(),
    }
}
