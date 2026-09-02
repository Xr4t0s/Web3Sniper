//! Producer stage: fans the watchlist out into one task per chain.

use std::sync::Arc;

use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use super::listener_sub::SubListener;
use crate::bus::{Bus, Event};
use crate::chain::provider::Providers;
use crate::config::watchlist::Watchlist;

pub async fn run(
    watchlist: Watchlist,
    providers: Arc<Providers>,
    bus: Bus,
    shutdown: CancellationToken,
) {
    let mut tasks = JoinSet::new();

    for (chain, config) in watchlist.chains {
        let Some(provider) = providers.get(&chain) else {
            continue; // already reported as ChainDown at connect time
        };
        bus.emit(Event::ChainConnected {
            chain: chain.clone(),
        });
        tasks.spawn(SubListener::new(chain, config, provider, bus.clone(), shutdown.clone()).run());
    }

    while tasks.join_next().await.is_some() {}
}
