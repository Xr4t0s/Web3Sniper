//! On-chain ABIs: the watched events, plus the getters the enrichment step
//! reads (§8.2 of `SPECS.md`).
//!
//! `sol!` generates a `SolEvent` type per event, used for both the topic0
//! (`T::SIGNATURE_HASH`) and decoding (`log.log_decode::<T>()`). The `indexed`
//! markers must match the deployed ABI exactly. All three events are from
//! verified source; a mismatch surfaces as an `Undecoded` event, never a panic.

use alloy::primitives::Address;
use alloy::rpc::types::Log;
use alloy::sol;
use alloy::sol_types::SolEvent;

sol! {
    // PonsV2LaunchFactory (0x7eD5…EC7e).
    #[derive(Debug, serde::Serialize)]
    event PoolGraduated(
        address indexed token,
        uint256 positionId,
        uint256 tokenAmount,
        uint256 pairTokenAmount
    );

    // PonsFamily V2 launch factory (0xe33E…2948).
    #[derive(Debug, serde::Serialize)]
    event Launched(
        address indexed token,
        address indexed curve,
        address indexed recipient,
        address launcher,
        uint256 quoteSpent,
        uint256 tokensReceived
    );

    // PonsFamily V1 launch factory (0xa5aa…1feb).
    #[derive(Debug, serde::Serialize)]
    event TokenLaunched(
        address indexed token,
        address indexed deployer,
        address indexed dexFactory,
        address pairToken,
        address pool,
        uint256 dexId,
        uint256 launchConfigId,
        uint256 positionId,
        uint256 restrictionsEndBlock,
        uint256 initialBuyAmount
    );

    #[sol(rpc)]
    interface IErc20 {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint256);
    }

    // PonsV2BondingCurve getters (call sites in the verified factory source).
    // Best-effort: any getter that is not present just fails and stays `null`.
    #[sol(rpc)]
    interface IPonsV2Curve {
        function realQuoteReserve() external view returns (uint256);
        function tokenReserve() external view returns (uint256);
        function phantomQuote() external view returns (uint256);
        function graduationThreshold() external view returns (uint256);
        function pairToken() external view returns (address);
    }
}

/// A log decoded into a known event.
#[derive(Debug)]
pub enum DecodedEvent {
    PoolGraduated(PoolGraduated),
    Launched(Launched),
    TokenLaunched(TokenLaunched),
}

impl DecodedEvent {
    pub fn name(&self) -> &'static str {
        match self {
            DecodedEvent::PoolGraduated(_) => "PoolGraduated",
            DecodedEvent::Launched(_) => "Launched",
            DecodedEvent::TokenLaunched(_) => "TokenLaunched",
        }
    }

    /// The token address, common to every watched event.
    pub fn token(&self) -> Address {
        match self {
            DecodedEvent::PoolGraduated(e) => e.token,
            DecodedEvent::Launched(e) => e.token,
            DecodedEvent::TokenLaunched(e) => e.token,
        }
    }
}

/// Dispatches a log by topic0. `None` if the topic0 is unknown or the payload
/// does not match the configured ABI.
pub fn decode(log: &Log) -> Option<DecodedEvent> {
    let topic0 = log.topic0()?;

    if *topic0 == PoolGraduated::SIGNATURE_HASH {
        return Some(DecodedEvent::PoolGraduated(
            log.log_decode::<PoolGraduated>().ok()?.inner.data,
        ));
    }
    if *topic0 == Launched::SIGNATURE_HASH {
        return Some(DecodedEvent::Launched(
            log.log_decode::<Launched>().ok()?.inner.data,
        ));
    }
    if *topic0 == TokenLaunched::SIGNATURE_HASH {
        return Some(DecodedEvent::TokenLaunched(
            log.log_decode::<TokenLaunched>().ok()?.inner.data,
        ));
    }
    None
}
