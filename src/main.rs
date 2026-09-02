mod banner;
mod bus;
mod chain;
mod config;
mod events;
mod stages;

use std::sync::Arc;

use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::bus::{Bus, Event};
use crate::chain::provider::Providers;
use crate::config::watchlist::Watchlist;
use crate::stages::executor::Executor;
use crate::stages::logger::Logger;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    banner::print();

    let watchlist = Watchlist::load("watchlist.yaml")?;

    let bus = Bus::new();
    let shutdown = CancellationToken::new();

    let mut tasks = JoinSet::new();
    tasks.spawn(stages::drive(
        Logger::from_env(),
        bus.subscribe(),
        shutdown.clone(),
    ));
    tasks.spawn(stages::drive(
        Executor::new(bus.clone()),
        bus.subscribe(),
        shutdown.clone(),
    ));

    bus.emit(Event::Started {
        chains: watchlist.chain_names(),
    });

    let (providers, failures) = Providers::connect(&watchlist).await;
    for (chain, err) in failures {
        bus.emit(Event::ChainDown {
            chain,
            reason: format!("{err:#}"),
        });
    }
    if providers.is_empty() {
        anyhow::bail!("no chain connected");
    }

    tasks.spawn(stages::listener::run(
        watchlist,
        Arc::new(providers),
        bus.clone(),
        shutdown.clone(),
    ));

    {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            shutdown.cancel();
        });
    }

    while let Some(res) = tasks.join_next().await {
        res?;
    }
    Ok(())
}
