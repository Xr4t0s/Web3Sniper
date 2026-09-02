//! The `Detection` payload — §4 of `SPECS.md`. Emitted by the listener's
//! enrichment task. Every field beyond the decoded log is best-effort: a failed
//! view call leaves it `None`.

use serde::Serialize;

use crate::bus::num::Dec;
use crate::config::watchlist::TargetKind;

#[derive(Clone, Debug, Serialize)]
pub struct Detection {
    pub chain: String,
    pub protocol: String,
    pub kind: TargetKind,
    pub event: &'static str,

    pub token: TokenInfo,
    pub creator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launcher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_buy: Option<DevBuy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_token: Option<TokenRef>,
    pub economics: Economics,
    pub market: MarketKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curve: Option<CurveState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool: Option<PoolState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valuation: Option<Valuation>,

    pub tx_hash: String,
    pub block_number: u64,
    pub log_index: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_timestamp: Option<u64>,
    pub observed_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenInfo {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_supply: Option<Dec>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenRef {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DevBuy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<Dec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<Dec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pct_bps: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Economics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graduation_threshold: Option<Dec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phantom_quote: Option<Dec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curve_fee_bps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_tax_bps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anti_snipe: Option<AntiSnipe>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snipe_tax: Option<SnipeTax>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AntiSnipe {
    pub max_wallet_bps: Option<u32>,
    pub max_tx_bps: Option<u32>,
    pub restriction_end_block: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SnipeTax {
    pub start_bps: Option<u32>,
    pub decay_secs: Option<u32>,
    pub launched_at: Option<u64>,
}

/// Where the token trades right now. `bonding_curve` before graduation, `pool`
/// after (and for protocols that launch straight into a DEX). The matching
/// `curve` / `pool` object on [`Detection`] carries the detail.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketKind {
    BondingCurve,
    Pool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CurveState {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_quote_reserve: Option<Dec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_reserve: Option<Dec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_bps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_native: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub venue: Option<String>,
    /// Pool address, or the Uniswap V4 pool id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_id: Option<Dec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub router: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_token: Option<Dec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_quote: Option<Dec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_native: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Valuation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_native: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidity_native: Option<Dec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fdv_native: Option<Dec>,
}
