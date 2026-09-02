//! Bus consumers. Each one subscribes to the full [`Event`] stream and does its
//! own thing with it: [`logger`] renders it for a human; future ones may push to
//! a dashboard socket, a spreadsheet, or a chat webhook.

pub mod logger;
pub mod style;

use tokio::sync::broadcast::error::RecvError;

use crate::event::Event;
use crate::event::bus::Receiver;

/// A sink for bus events.
pub trait Consumer: Send + 'static {
    async fn on_event(&mut self, event: &Event);

    /// The bus dropped `dropped` events because this consumer was too slow.
    async fn on_lag(&mut self, _dropped: u64) {}
}

/// Drives a consumer until the bus closes (every producer dropped).
pub async fn drive<C: Consumer>(mut consumer: C, mut rx: Receiver) {
    loop {
        match rx.recv().await {
            Ok(event) => consumer.on_event(&event).await,
            Err(RecvError::Lagged(dropped)) => consumer.on_lag(dropped).await,
            Err(RecvError::Closed) => break,
        }
    }
}
