//! Renders the bus to stdout. Human-readable coloured lines by default; the raw
//! envelope as one JSON object per line when `DEBUG=true`. The only stage that
//! writes stdout.

use serde_json::json;

use super::Stage;
use super::style::{bold, bold_cyan, cyan, dim, green, red, short_hex, yellow};
use crate::bus::{Envelope, Level};
use crate::events::Event;

pub struct Logger {
    json: bool,
}

impl Logger {
    pub fn from_env() -> Self {
        let json = std::env::var("DEBUG")
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
        Self { json }
    }
}

impl Stage for Logger {
    async fn on_event(&mut self, env: &Envelope) {
        if self.json {
            println!("{}", serde_json::to_string(env).unwrap_or_default());
        } else {
            human_line(level_tag(env.level), &env.topic, &summary(&env.payload));
        }
    }

    async fn on_lag(&mut self, dropped: u64) {
        let topic = "bus.logger.lagged";
        if self.json {
            println!(
                "{}",
                json!({ "level": "warn", "topic": topic, "dropped": dropped })
            );
        } else {
            human_line(
                level_tag(Level::Warn),
                topic,
                &format!("fell behind, dropped {dropped} events"),
            );
        }
    }
}

fn human_line(tag: String, topic: &str, summary: &str) {
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
        Event::Resubscribed {
            source, attempt, ..
        } => {
            format!("{source}: back after {attempt} attempt(s)")
        }
        Event::Alive {
            last_log_age_secs, ..
        } => match last_log_age_secs {
            Some(age) => format!("last log {age}s ago"),
            None => "no log yet".into(),
        },
        Event::Undecoded { source, topic0, .. } => {
            let t = topic0.as_deref().unwrap_or("none");
            format!("{source}: log did not match ABI (topic0 {})", short_hex(t))
        }
        Event::WatchStopped { source, reason, .. } => format!("{source}: {reason}"),
        Event::Detection(d) => {
            let sym = d.token.symbol.as_deref().unwrap_or("?");
            let name = d.token.name.as_deref().unwrap_or("?");
            let mut line = format!(
                "{} {}  token {}",
                cyan(&d.protocol),
                bold(&format!("{sym} ({name})")),
                dim(&short_hex(&d.token.address)),
            );
            line.push_str(&dim(&format!("  block {}", d.block_number)));
            line
        }
        Event::TradeIntent(i) => {
            let sym = i.symbol.as_deref().unwrap_or("?");
            let size = i
                .amount_in
                .as_ref()
                .map(|a| a.to_string())
                .unwrap_or_else(|| "?".into());
            format!(
                "{} {} on {} {} {}",
                bold(sym),
                dim(&format!("buy {size} {}", i.quote_token)),
                i.venue,
                dim(&short_hex(&i.target)),
                dim("[dry-run]"),
            )
        }
        Event::TradeSkipped { source, reason, .. } => format!("{source}: {reason}"),
    }
}
