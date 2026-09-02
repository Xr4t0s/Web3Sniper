pub mod sublistener;

use std::sync::Arc;

use tokio::sync::broadcast::Sender;
use tokio::task::JoinHandle;

use crate::config::watchlist::Watchlist;
use crate::event::{Event, ListenerEvent};
use crate::provider::Providers;
use sublistener::SubListener;

pub struct Listener {
    pub watchlist: Watchlist,
    pub providers: Arc<Providers>,
    pub tx: Sender<Event>,
}

impl Listener {
    pub fn new(watchlist: Watchlist, providers: Arc<Providers>, tx: Sender<Event>) -> Self {
        Listener { watchlist, providers, tx }
    }

    /// Spawne une sub-listener par chain, émet `Ready`, puis attend les tasks.
    pub async fn start(self) {
        let mut handles: Vec<JoinHandle<()>> = Vec::new();

        for (name, chain) in self.watchlist.chains {
            let provider = self
                .providers
                .get(&name)
                .expect("provider manquant pour une chain de la watchlist");
            let sublistener = SubListener::new(name, chain, provider, self.tx.clone());
            handles.push(tokio::spawn(sublistener.run()));
        }

        let _ = self.tx.send(Event::Listener(ListenerEvent::Ready));

        for handle in handles {
            let _ = handle.await;
        }
    }
}
