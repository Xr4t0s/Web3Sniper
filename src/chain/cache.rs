//! Process-wide cache of immutable ERC-20 facts.
//!
//! `name` / `symbol` / `decimals` / `totalSupply` never change, so keying by
//! address and caching forever is safe. It pays off three ways: a token's later
//! graduation reuses its launch lookup, a relaunch reuses the first, and the
//! handful of quote tokens (WETH / USDG / native wrappers) that recur on every
//! single launch are fetched once.
//!
//! Two-generation eviction: when the hot map fills, it becomes the cold map and
//! a fresh hot map starts. Entries survive between `CAP` and `2 * CAP` inserts,
//! with no per-entry bookkeeping and no dependency.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use alloy::primitives::{Address, U256};

#[derive(Clone, Default)]
pub struct TokenFacts {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub total_supply: Option<U256>,
}

impl TokenFacts {
    /// Worth caching only if a call actually returned something.
    pub fn is_useful(&self) -> bool {
        self.symbol.is_some() || self.decimals.is_some() || self.total_supply.is_some()
    }
}

const CAP: usize = 4096;

pub struct FactCache {
    gens: Mutex<(HashMap<Address, TokenFacts>, HashMap<Address, TokenFacts>)>,
}

impl FactCache {
    pub fn get(&self, addr: &Address) -> Option<TokenFacts> {
        let (hot, cold) = &*self.gens.lock().unwrap();
        hot.get(addr).or_else(|| cold.get(addr)).cloned()
    }

    pub fn put(&self, addr: Address, facts: TokenFacts) {
        let (hot, cold) = &mut *self.gens.lock().unwrap();
        if hot.len() >= CAP {
            std::mem::swap(hot, cold);
            hot.clear();
        }
        hot.insert(addr, facts);
    }
}

pub fn facts() -> &'static FactCache {
    static CACHE: OnceLock<FactCache> = OnceLock::new();
    CACHE.get_or_init(|| FactCache {
        gens: Mutex::new((HashMap::new(), HashMap::new())),
    })
}
