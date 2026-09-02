//! `watchlist.yaml` — the chains to watch and the contracts on each.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

/// What a watched contract produces, and which downstream logic applies.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Launch,
    Graduation,
}

impl TargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TargetKind::Launch => "launch",
            TargetKind::Graduation => "graduation",
        }
    }
}

/// One contract to watch on a chain. Several targets may share a `kind`
/// (coexisting factory / pool versions).
#[derive(Debug, Deserialize)]
pub struct Target {
    pub kind: TargetKind,
    /// Human label for logs, e.g. `"PonsFamily V2"`.
    pub name: String,
    pub address: String,
    /// Event signature to filter on, e.g. `PoolGraduated(address,uint256,uint256,uint256)`.
    pub event: String,
}

impl Target {
    /// Canonical signature for `Filter::event`: no `event` keyword, no stray
    /// whitespace (both common when pasting from Solidity).
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
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let file =
            std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
        serde_yaml::from_reader(file).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn chain_names(&self) -> Vec<String> {
        self.chains.keys().cloned().collect()
    }
}
