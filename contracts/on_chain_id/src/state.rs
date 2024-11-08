use crate::error::ContractError;
use cosmwasm_std::{Addr, Binary, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

//Addr being the Key owner
pub const KEYS: Map<&Addr, Vec<Key>> = Map::new("keys");

//Addr being the Identity owner
pub const CLAIMS: Map<&Addr, Vec<Claim>> = Map::new("claims");

//Addr being the Owner of the Identity (not to be confused with the Key owner)
pub const OWNER: Item<Addr> = Item::new("owner");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key {
    pub owner: Addr,
    pub key_type: KeyType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Claim {
    pub topic: Uint128,
    pub issuer: Addr,
    pub signature: Binary,
    pub data: Binary,
    pub uri: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum KeyType {
    // 1: MANAGEMENT keys, which can manage the identity
    ManagementKey,
    // 2: EXECUTION keys, which perform actions in this identities name (signing, logins, transactions, etc.)
    ExecutionKey,
    // 3: CLAIM signer keys, used to sign claims on other identities which need to be revokable.
    ClaimSignerKey,
    // 4: ENCRYPTION keys, used to encrypt data e.g. hold in claims.
    EncryptionKey,
}

impl FromStr for KeyType {
    type Err = ContractError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ManagementKey" => Ok(KeyType::ManagementKey),
            "ExecutionKey" => Ok(KeyType::ExecutionKey),
            "ClaimSignerKey" => Ok(KeyType::ClaimSignerKey),
            "EncryptionKey" => Ok(KeyType::EncryptionKey),
            _ => Err(ContractError::InvalidKeyType {
                key_type: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyType::ManagementKey => write!(f, "ManagementKey"),
            KeyType::ExecutionKey => write!(f, "ExecutionKey"),
            KeyType::ClaimSignerKey => write!(f, "ClaimSignerKey"),
            KeyType::EncryptionKey => write!(f, "EncryptionKey"),
        }
    }
}
