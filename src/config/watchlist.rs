use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct TargetConfig {
    pub deployer: String,
    pub method_id: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct ChainConfig {
    pub listen_launch: bool,
    pub listen_graduation: bool,
}

#[derive(Debug, Deserialize)]
pub struct Chain {
    pub wss_rpc_url: String,
    pub https_rpc_url: String,
    pub launch: Option<TargetConfig>,
    pub graduation: Option<TargetConfig>,

    #[serde(default)]
    pub config: ChainConfig
}

#[derive(Debug, Deserialize)]
pub struct Watchlist {
    pub chains: HashMap<String, Chain>,
}

impl Watchlist {

    pub fn load_config(file_path: &str) -> Result<Watchlist, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(file_path)?;
        let mut watchlist: Watchlist = serde_yaml::from_reader(file)?;

        for chain in watchlist.chains.values_mut() {
            chain.config.listen_launch = chain.launch.is_some();
            chain.config.listen_graduation = chain.graduation.is_some();
        }

        Ok(watchlist)
    }
}