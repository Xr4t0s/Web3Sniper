//! Startup banner. Skipped when stdout is not a terminal, so piped or
//! `DEBUG=true` JSONL output stays machine-clean.

use std::io::IsTerminal;

use crate::stages::style::{bold_cyan, dim};

pub fn print() {
    if !std::io::stdout().is_terminal() {
        return;
    }
    let art = [
        r"                _     _____             _                ",
        r"__      _____  | |__ |___ / ___ _ __ (_)_ __  ___ _ __  ",
        r"\ \ /\ / / _ \ | '_ \  |_ \/ __| '_ \| | '_ \/ -_) '__| ",
        r" \ V  V /  __/ | |_) |___) \__ \ | | | | |_) \___|_|    ",
        r"  \_/\_/ \___| |_.__/|____/|___/_| |_|_| .__/           ",
        r"                                       |_|              ",
    ];
    println!();
    for line in art {
        println!("{}", bold_cyan(line));
    }
    println!(
        "  {}",
        dim(&format!(
            "v{} · on-chain launch & graduation watcher",
            env!("CARGO_PKG_VERSION"),
        )),
    );
    println!();
}
