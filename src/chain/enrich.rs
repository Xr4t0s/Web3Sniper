//! Turns a decoded log into a full [`Detection`]. Runs in a detached task per
//! log so the WS reader never blocks. Every extra field is best-effort: calls
//! run concurrently under one time budget, and anything that fails or times out
//! stays `None`.
//!
//! Two guards keep the RPC bill down: immutable token facts come from
//! [`cache`], and at most `MAX_CONCURRENT` enrichments hit the RPC at once
//! (a launch burst queues on the semaphore).
//!
//! Follow-up (SPECS §6): collapse the concurrent calls into a single Multicall3
//! round-trip; resolve the PonsV1 `dexId` → venue/router lookup.

use std::future::Future;
use std::time::Duration;

use alloy::primitives::{Address, U256};
use alloy::providers::Provider as _;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::bus::num::Dec;
use crate::chain::cache::{self, TokenFacts};
use crate::chain::contracts::{DecodedEvent, IErc20, IPonsV2Curve};
use crate::chain::provider::Provider;
use crate::config::watchlist::TargetKind;
use crate::events::detection::*;

const BUDGET: Duration = Duration::from_millis(1500);

/// Cap on concurrent enrichments so a launch burst does not spike the RPC past
/// its rate limit; the rest queue here.
const MAX_CONCURRENT: usize = 6;
static PERMITS: Semaphore = Semaphore::const_new(MAX_CONCURRENT);

async fn within<F: Future>(f: F) -> Option<F::Output> {
    timeout(BUDGET, f).await.ok()
}

/// Immutable token facts, from cache or one RPC round.
async fn token_facts<P: alloy::providers::Provider>(provider: P, addr: Address) -> TokenFacts {
    if let Some(hit) = cache::facts().get(&addr) {
        return hit;
    }
    let erc20 = IErc20::new(addr, provider);
    let (name_c, sym_c, dec_c, sup_c) = (
        erc20.name(),
        erc20.symbol(),
        erc20.decimals(),
        erc20.totalSupply(),
    );
    let facts = within(async {
        let (n, s, d, ts) = tokio::join!(name_c.call(), sym_c.call(), dec_c.call(), sup_c.call());
        TokenFacts {
            name: n.ok(),
            symbol: s.ok(),
            decimals: d.ok(),
            total_supply: ts.ok(),
        }
    })
    .await
    .unwrap_or_default();

    if facts.is_useful() {
        cache::facts().put(addr, facts.clone());
    }
    facts
}

fn bps(part: U256, whole: U256) -> Option<u32> {
    (!whole.is_zero()).then(|| {
        (part.saturating_mul(U256::from(10_000)) / whole)
            .try_into()
            .unwrap_or(u32::MAX)
    })
}

pub async fn enrich(
    chain: &str,
    protocol: &str,
    kind: TargetKind,
    decoded: &DecodedEvent,
    log: &alloy::rpc::types::Log,
    provider: &Provider,
) -> Detection {
    let _permit = PERMITS.acquire().await;
    let http = &provider.http;
    let token_addr = decoded.token();

    // Token facts (cached) + block timestamp, concurrent.
    let (facts, block_timestamp) = tokio::join!(token_facts(http, token_addr), async {
        match log.block_number {
            Some(n) => http
                .get_block_by_number(n.into())
                .await
                .ok()
                .flatten()
                .map(|b| b.header.timestamp),
            None => None,
        }
    });
    let total_supply = facts.total_supply;

    let token = TokenInfo {
        address: token_addr.to_string(),
        name: facts.name,
        symbol: facts.symbol,
        decimals: facts.decimals,
        total_supply: total_supply.map(Dec),
    };

    let mut economics = Economics::default();
    let mut quote_token: Option<TokenRef> = None;
    let mut curve: Option<CurveState> = None;
    let mut pool: Option<PoolState> = None;
    let (creator, launcher, dev_buy, market) = match decoded {
        DecodedEvent::Launched(e) => {
            let c = IPonsV2Curve::new(e.curve, http);
            let (rqr_c, tr_c, pq_c, gt_c, pair_c) = (
                c.realQuoteReserve(),
                c.tokenReserve(),
                c.phantomQuote(),
                c.graduationThreshold(),
                c.pairToken(),
            );
            let (rqr, tr, pq, gt, pair) = within(async {
                let (a, b, c, d, p) = tokio::join!(
                    rqr_c.call(),
                    tr_c.call(),
                    pq_c.call(),
                    gt_c.call(),
                    pair_c.call(),
                );
                (a.ok(), b.ok(), c.ok(), d.ok(), p.ok())
            })
            .await
            .unwrap_or((None, None, None, None, None));

            economics.phantom_quote = pq.map(Dec);
            economics.graduation_threshold = gt.map(Dec);
            if let Some(addr) = pair {
                quote_token = Some(quote_ref(addr, http).await);
            }

            curve = Some(CurveState {
                address: e.curve.to_string(),
                progress_bps: match (rqr, gt) {
                    (Some(r), Some(g)) => bps(r, g),
                    _ => None,
                },
                real_quote_reserve: rqr.map(Dec),
                token_reserve: tr.map(Dec),
                price_native: None,
            });

            (
                e.recipient.to_string(),
                Some(e.launcher.to_string()),
                Some(DevBuy {
                    tokens: Some(Dec(e.tokensReceived)),
                    quote: Some(Dec(e.quoteSpent)),
                    pct_bps: total_supply.and_then(|ts| bps(e.tokensReceived, ts)),
                }),
                MarketKind::BondingCurve,
            )
        }

        DecodedEvent::PoolGraduated(e) => {
            pool = Some(PoolState {
                venue: Some("uniswap-v4".into()),
                address: None,
                position_id: Some(Dec(e.positionId)),
                router: None,
                reserve_token: Some(Dec(e.tokenAmount)),
                reserve_quote: Some(Dec(e.pairTokenAmount)),
                price_native: None,
            });
            (String::new(), None, None, MarketKind::Pool)
        }

        DecodedEvent::TokenLaunched(e) => {
            quote_token = Some(quote_ref(e.pairToken, http).await);
            pool = Some(PoolState {
                venue: None, // TODO: resolve from dexId
                address: Some(e.pool.to_string()),
                position_id: Some(Dec(e.positionId)),
                router: None, // TODO: resolve from dexId
                reserve_token: None,
                reserve_quote: None,
                price_native: None,
            });
            (
                e.deployer.to_string(),
                None,
                Some(DevBuy {
                    tokens: Some(Dec(e.initialBuyAmount)),
                    quote: None,
                    pct_bps: total_supply.and_then(|ts| bps(e.initialBuyAmount, ts)),
                }),
                MarketKind::Pool,
            )
        }
    };

    Detection {
        chain: chain.to_string(),
        protocol: protocol.to_string(),
        kind,
        event: decoded.name(),
        token,
        creator,
        launcher,
        dev_buy,
        quote_token,
        economics,
        market,
        curve,
        pool,
        valuation: None,
        tx_hash: log
            .transaction_hash
            .map(|h| h.to_string())
            .unwrap_or_default(),
        block_number: log.block_number.unwrap_or_default(),
        log_index: log.log_index.unwrap_or_default(),
        block_timestamp,
        observed_at: chrono::Local::now().to_rfc3339(),
    }
}

async fn quote_ref<P: alloy::providers::Provider>(addr: Address, provider: P) -> TokenRef {
    let facts = token_facts(provider, addr).await;
    TokenRef {
        address: addr.to_string(),
        symbol: facts.symbol,
        decimals: facts.decimals,
    }
}
