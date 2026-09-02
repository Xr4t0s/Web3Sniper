//! One task per chain: a reconnecting log subscription per configured target,
//! plus a liveness heartbeat. Each matching log spawns a detached enrichment
//! task (§8.4 of `SPECS.md`) so the stream reader never blocks.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use alloy::primitives::Address;
use alloy::providers::Provider as _;
use alloy::rpc::types::Filter;
use futures_util::{StreamExt, future::join_all};
use tokio_util::sync::CancellationToken;

use crate::bus::{Bus, Event};
use crate::chain::provider::Provider;
use crate::chain::{contracts, enrich};
use crate::config::watchlist::{Chain, Target};

const HEARTBEAT: Duration = Duration::from_secs(30);
const BACKOFF_MAX: Duration = Duration::from_secs(30);
/// A subscription that lasted at least this long resets the backoff.
const HEALTHY_AFTER: Duration = Duration::from_secs(60);

pub struct SubListener {
    chain: String,
    config: Chain,
    provider: Arc<Provider>,
    bus: Bus,
    shutdown: CancellationToken,
    last_log: Arc<AtomicU64>,
}

impl SubListener {
    pub fn new(
        chain: String,
        config: Chain,
        provider: Arc<Provider>,
        bus: Bus,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            chain,
            config,
            provider,
            bus,
            shutdown,
            last_log: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn run(self) {
        let watchers = join_all(self.config.targets.iter().map(|t| self.watch(t)));
        tokio::join!(watchers, self.heartbeat());
    }

    /// Subscribe → stream → on drop, back off and resubscribe. Runs until shutdown.
    async fn watch(&self, target: &Target) {
        let address: Address = match target.address.parse() {
            Ok(addr) => addr,
            Err(e) => return self.emit_stopped(target, format!("invalid address: {e}")),
        };
        let filter = Filter::new()
            .address(address)
            .event(target.event_signature());

        let mut attempt: u32 = 0;
        let mut backoff = Duration::from_secs(1);

        loop {
            if self.shutdown.is_cancelled() {
                return;
            }

            match self.provider.ws.subscribe_logs(&filter).await {
                Ok(subscription) => {
                    if attempt == 0 {
                        self.bus.emit(Event::Watching {
                            chain: self.chain.clone(),
                            source: target.name.clone(),
                            kind: target.kind,
                            address: address.to_string(),
                            signature: target.event_signature().to_string(),
                        });
                    } else {
                        self.bus.emit(Event::Resubscribed {
                            chain: self.chain.clone(),
                            source: target.name.clone(),
                            attempt,
                        });
                    }

                    let started = Instant::now();
                    let mut stream = subscription.into_stream();
                    loop {
                        tokio::select! {
                            _ = self.shutdown.cancelled() => return,
                            next = stream.next() => match next {
                                Some(log) => {
                                    self.last_log.store(now_secs(), Ordering::Relaxed);
                                    self.handle(target, log);
                                }
                                None => break,
                            },
                        }
                    }

                    self.emit_stopped(target, "subscription dropped".into());
                    if started.elapsed() >= HEALTHY_AFTER {
                        attempt = 0;
                        backoff = Duration::from_secs(1);
                        continue;
                    }
                }
                Err(e) => self.emit_stopped(target, format!("subscribe failed: {e}")),
            }

            attempt += 1;
            tokio::select! {
                _ = self.shutdown.cancelled() => return,
                _ = tokio::time::sleep(backoff) => {}
            }
            backoff = (backoff * 2).min(BACKOFF_MAX);
        }
    }

    fn handle(&self, target: &Target, log: alloy::rpc::types::Log) {
        let Some(decoded) = contracts::decode(&log) else {
            self.bus.emit(Event::Undecoded {
                chain: self.chain.clone(),
                source: target.name.clone(),
                topic0: log.topic0().map(|t| t.to_string()),
            });
            return;
        };

        let bus = self.bus.clone();
        let provider = self.provider.clone();
        let chain = self.chain.clone();
        let protocol = target.name.clone();
        let kind = target.kind;
        tokio::spawn(async move {
            let detection =
                enrich::enrich(&chain, &protocol, kind, &decoded, &log, &provider).await;
            bus.emit(Event::Detection(Box::new(detection)));
        });
    }

    async fn heartbeat(&self) {
        let mut tick = tokio::time::interval(HEARTBEAT);
        tick.tick().await; // fire the first one a full interval from now
        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => return,
                _ = tick.tick() => {
                    let last = self.last_log.load(Ordering::Relaxed);
                    let age = (last != 0).then(|| now_secs().saturating_sub(last));
                    self.bus.emit(Event::Alive {
                        chain: self.chain.clone(),
                        last_log_age_secs: age,
                    });
                }
            }
        }
    }

    fn emit_stopped(&self, target: &Target, reason: String) {
        self.bus.emit(Event::WatchStopped {
            chain: self.chain.clone(),
            source: target.name.clone(),
            reason,
        });
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
