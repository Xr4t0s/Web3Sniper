//! Renders the bus to stdout, one coloured line per event. The only place in
//! the program that writes to stdout.

use super::Consumer;
use super::style::{bold, bold_cyan, cyan, dim, green, red, short_hex, yellow};
use crate::event::{Event, Level};

pub struct Logger;

impl Consumer for Logger {
    async fn on_event(&mut self, event: &Event) {
        line(level_tag(event.level()), &event.topic(), &summary(event));
    }

    async fn on_lag(&mut self, dropped: u64) {
        line(
            level_tag(Level::Warn),
            "bus.logger.lagged",
            &format!("fell behind, dropped {dropped} events"),
        );
    }
}

fn line(tag: String, topic: &str, summary: &str) {
    let ts = chrono::Local::now().format("%H:%M:%S%.3f");
    println!(
        "{} {} {} {}",
        dim(&ts.to_string()),
        tag,
        bold_cyan(topic),
        summary
    );
}

fn level_tag(level: Level) -> String {
    match level {
        Level::Info => green("INFO "),
        Level::Warn => yellow("WARN "),
        Level::Error => red("ERROR"),
    }
}

fn summary(event: &Event) -> String {
    match event {
        Event::Started { chains } => format!("watching {}", chains.join(", ")),
        Event::ChainConnected { chain } => format!("{chain}: RPC connected"),
        Event::ChainDown { chain, reason } => format!("{chain}: {reason}"),
        Event::Watching {
            address, signature, ..
        } => {
            format!("{} {}", dim(&short_hex(address)), signature)
        }
        Event::Undecoded { source, topic0, .. } => {
            let t = topic0.as_deref().unwrap_or("none");
            format!("{source}: log did not match ABI (topic0 {})", short_hex(t))
        }
        Event::WatchStopped { source, reason, .. } => format!("{source}: {reason}"),
        Event::Detection(d) => {
            let label = d
                .metadata
                .as_ref()
                .map(|m| m.label())
                .unwrap_or_else(|| "?".into());
            let mut line = format!(
                "{} {}  token {}",
                cyan(&d.source),
                bold(&label),
                dim(&short_hex(&d.token)),
            );
            if let Some(block) = d.block_number {
                line.push_str(&dim(&format!("  block {block}")));
            }
            line
        }
    }
}
