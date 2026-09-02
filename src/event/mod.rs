pub mod contracts;

/// Données communes à toute détection on-chain.
#[derive(Clone, Debug)]
pub struct Detection {
    pub chain: String,
    pub tx_hash: String,
    pub block_number: u64,
    /// Adresse du contrat déployé / concerné.
    pub address: String,
    pub deployer: String,
    pub method_id: String,
}

/// Racine de tout ce qui transite dans le `broadcast::Sender`.
/// Un variant par sous-système : les futurs modules ajoutent le leur ici.
#[derive(Clone, Debug)]
pub enum Event {
    Listener(ListenerEvent),
}

/// Events émis par le listener et ses sub-listeners.
#[derive(Clone, Debug)]
pub enum ListenerEvent {
    /// Toutes les sub-listeners ont été spawnées.
    Ready,
    ListeningLaunches { chain: String },
    ListeningGraduations { chain: String },
    /// Une chain vient de démarrer son écoute.
    ChainUp { chain: String },
    /// Une chain a coupé / échoué.
    ChainDown { chain: String, err: String },
    /// Un nouveau token a été lancé.
    Launch(Detection),
    /// Un token a "gradué".
    Graduation(Detection),
}

impl Event {
    /// Raccourci pour les consumers qui ne s'intéressent qu'au listener.
    pub fn as_listener(&self) -> Option<&ListenerEvent> {
        match self {
            Event::Listener(e) => Some(e),
        }
    }
}
