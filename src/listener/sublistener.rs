use std::sync::Arc;

use tokio::sync::broadcast::Sender;

use super::watchlist::Chain;
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

    /// Point d'entrée de la task.
    pub async fn run(self) {
        let _ = self.tx.send(Event::Listener(ListenerEvent::ChainUp {
            chain: self.name.clone(),
        }));

        // Les deux watchers tournent en concurrence sur la même task :
        // `run` ne rend la main que quand les deux sont terminés.
        let launch = async {
            if self.chain.config.listen_launch {
                self.watch_launch().await;
            }
        };
        let graduation = async {
            if self.chain.config.listen_graduation {
                self.watch_graduation().await;
            }
        };

        tokio::join!(launch, graduation);
    }

    /// STUB — à implémenter : connexion `self.chain.wss_rpc_url`, filtre logs sur
    /// `self.chain.launch`, construire un `Detection` puis
    /// `self.tx.send(Event::Listener(ListenerEvent::Launch(detection)))`.
    async fn watch_launch(&self) {
        let _ = self.tx.send(Event::Listener(ListenerEvent::ListeningLaunches { chain: self.name.clone() }));
        std::future::pending::<()>().await;
    }

    /// STUB — idem via `self.chain.graduation` →
    /// `Event::Listener(ListenerEvent::Graduation(detection))`.
    async fn watch_graduation(&self) {
        let _ = self.tx.send(Event::Listener(ListenerEvent::ListeningGraduations { chain: self.name.clone() }));
        std::future::pending::<()>().await;
    }
}
