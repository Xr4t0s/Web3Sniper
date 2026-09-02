//! Fans the watchlist out into one task per chain.

pub mod sublistener;

use std::sync::Arc;

use tokio::task::JoinSet;

use crate::config::watchlist::Watchlist;
use crate::event::Event;
use crate::event::bus::Sender;
use crate::provider::Providers;
use sublistener::SubListener;

pub struct Listener {
    watchlist: Watchlist,
    providers: Arc<Providers>,
    bus: Sender,
}

impl Listener {
    pub fn new(watchlist: Watchlist, providers: Providers, bus: Sender) -> Self {
        Self {
            watchlist,
            providers: Arc::new(providers),
            bus,
        }
    }

    /// Spawns a [`SubListener`] per chain and runs until all of them stop.
    pub async fn run(self) {
        let mut tasks = JoinSet::new();

        for (chain, config) in self.watchlist.chains {
            let Some(provider) = self.providers.get(&chain) else {
                continue; // already reported as ChainDown at connect time
            };
            let _ = self.bus.send(Event::ChainConnected {
                chain: chain.clone(),
            });
            tasks.spawn(SubListener::new(chain, config, provider, self.bus.clone()).run());
        }

        while tasks.join_next().await.is_some() {}
    }
}
