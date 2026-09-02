use std::collections::HashMap;
use std::sync::Arc;

use crate::listener::watchlist::Watchlist;

/// Accès RPC à une chain. Partagé (`Arc`) entre Listener, Executor, etc.
#[derive(Debug)]
pub struct Provider {
    pub chain: String,
    pub http_url: String,
    pub ws_url: String,
    // plus tard: client HTTP réutilisable, pool WS + reconnexion, nonce manager...
}

impl Provider {
    pub fn new(chain: String, http_url: String, ws_url: String) -> Self {
        Provider { chain, http_url, ws_url }
    }

    /// STUB — ouvrir la connexion WS, poser un filtre `eth_subscribe("logs", ...)`
    /// et renvoyer le stream de logs.
    pub async fn subscribe_logs(&self /* , filter */) {
        todo!("WS subscribe sur {}", self.ws_url)
    }

    /// STUB — appel JSON-RPC HTTP (eth_call, eth_sendRawTransaction, ...).
    pub async fn call(&self /* , req */) {
        todo!("HTTP RPC sur {}", self.http_url)
    }
}

/// Un `Provider` par chain. Construite une fois dans `main`, partagée en `Arc`.
#[derive(Debug, Default)]
pub struct Providers {
    chains: HashMap<String, Arc<Provider>>,
}

impl Providers {
    pub fn from_watchlist(watchlist: &Watchlist) -> Self {
        let chains = watchlist
            .chains
            .iter()
            .map(|(name, chain)| {
                let provider = Provider::new(
                    name.clone(),
                    chain.https_rpc_url.clone(),
                    chain.wss_rpc_url.clone(),
                );
                (name.clone(), Arc::new(provider))
            })
            .collect();

        Providers { chains }
    }

    /// Le provider d'une chain donnée (clone du `Arc`, pas de la connexion).
    pub fn get(&self, chain: &str) -> Option<Arc<Provider>> {
        self.chains.get(chain).cloned()
    }
}
