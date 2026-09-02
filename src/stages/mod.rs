//! Bus stages. Each subscribes to the whole [`Envelope`] stream; some also emit
//! (they hold a [`crate::bus::Bus`]). `drive` runs a stage until the shutdown
//! token fires or the bus closes.

pub mod executor;
pub mod listener;
pub mod logger;
pub mod style;

mod listener_sub;

use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::CancellationToken;

use crate::bus::{Envelope, Receiver};

pub trait Stage: Send + 'static {
    async fn on_event(&mut self, env: &Envelope);

    /// The bus dropped `dropped` events because this stage was too slow.
    async fn on_lag(&mut self, _dropped: u64) {}
}

pub async fn drive<S: Stage>(mut stage: S, mut rx: Receiver, shutdown: CancellationToken) {
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            received = rx.recv() => match received {
                Ok(env) => stage.on_event(&env).await,
                Err(RecvError::Lagged(dropped)) => stage.on_lag(dropped).await,
                Err(RecvError::Closed) => break,
            },
        }
    }
}
