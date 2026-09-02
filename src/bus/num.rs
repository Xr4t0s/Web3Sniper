//! Numeric wrapper: on-chain integers serialize as decimal strings in base
//! units (`"1000000000000000000"`), never hex — see §2 of `SPECS.md`.

use alloy::primitives::U256;
use serde::{Serialize, Serializer};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dec(pub U256);

impl From<U256> for Dec {
    fn from(v: U256) -> Self {
        Dec(v)
    }
}

impl Serialize for Dec {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}

impl std::fmt::Display for Dec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
