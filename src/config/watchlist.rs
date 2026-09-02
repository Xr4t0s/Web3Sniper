use serde::Deserialize;
use std::collections::HashMap;

/// Ce qu'un contrat surveillé produit — détermine la logique appliquée en aval.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Launch,
    Graduation,
}

/// Un contrat à surveiller sur une chain : son adresse et l'event à filtrer.
/// Plusieurs `Target` de même `kind` sont permis (versions de factory/pool
/// coexistantes).
#[derive(Debug, Deserialize)]
pub struct Target {
    pub kind: TargetKind,
    /// Nom lisible du protocole/contrat, pour les logs.
    pub name: String,
    pub address: String,
    /// Signature humaine de l'event, ex. `PoolGraduated(address,uint256,uint256,uint256)`.
    pub event: String,
}

impl Target {
    /// Signature canonique acceptée par `Filter::event` : sans le mot-clé
    /// `event` ni les espaces parasites qu'on copie souvent depuis le code Solidity.
    pub fn event_signature(&self) -> &str {
        self.event
            .trim()
            .strip_prefix("event ")
            .unwrap_or(&self.event)
            .trim()
    }
}

#[derive(Debug, Deserialize)]
pub struct Chain {
    pub wss_rpc_url: String,
    pub https_rpc_url: String,
    #[serde(default)]
    pub targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
pub struct Watchlist {
    pub chains: HashMap<String, Chain>,
}

impl Watchlist {
    pub fn load_config(file_path: &str) -> Result<Watchlist, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(file_path)?;
        let watchlist: Watchlist = serde_yaml::from_reader(file)?;

        Ok(watchlist)
    }
}
