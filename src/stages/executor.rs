//! STUB executor. Reacts to a `Detection`, applies a small hardcoded gate, and
//! emits a dry-run `TradeIntent` or a `TradeSkipped` on the bus — no
//! transaction, no signing, no RPC.
//!
//! The gate is a placeholder for the opportunist stage. Once that exists this
//! consumes `opportunity.*` instead and the gate here is deleted.

use alloy::primitives::U256;

use super::Stage;
use crate::bus::num::Dec;
use crate::bus::{Bus, Envelope};
use crate::events::detection::{Detection, MarketKind};
use crate::events::{Event, TradeIntent};

/// Dev holding more than this share of supply → skip (they dump on you).
const MAX_DEV_BUY_BPS: u32 = 2500; // 25%
/// Buy size in native base units, used only when the quote asset is native.
const BUY_SIZE_NATIVE: u128 = 50_000_000_000_000_000; // 0.05
const MAX_SLIPPAGE_BPS: u32 = 1500; // 15%
const NATIVE: &str = "0x0000000000000000000000000000000000000000";

pub struct Executor {
    bus: Bus,
}

impl Executor {
    pub fn new(bus: Bus) -> Self {
        Self { bus }
    }

    fn evaluate(&self, d: &Detection) {
        let skip = |reason: String| {
            self.bus.emit(Event::TradeSkipped {
                chain: d.chain.clone(),
                token: d.token.address.clone(),
                source: d.protocol.clone(),
                kind: d.kind,
                reason,
            });
        };

        // 1 — a contract to route the buy through
        let (venue, target) = match d.market {
            MarketKind::BondingCurve => match &d.curve {
                Some(c) => ("bonding_curve".to_string(), c.address.clone()),
                None => return skip("no curve address".into()),
            },
            MarketKind::Pool => match d.pool.as_ref().and_then(|p| p.address.clone()) {
                Some(addr) => {
                    let venue = d
                        .pool
                        .as_ref()
                        .and_then(|p| p.venue.clone())
                        .unwrap_or_else(|| "pool".into());
                    (venue, addr)
                }
                None => return skip("no pool address".into()),
            },
        };

        // 2 — dev did not take too much of the supply
        if let Some(pct) = d.dev_buy.as_ref().and_then(|b| b.pct_bps)
            && pct > MAX_DEV_BUY_BPS
        {
            return skip(format!("dev holds {:.0}%", pct as f64 / 100.0));
        }

        // 3 — the token answers basic ERC-20 calls
        if d.token.symbol.is_none() {
            return skip("token metadata unreadable".into());
        }

        let (quote_token, amount_in) = match &d.quote_token {
            Some(q) if q.address.eq_ignore_ascii_case(NATIVE) => {
                ("native".to_string(), Some(Dec(U256::from(BUY_SIZE_NATIVE))))
            }
            Some(q) => (q.address.clone(), None),
            None => ("unresolved".to_string(), None),
        };

        self.bus.emit(Event::TradeIntent(Box::new(TradeIntent {
            chain: d.chain.clone(),
            token: d.token.address.clone(),
            symbol: d.token.symbol.clone(),
            source: d.protocol.clone(),
            kind: d.kind,
            venue,
            target,
            quote_token,
            amount_in,
            max_slippage_bps: MAX_SLIPPAGE_BPS,
            dry_run: true,
        })));
    }
}

impl Stage for Executor {
    async fn on_event(&mut self, env: &Envelope) {
        if let Event::Detection(d) = &env.payload {
            self.evaluate(d);
        }
    }
}
