//! RPC access, one [`Provider`] per chain.

use std::collections::HashMap;
use std::sync::Arc;

use alloy::providers::{DynProvider, Provider as _, ProviderBuilder};
use anyhow::Context;
use futures_util::future::join_all;

use crate::config::watchlist::Watchlist;

/// Connections to one chain, shared (`Arc`) across the listener and future
/// consumers. `http` serves JSON-RPC calls (`eth_call`, `eth_sendRawTransaction`);
/// `ws` serves subscriptions (`subscribe_logs`).
#[derive(Debug)]
pub struct Provider {
    pub http: DynProvider,
    pub ws: DynProvider,
}

impl Provider {
    async fn connect(chain: &str, http_url: &str, ws_url: &str) -> anyhow::Result<Self> {
        let http = ProviderBuilder::new()
            .connect(http_url)
            .await
            .with_context(|| format!("{chain}: HTTP connect"))?
            .erased();
        let ws = ProviderBuilder::new()
            .connect(ws_url)
            .await
            .with_context(|| format!("{chain}: WS connect"))?
            .erased();
        Ok(Self { http, ws })
    }
}

/// One [`Provider`] per chain, built once at startup.
#[derive(Debug, Default)]
pub struct Providers {
    chains: HashMap<String, Arc<Provider>>,
}

impl Providers {
    /// Connects every chain concurrently and independently. Returns the
    /// providers that came up, plus `(chain, error)` for those that did not —
    /// one bad endpoint never blocks the others.
    pub async fn connect(watchlist: &Watchlist) -> (Self, Vec<(String, anyhow::Error)>) {
        let connects = watchlist.chains.iter().map(|(name, chain)| async move {
            let result = Provider::connect(name, &chain.https_rpc_url, &chain.wss_rpc_url).await;
            (name.clone(), result)
        });

        let mut chains = HashMap::new();
        let mut failures = Vec::new();
        for (name, result) in join_all(connects).await {
            match result {
                Ok(provider) => {
                    chains.insert(name, Arc::new(provider));
                }
                Err(e) => failures.push((name, e)),
            }
        }
        (Self { chains }, failures)
    }

    pub fn is_empty(&self) -> bool {
        self.chains.is_empty()
    }

    /// The provider for a chain (clones the `Arc`, not the connection).
    pub fn get(&self, chain: &str) -> Option<Arc<Provider>> {
        self.chains.get(chain).cloned()
    }
}
