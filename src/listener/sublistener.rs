use std::sync::Arc;

use alloy::primitives::Address;
use alloy::providers::Provider as AlloyProvider;
use alloy::rpc::types::Filter;
use futures_util::{stream::FuturesUnordered, StreamExt};
use tokio::sync::broadcast::Sender;

use crate::config::watchlist::{Chain, Target, TargetKind};
use crate::event::contracts;
use crate::event::{Event, ListenerEvent};
use crate::provider::Provider;

/// Écoute une seule chain. Une task tokio par instance (`self` owned → `'static`).
pub struct SubListener {
    pub name: String,
    pub chain: Chain,
    pub provider: Arc<Provider>,
    pub tx: Sender<Event>,
}

impl SubListener {
    pub fn new(name: String, chain: Chain, provider: Arc<Provider>, tx: Sender<Event>) -> Self {
        SubListener { name, chain, provider, tx }
    }

    /// Point d'entrée de la task : un watcher par `target`, tous en concurrence.
    /// `run` ne rend la main que quand tous les watchers sont terminés.
    pub async fn run(self) {
        let _ = self.tx.send(Event::Listener(ListenerEvent::ChainUp {
            chain: self.name.clone(),
        }));

        let watchers: FuturesUnordered<_> =
            self.chain.targets.iter().map(|t| self.watch(t)).collect();
        watchers.collect::<()>().await;
    }

    /// STUB — souscrit aux logs de `target.address` filtrés sur `target.event`,
    /// et pour l'instant se contente de les logger. À terme : construire un
    /// `Detection` et l'émettre via `ListenerEvent::Launch` / `Graduation`.
    async fn watch(&self, target: &Target) {
        let label = format!("{} / {} ({:?})", self.name, target.name, target.kind);

        let address: Address = target
            .address
            .parse()
            .unwrap_or_else(|_| panic!("{label}: adresse invalide: {}", target.address));
        
        let filter = Filter::new().address(address).event(target.event_signature());

        let subscription = self
            .provider
            .ws
            .subscribe_logs(&filter)
            .await
            .unwrap_or_else(|e| panic!("{label}: subscribe échoué: {e}"));

        println!("[{label}] écoute {address}");

        let mut stream = subscription.into_stream();

        while let Some(log) = stream.next().await {
            match contracts::decode(&log) {
                Some(decoded) => {
                    let token = decoded.token();
                    let meta = contracts::token_metadata(&self.provider.http, token).await;
                    println!("[{label}] {} {token} — {decoded:?}", meta.label());
                }
                None => {
                    println!("[{label}] log non décodé (topic0/ABI inattendu): {log:?}");
                }
            }
        }
    }
}
