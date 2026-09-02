//! Typed bus payloads. One variant per fact; the emitting subsystem is the
//! first segment of [`Event::topic`]. Downstream stages (tracker, opportunist,
//! trader) add their own variants here as they are built.

pub mod detection;
pub mod trade;

use serde::Serialize;

pub use detection::Detection;
pub use trade::TradeIntent;

use crate::bus::Level;
use crate::config::watchlist::TargetKind;

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
    /// A dropped subscription was re-established.
    Resubscribed {
        chain: String,
        source: String,
        attempt: u32,
    },
    /// Liveness tick for a chain's watchers. `None` age = no log seen yet.
    Alive {
        chain: String,
        last_log_age_secs: Option<u64>,
    },
    /// A matching log could not be decoded with the configured ABI.
    Undecoded {
        chain: String,
        source: String,
        topic0: Option<String>,
    },
    /// A subscription failed to start, or ended for good.
    WatchStopped {
        chain: String,
        source: String,
        reason: String,
    },
    /// A launch or graduation, enriched with token / market state.
    Detection(Box<Detection>),
    /// The executor would buy on this signal (stub: `dry_run` always true).
    TradeIntent(Box<TradeIntent>),
    /// The executor passed on a signal, with the reason.
    TradeSkipped {
        chain: String,
        token: String,
        source: String,
        kind: TargetKind,
        reason: String,
    },
}

impl Event {
    /// First segment of the topic and the envelope `subsystem` field.
    pub fn subsystem(&self) -> &'static str {
        match self {
            Event::Started { .. } => "app",
            Event::TradeIntent(_) | Event::TradeSkipped { .. } => "executor",
            _ => "listener",
        }
    }

    /// Dotted routing key `<subsystem>.<chain>.<kind>.<detail>`.
    pub fn topic(&self) -> String {
        match self {
            Event::Started { .. } => "app.started".into(),
            Event::ChainConnected { chain } => format!("listener.{chain}.connected"),
            Event::ChainDown { chain, .. } => format!("listener.{chain}.down"),
            Event::Watching { chain, kind, .. } => {
                format!("listener.{chain}.{}.watching", kind.as_str())
            }
            Event::Resubscribed { chain, .. } => format!("listener.{chain}.resubscribed"),
            Event::Alive { chain, .. } => format!("listener.{chain}.alive"),
            Event::Undecoded { chain, .. } => format!("listener.{chain}.undecoded"),
            Event::WatchStopped { chain, .. } => format!("listener.{chain}.watch_stopped"),
            Event::Detection(d) => {
                format!("listener.{}.{}.{}", d.chain, d.kind.as_str(), d.event)
            }
            Event::TradeIntent(i) => {
                format!("executor.{}.{}.intent", i.chain, i.kind.as_str())
            }
            Event::TradeSkipped { chain, kind, .. } => {
                format!("executor.{chain}.{}.skip", kind.as_str())
            }
        }
    }

    pub fn level(&self) -> Level {
        match self {
            Event::Undecoded { .. } => Level::Warn,
            Event::ChainDown { .. } | Event::WatchStopped { .. } => Level::Error,
            _ => Level::Info,
        }
    }

    /// `<chain>:<token>` — the per-token join key, set on detections.
    pub fn trace_id(&self) -> Option<String> {
        match self {
            Event::Detection(d) => Some(format!("{}:{}", d.chain, d.token.address.to_lowercase())),
            Event::TradeIntent(i) => Some(format!("{}:{}", i.chain, i.token.to_lowercase())),
            Event::TradeSkipped { chain, token, .. } => {
                Some(format!("{chain}:{}", token.to_lowercase()))
            }
            _ => None,
        }
    }

    /// `<chain>:<tx>:<log_index>` — dedup identity, set on detections.
    pub fn event_id(&self) -> Option<String> {
        match self {
            Event::Detection(d) => Some(format!("{}:{}:{}", d.chain, d.tx_hash, d.log_index)),
            _ => None,
        }
    }
}
