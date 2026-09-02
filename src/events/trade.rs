//! Executor payloads. `TradeIntent` is what the executor *would* submit; while
//! it is a stub, `dry_run` is always true and no transaction is built.

use serde::Serialize;

use crate::bus::num::Dec;
use crate::config::watchlist::TargetKind;

#[derive(Clone, Debug, Serialize)]
pub struct TradeIntent {
    pub chain: String,
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Protocol the signal came from.
    pub source: String,
    pub kind: TargetKind,
    /// `bonding_curve`, `uniswap-v4`, or a DEX name.
    pub venue: String,
    /// Contract the buy would call (curve, pool, or router).
    pub target: String,
    /// Quote asset address, `"native"`, or `"unresolved"`.
    pub quote_token: String,
    /// Size in quote base units. Set only when the quote is native; sizing for
    /// ERC-20 quotes is deferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_in: Option<Dec>,
    pub max_slippage_bps: u32,
    /// Always true until execution is implemented.
    pub dry_run: bool,
}
