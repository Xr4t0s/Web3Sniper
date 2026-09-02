//! Définitions typées des events on-chain surveillés, générées par `alloy::sol!`.
//!
//! `sol!` produit pour chaque event un type implémentant `SolEvent` : on s'en
//! sert à la fois pour le topic0 (`T::SIGNATURE_HASH`) et pour décoder un log
//! (`log.log_decode::<T>()`).
//!
//! ⚠️ Les marqueurs `indexed` doivent correspondre EXACTEMENT à l'ABI déployée,
//! sinon le décodage échoue (les champs indexed vivent dans les topics, les
//! autres dans `data`). `PoolGraduated` vient de la source vérifiée de
//! `PonsV2LaunchFactory`. Les deux events de launch sont à confirmer contre
//! l'explorer — marqués TODO.

use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::rpc::types::Log;
use alloy::sol;
use alloy::sol_types::SolEvent;

sol! {
    // --- PonsV2LaunchFactory (0x7eD5…EC7e) — source vérifiée ---
    #[derive(Debug)]
    event PoolGraduated(
        address indexed token,
        uint256 positionId,
        uint256 tokenAmount,
        uint256 pairTokenAmount
    );

    // --- PonsFamily V2 launch factory (0xe33E…2948) ---
    // TODO: confirmer les `indexed` contre l'ABI déployée.
    #[derive(Debug)]
    event Launched(
        address indexed token,
        address indexed pool,
        address indexed deployer,
        address quoteToken,
        uint256 arg4,
        uint256 arg5
    );

    // Metadata ERC-20 : les events de launch ne portent que des adresses,
    // il faut interroger le token lui-même (via le provider HTTP) pour name/symbol.
    #[sol(rpc)]
    interface IErc20Metadata {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
    }

    // --- PonsFamily launch factory (0xa5aa…1feb) ---
    // TODO: confirmer les `indexed` contre l'ABI déployée.
    #[derive(Debug)]
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
}

/// Un log décodé en event connu.
#[derive(Debug)]
pub enum DecodedEvent {
    PoolGraduated(PoolGraduated),
    Launched(Launched),
    TokenLaunched(TokenLaunched),
}

impl DecodedEvent {
    /// Adresse du token concerné, commune à tous les events surveillés.
    pub fn token(&self) -> alloy::primitives::Address {
        match self {
            DecodedEvent::PoolGraduated(e) => e.token,
            DecodedEvent::Launched(e) => e.token,
            DecodedEvent::TokenLaunched(e) => e.token,
        }
    }
}

/// Metadata d'un token, chaque champ absent si l'appel revert (token non standard).
#[derive(Debug, Default)]
pub struct TokenMetadata {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
}

impl TokenMetadata {
    /// `SYMBOL (name)` pour l'affichage, avec des `?` sur les champs manquants.
    pub fn label(&self) -> String {
        format!(
            "{} ({})",
            self.symbol.as_deref().unwrap_or("?"),
            self.name.as_deref().unwrap_or("?"),
        )
    }
}

/// Lit name/symbol/decimals du token via un provider (typiquement le HTTP).
pub async fn token_metadata<P: Provider>(provider: P, token: Address) -> TokenMetadata {
    let erc20 = IErc20Metadata::IErc20MetadataInstance::new(token, provider);
    TokenMetadata {
        name: erc20.name().call().await.ok(),
        symbol: erc20.symbol().call().await.ok(),
        decimals: erc20.decimals().call().await.ok(),
    }
}

/// Tente de décoder un log en l'un des events connus (dispatch par topic0).
/// `None` si le topic0 est inconnu ou si le décodage échoue (ABI qui ne
/// correspond pas).
pub fn decode(log: &Log) -> Option<DecodedEvent> {
    let topic0 = log.topic0()?;

    if *topic0 == PoolGraduated::SIGNATURE_HASH {
        return Some(DecodedEvent::PoolGraduated(log.log_decode::<PoolGraduated>().ok()?.inner.data));
    }
    if *topic0 == Launched::SIGNATURE_HASH {
        return Some(DecodedEvent::Launched(log.log_decode::<Launched>().ok()?.inner.data));
    }
    if *topic0 == TokenLaunched::SIGNATURE_HASH {
        return Some(DecodedEvent::TokenLaunched(log.log_decode::<TokenLaunched>().ok()?.inner.data));
    }
    None
}
