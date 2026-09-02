//! One task per chain; inside it, one log subscription per configured target.

use std::sync::Arc;

use alloy::primitives::Address;
use alloy::providers::Provider as _;
use alloy::rpc::types::Filter;
use futures_util::{StreamExt, stream::FuturesUnordered};

use crate::config::watchlist::{Chain, Target};
use crate::contracts;
use crate::event::bus::Sender;
use crate::event::{Detection, Event};
use crate::provider::Provider;

pub struct SubListener {
    chain: String,
    config: Chain,
    provider: Arc<Provider>,
    bus: Sender,
}

impl SubListener {
    pub fn new(chain: String, config: Chain, provider: Arc<Provider>, bus: Sender) -> Self {
        Self {
            chain,
            config,
            provider,
            bus,
        }
    }

    /// Watches every target concurrently; returns once all subscriptions end.
    pub async fn run(self) {
        let watchers: FuturesUnordered<_> =
            self.config.targets.iter().map(|t| self.watch(t)).collect();
        watchers.collect::<()>().await;
    }

    async fn watch(&self, target: &Target) {
        let address: Address = match target.address.parse() {
            Ok(addr) => addr,
            Err(e) => return self.stopped(target, format!("invalid address: {e}")),
        };

        let filter = Filter::new()
            .address(address)
            .event(target.event_signature());
        let subscription = match self.provider.ws.subscribe_logs(&filter).await {
            Ok(sub) => sub,
            Err(e) => return self.stopped(target, format!("subscribe failed: {e}")),
        };

        let _ = self.bus.send(Event::Watching {
            chain: self.chain.clone(),
            source: target.name.clone(),
            kind: target.kind,
            address: address.to_string(),
            signature: target.event_signature().to_string(),
        });

        let mut stream = subscription.into_stream();
        while let Some(log) = stream.next().await {
            match contracts::decode(&log) {
                Some(decoded) => {
                    let token = decoded.token();
                    let metadata = contracts::token_metadata(&self.provider.http, token).await;
                    let _ = self.bus.send(Event::Detection(Detection {
                        chain: self.chain.clone(),
                        kind: target.kind,
                        source: target.name.clone(),
                        event: decoded.name(),
                        token: token.to_string(),
                        block_number: log.block_number,
                        tx_hash: log.transaction_hash.map(|h| h.to_string()),
                        metadata: Some(metadata),
                        payload: decoded.payload(),
                    }));
                }
                None => {
                    let _ = self.bus.send(Event::Undecoded {
                        chain: self.chain.clone(),
                        source: target.name.clone(),
                        topic0: log.topic0().map(|t| t.to_string()),
                    });
                }
            }
        }

        self.stopped(target, "subscription closed".into());
    }

    fn stopped(&self, target: &Target, reason: String) {
        let _ = self.bus.send(Event::WatchStopped {
            chain: self.chain.clone(),
            source: target.name.clone(),
            reason,
        });
    }
}
