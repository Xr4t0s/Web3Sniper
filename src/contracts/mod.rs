//! On-chain ABIs: the events we watch and the ERC-20 metadata call.
//!
//! `sol!` generates a `SolEvent` type per event; we use it for both the topic0
//! (`T::SIGNATURE_HASH`) and decoding (`log.log_decode::<T>()`).
//!
//! The `indexed` markers must match the deployed ABI exactly (indexed fields
//! live in topics, the rest in `data`). `PoolGraduated` is from the verified
//! `PonsV2LaunchFactory` source; the launch events are inferred and marked
//! TODO — a mismatch surfaces as an `Undecoded` event, never a panic.

use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::rpc::types::Log;
use alloy::sol;
use alloy::sol_types::SolEvent;
use serde::Serialize;

sol! {
    // PonsV2LaunchFactory (0x7eD5…EC7e) — verified source.
    #[derive(Debug, serde::Serialize)]
    event PoolGraduated(
        address indexed token,
        uint256 positionId,
        uint256 tokenAmount,
        uint256 pairTokenAmount
    );

    // PonsFamily V2 launch factory (0xe33E…2948). TODO: confirm `indexed`.
    #[derive(Debug, serde::Serialize)]
    event Launched(
        address indexed token,
        address indexed pool,
        address indexed deployer,
        address quoteToken,
        uint256 arg4,
        uint256 arg5
    );

    // PonsFamily launch factory (0xa5aa…1feb). TODO: confirm `indexed`.
    #[derive(Debug, serde::Serialize)]
    event TokenLaunched(
        address indexed token,
        address indexed pool,
        address indexed deployer,
        address quoteToken,
        address arg4,
        uint256 arg5,
        uint256 arg6,
        uint256 arg7,
        uint256 arg8,
        uint256 arg9
    );

    #[sol(rpc)]
    interface IErc20Metadata {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
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
    /// Solidity event name.
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

    /// Decoded fields as JSON, for consumers that want the raw payload.
    pub fn payload(&self) -> serde_json::Value {
        let value = match self {
            DecodedEvent::PoolGraduated(e) => serde_json::to_value(e),
            DecodedEvent::Launched(e) => serde_json::to_value(e),
            DecodedEvent::TokenLaunched(e) => serde_json::to_value(e),
        };
        value.unwrap_or(serde_json::Value::Null)
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

/// Token metadata; a field is `None` when the call reverts (non-standard token).
#[derive(Clone, Debug, Default, Serialize)]
pub struct TokenMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
}

impl TokenMetadata {
    /// `SYMBOL (name)` for display, `?` for missing fields.
    pub fn label(&self) -> String {
        format!(
            "{} ({})",
            self.symbol.as_deref().unwrap_or("?"),
            self.name.as_deref().unwrap_or("?"),
        )
    }
}

/// Reads `name` / `symbol` / `decimals` from a token (use the HTTP provider).
pub async fn token_metadata<P: Provider>(provider: P, token: Address) -> TokenMetadata {
    let erc20 = IErc20Metadata::new(token, provider);
    TokenMetadata {
        name: erc20.name().call().await.ok(),
        symbol: erc20.symbol().call().await.ok(),
        decimals: erc20.decimals().call().await.ok(),
    }
}
