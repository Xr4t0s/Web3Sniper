//! The internal event bus: a broadcast channel of [`Envelope`]s.
//!
//! Producers hold a [`Bus`] and call [`Bus::emit`]; every stage that subscribes
//! gets an `Arc<Envelope>`. The envelope wraps a typed [`Event`] payload with
//! routing metadata (§2 of `SPECS.md`). `main` owns one `Bus` for the whole
//! process; shutdown is an explicit `CancellationToken`, not the channel
//! closing, so stages may hold a `Bus` and emit freely.

pub mod num;

use std::sync::Arc;

use serde::Serialize;
use tokio::sync::broadcast;

pub use crate::events::Event;

/// Schema version of the envelope + payloads. Bump on a breaking change.
pub const SCHEMA: u32 = 1;

/// Broadcast buffer. A slow stage lags (and says so) rather than blocking.
const CAPACITY: usize = 4096;

pub type Receiver = broadcast::Receiver<Arc<Envelope>>;

/// Severity, for stages that rank or colour events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// One bus message: routing metadata + a flattened typed payload.
#[derive(Clone, Debug, Serialize)]
pub struct Envelope {
    pub schema: u32,
    pub ts: String,
    pub subsystem: &'static str,
    pub topic: String,
    pub level: Level,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caused_by: Option<String>,
    #[serde(flatten)]
    pub payload: Event,
}

impl Envelope {
    fn wrap(payload: Event, caused_by: Option<String>) -> Self {
        Self {
            schema: SCHEMA,
            ts: chrono::Local::now().to_rfc3339(),
            subsystem: payload.subsystem(),
            topic: payload.topic(),
            level: payload.level(),
            trace_id: payload.trace_id(),
            event_id: payload.event_id(),
            caused_by,
            payload,
        }
    }
}

/// A cloneable handle to publish on the bus.
#[derive(Clone)]
pub struct Bus {
    tx: broadcast::Sender<Arc<Envelope>>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            tx: broadcast::channel(CAPACITY).0,
        }
    }

    pub fn subscribe(&self) -> Receiver {
        self.tx.subscribe()
    }

    /// Publish an event. Never blocks; drops silently if there are no receivers.
    pub fn emit(&self, event: Event) {
        let _ = self.tx.send(Arc::new(Envelope::wrap(event, None)));
    }

    /// Publish an event derived from an earlier one. Used by the enrichment
    /// stages (tracker, opportunist, trader) to set `caused_by`.
    #[allow(dead_code)]
    pub fn emit_caused(&self, event: Event, caused_by: impl Into<String>) {
        let _ = self
            .tx
            .send(Arc::new(Envelope::wrap(event, Some(caused_by.into()))));
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}
