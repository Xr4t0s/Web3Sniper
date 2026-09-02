mod config;
mod event;
mod listener;
mod logger;
mod provider;
mod executor;

use std::sync::Arc;

use tokio::sync::broadcast;

use config::watchlist::Watchlist;
use provider::Providers;

#[tokio::main]
async fn main() {
    let (tx, rx) = broadcast::channel::<event::Event>(1024);

    let watchlist = Watchlist::load_config("watchlist.yaml").unwrap();
    let providers = Arc::new(Providers::from_watchlist(&watchlist).await.unwrap());

    let logger_handle = tokio::spawn(async move {
        logger::Logger::start(logger::Level::Info, rx).await;
    });

    let listener_handle = tokio::spawn(async move {
        listener::Listener::new(watchlist, providers, tx).start().await;
    });

    let _ = tokio::join!(logger_handle, listener_handle);
}
