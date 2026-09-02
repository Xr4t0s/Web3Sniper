use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use alloy::providers::{DynProvider, Provider as AlloyProvider, ProviderBuilder};

use crate::config::watchlist::Watchlist;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Accès RPC à une chain. Partagé (`Arc`) entre Listener, Executor, etc.
///
/// `http` sert aux appels JSON-RPC (eth_call, eth_sendRawTransaction, ...),
/// `ws` aux souscriptions (`ws.subscribe_logs(&filter)`).
#[derive(Debug)]
pub struct Provider {
    pub chain: String,
    pub http: DynProvider,
    pub ws: DynProvider,
}

impl Provider {
    pub async fn new(chain: String, http_url: String, ws_url: String) -> Result<Self> {
        let http = ProviderBuilder::new().connect(&http_url).await?.erased();
        let ws = ProviderBuilder::new().connect(&ws_url).await?.erased();
        Ok(Self { chain, http, ws })
    }
}

/// Un `Provider` par chain. Construite une fois dans `main`, partagée en `Arc`.
#[derive(Debug, Default)]
pub struct Providers {
    chains: HashMap<String, Arc<Provider>>,
}

impl Providers {
    pub async fn from_watchlist(watchlist: &Watchlist) -> Result<Self> {
        let mut chains = HashMap::new();

        for (name, chain) in &watchlist.chains {
            let provider = Provider::new(
                name.clone(),
                chain.https_rpc_url.clone(),
                chain.wss_rpc_url.clone(),
            )
            .await?;

            chains.insert(name.clone(), Arc::new(provider));
        }

        Ok(Self { chains })
    }

    /// Le provider d'une chain donnée (clone du `Arc`, pas de la connexion).
    pub fn get(&self, chain: &str) -> Option<Arc<Provider>> {
        self.chains.get(chain).cloned()
    }
}
