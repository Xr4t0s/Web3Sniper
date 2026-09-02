mod banner;
mod config;
mod consumer;
mod contracts;
mod event;
mod listener;
mod provider;

use tokio::task::JoinSet;

use crate::config::watchlist::Watchlist;
use crate::consumer::logger::Logger;
use crate::event::{Event, bus};
use crate::listener::Listener;
use crate::provider::Providers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    banner::print();

    let watchlist = Watchlist::load("watchlist.yaml")?;

    let bus = bus::channel();
    let mut tasks = JoinSet::new();
    tasks.spawn(consumer::drive(Logger, bus.subscribe()));

    let _ = bus.send(Event::Started { chains: watchlist.chain_names() });

    let (providers, failures) = Providers::connect(&watchlist).await;
    for (chain, err) in failures {
        let _ = bus.send(Event::ChainDown { chain, reason: format!("{err:#}") });
    }
    if providers.is_empty() {
        anyhow::bail!("no chain connected");
    }

    tasks.spawn(Listener::new(watchlist, providers, bus.clone()).run());
    drop(bus); // consumers exit once every producer has.

    while let Some(res) = tasks.join_next().await {
        res?;
    }
    Ok(())
}
