//! The internal event bus.
//!
//! Every subsystem publishes typed [`Event`]s here instead of printing. Any
//! number of consumers subscribe (see [`crate::consumer`]): the logger renders
//! them for a human, and later a dashboard feed, a spreadsheet writer or a
//! Discord/Telegram relay can consume the exact same stream. Events are
//! `Serialize`, so a consumer that needs a machine payload just serialises them.

use serde::Serialize;

use crate::config::watchlist::TargetKind;
use crate::contracts::TokenMetadata;

/// Bus wiring. Producers hold a [`Sender`], consumers a [`Receiver`].
pub mod bus {
    use super::Event;
    use tokio::sync::broadcast;

    pub type Sender = broadcast::Sender<Event>;
    pub type Receiver = broadcast::Receiver<Event>;

    /// Buffer sized so a briefly slow consumer lags (and says so) rather than
    /// stalling producers.
    const CAPACITY: usize = 4096;

    pub fn channel() -> Sender {
        broadcast::channel(CAPACITY).0
    }
}

/// Severity, for consumers that rank or colour events. Others ignore it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// Anything worth reacting to. One variant per fact, not per log line.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Startup: watchlist parsed, these chains are configured.
    Started { chains: Vec<String> },
    /// A chain's RPC providers are connected.
    ChainConnected { chain: String },
    /// A chain was skipped: its RPC endpoints could not be reached.
    ChainDown { chain: String, reason: String },
    /// A target's log subscription is live.
    Watching {
        chain: String,
        source: String,
        kind: TargetKind,
        address: String,
        signature: String,
    },
    /// A matching log could not be decoded with the configured ABI.
    Undecoded {
        chain: String,
        source: String,
        topic0: Option<String>,
    },
    /// A subscription could not start or has ended.
    WatchStopped {
        chain: String,
        source: String,
        reason: String,
    },
    /// A launch or graduation was detected.
    Detection(Detection),
}

/// A confirmed on-chain launch or graduation.
#[derive(Clone, Debug, Serialize)]
pub struct Detection {
    pub chain: String,
    pub kind: TargetKind,
    /// Configured target name, e.g. `"PonsFamily V2"`.
    pub source: String,
    /// Solidity event name, e.g. `"TokenLaunched"`.
    pub event: &'static str,
    pub token: String,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<TokenMetadata>,
    /// Decoded event fields, shape depends on `event`.
    pub payload: serde_json::Value,
}

impl Event {
    /// Dotted routing key, e.g. `chains.rh.launch.TokenLaunched`.
    pub fn topic(&self) -> String {
        match self {
            Event::Started { .. } => "app.started".into(),
            Event::ChainConnected { chain } => format!("chains.{chain}.connected"),
            Event::ChainDown { chain, .. } => format!("chains.{chain}.down"),
            Event::Watching { chain, kind, .. } => {
                format!("chains.{chain}.{}.watching", kind.as_str())
            }
            Event::Undecoded { chain, .. } => format!("chains.{chain}.undecoded"),
            Event::WatchStopped { chain, .. } => format!("chains.{chain}.watch_stopped"),
            Event::Detection(d) => {
                format!("chains.{}.{}.{}", d.chain, d.kind.as_str(), d.event)
            }
        }
    }

    pub fn level(&self) -> Level {
        match self {
            Event::Undecoded { .. } => Level::Warn,
            Event::WatchStopped { .. } | Event::ChainDown { .. } => Level::Error,
            _ => Level::Info,
        }
    }
}
