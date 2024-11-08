#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Uint128,
};
use cw2::set_contract_version;
use std::str::FromStr;

use crate::claim_management::{execute_add_claim, execute_remove_claim};
use crate::error::ContractError;
use crate::key_management::{execute_add_key, execute_remove_key};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Claim, Key, KeyType, CLAIMS, KEYS, OWNER};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:onchainid";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION).map_err(|e| {
        ContractError::Std(StdError::generic_err(format!(
            "Failed to set contract version: {}",
            e
        )))
    })?;

    let owner = deps
        .api
        .addr_validate(&msg.owner)
        .map_err(|e| ContractError::InvalidAddress {
            reason: format!("Invalid owner address: {}", e),
        })?;

    // Create and save the management key for the owner
    let key = Key {
        key_type: KeyType::ManagementKey,
        owner: owner.clone(),
    };
    KEYS.save(deps.storage, &owner, &vec![key])
        .map_err(|e| ContractError::SaveError {
            entity: "keys".to_string(),
            reason: e.to_string(),
        })?;
    // Save the owner
    OWNER
        .save(deps.storage, &owner)
        .map_err(|e| ContractError::SaveError {
            entity: "owner".to_string(),
            reason: e.to_string(),
        })?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddKey {
            key_owner,
            key_type,
        } => execute_add_key(deps, info, key_owner, key_type),
        ExecuteMsg::RevokeKey {
            key_owner,
            key_type,
        } => execute_remove_key(deps, info, key_owner, key_type),
        ExecuteMsg::AddClaim {
            claim,
            public_key,
            user_addr,
        } => execute_add_claim(deps, info, claim, public_key, user_addr),
        ExecuteMsg::RemoveClaim {
            claim_topic,
            user_addr,
        } => execute_remove_claim(deps, info, claim_topic, user_addr),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetKey {
            key_owner,
            key_type,
        } => to_json_binary(&query_key(deps, key_owner, key_type)?),
        QueryMsg::GetValidatedClaimsForUser { user_addr } => {
            to_json_binary(&get_validated_claims_for_user(deps, user_addr)?)
        }

        QueryMsg::VerifyClaim {
            claim_id,
            user_addr,
        } => to_json_binary(&verify_claim(deps, claim_id, user_addr)?),
        QueryMsg::GetOwner {} => to_json_binary(&query_owner(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let current_version = cw2::get_contract_version(deps.storage)?;

    if current_version.contract != CONTRACT_NAME {
        return Err(ContractError::InvalidContract {
            expected: CONTRACT_NAME.to_string(),
            actual: current_version.contract,
        });
    }

    let new_version = CONTRACT_VERSION.parse::<semver::Version>().unwrap();
    let stored_version = current_version.version.parse::<semver::Version>().unwrap();

    if stored_version >= new_version {
        return Err(ContractError::AlreadyMigrated {
            current_version: stored_version.to_string(),
            new_version: new_version.to_string(),
        });
    }

    // Perform any necessary state migrations here

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", current_version.version)
        .add_attribute("to_version", CONTRACT_VERSION))
}

fn query_key(deps: Deps, key_owner: String, key_type: String) -> StdResult<Key> {
    let key_owner = deps
        .api
        .addr_validate(&key_owner)
        .map_err(|e| StdError::generic_err(format!("Invalid key owner address: {}", e)))?;
    let key_type = KeyType::from_str(&key_type)
        .map_err(|_| StdError::generic_err(format!("Invalid key type: {}", key_type)))?;
    let owner = OWNER
        .load(deps.storage)
        .map_err(|e| StdError::generic_err(format!("Failed to load owner: {}", e)))?;
    let keys = KEYS.load(deps.storage, &owner).map_err(|e| {
        StdError::generic_err(format!("Failed to load keys for owner {}: {}", owner, e))
    })?;
    keys.iter()
        .find(|key| key.key_type == key_type && key.owner == key_owner)
        .cloned()
        .ok_or_else(|| {
            StdError::not_found(format!(
                "Key not found for owner {} and type {:?}",
                key_owner, key_type
            ))
        })
}

fn get_validated_claims_for_user(deps: Deps, user_addr: Addr) -> StdResult<Vec<Claim>> {
    let user_addr = deps.api.addr_validate(user_addr.as_str())?;

    let claims = CLAIMS
        .load(deps.storage, &user_addr)
        .map_err(|e| StdError::generic_err(format!("User has no claims {}: {}", user_addr, e)))?;
    Ok(claims)
}

fn verify_claim(deps: Deps, claim_id: Uint128, user_addr: Addr) -> StdResult<bool> {
    let user_addr = deps.api.addr_validate(user_addr.as_str())?;
    let claims = CLAIMS
        .load(deps.storage, &user_addr)
        .map_err(|e| StdError::generic_err(format!("User has no claims  {}: {}", user_addr, e)))?;

    Ok(claims.iter().any(|claim| claim.topic == claim_id))
}

fn query_owner(deps: Deps) -> StdResult<Addr> {
    OWNER
        .load(deps.storage)
        .map_err(|e| StdError::generic_err(format!("Failed to load owner: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::hash_claim_without_signature;
    use cosmwasm_std::{testing::MockApi, Addr, Binary};
    use cw_multi_test::{App, ContractWrapper, Executor};
    use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};

    fn instantiate_contract(app: &mut App, owner: Addr) -> Addr {
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        app.instantiate_contract(
            code_id,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
            },
            &[],
            "On-chain ID Contract",
            Some(owner.to_string()),
        )
        .unwrap()
    }

    fn create_wallet(app: &App) -> (Addr, SecretKey, PublicKey) {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut rand::thread_rng());
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let addr = app.api().addr_make(&public_key.to_string());
        (addr.clone(), secret_key, public_key)
    }

    #[test]
    fn proper_initialization() {
        let mut app = App::default();
        let owner = app.api().addr_make("owner");

        let contract_addr = instantiate_contract(&mut app, owner.clone());

        // Test query_owner
        let res: Addr = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::GetOwner {})
            .unwrap();
        assert_eq!(res, owner);
    }

    #[test]
    fn add_and_remove_key() {
        let mut app = App::default();
        let owner = app.api().addr_make("owner");
        let contract_addr = instantiate_contract(&mut app, owner.clone());

        let key_owner = app.api().addr_make("new_key_owner");

        // Test adding a key
        let msg = ExecuteMsg::AddKey {
            key_owner: key_owner.to_string(),
            key_type: "ExecutionKey".to_string(),
        };
        app.execute_contract(owner.clone(), contract_addr.clone(), &msg, &[])
            .unwrap();
        // Test querying the added key
        let res: Key = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetKey {
                    key_owner: key_owner.to_string().clone(),
                    key_type: "ExecutionKey".to_string(),
                },
            )
            .unwrap();
        assert_eq!(res.owner, Addr::unchecked(key_owner.clone()));
        assert_eq!(res.key_type, KeyType::ExecutionKey);

        // Test removing the key
        let msg = ExecuteMsg::RevokeKey {
            key_owner: key_owner.to_string(),
            key_type: "ExecutionKey".to_string(),
        };
        app.execute_contract(owner.clone(), contract_addr.clone(), &msg, &[])
            .unwrap();

        // Verify the key is removed
        let res: Result<Key, _> = app.wrap().query_wasm_smart(
            contract_addr,
            &QueryMsg::GetKey {
                key_owner: key_owner.to_string(),
                key_type: "ExecutionKey".to_string(),
            },
        );
        assert!(res.is_err());
    }

    #[test]
    fn add_and_remove_claim() {
        let mut app = App::default();
        let (owner_addr, owner_secret_key, owner_public_key) = create_wallet(&app);
        let contract_addr = instantiate_contract(&mut app, owner_addr.clone());
        let user_addr = MockApi::default().addr_make("user_addr");

        // Add a claim signer key first
        let msg = ExecuteMsg::AddKey {
            key_owner: owner_addr.to_string(),
            key_type: "ClaimSignerKey".to_string(),
        };
        app.execute_contract(owner_addr.clone(), contract_addr.clone(), &msg, &[])
            .unwrap();

        // Create a claim
        let claim = Claim {
            topic: Uint128::one(),
            issuer: owner_addr.clone(),
            signature: Binary::from(vec![]), // This will be filled later
            data: Binary::from(vec![4, 5, 6]),
            uri: "https://example.com".to_string(),
        };

        // Hash the claim data (excluding signature)
        let message_hash = hash_claim_without_signature(&claim);

        // Sign the hash
        let secp = Secp256k1::new();
        let message = Message::from_slice(&message_hash).unwrap();
        let signature = secp.sign_ecdsa(&message, &owner_secret_key);

        // Create the final claim with the signature
        let signed_claim = Claim {
            signature: Binary::from(signature.serialize_compact()),
            ..claim
        };

        // Test adding the claim
        let msg = ExecuteMsg::AddClaim {
            claim: signed_claim.clone(),
            public_key: Binary::from(owner_public_key.serialize()),
            user_addr: user_addr.clone(),
        };
        app.execute_contract(owner_addr.clone(), contract_addr.clone(), &msg, &[])
            .unwrap();

        // Test querying the added claim
        let res: bool = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::VerifyClaim {
                    claim_id: Uint128::one(),
                    user_addr: user_addr.clone(),
                },
            )
            .unwrap();
        assert_eq!(res, true);

        // Test removing the claim
        let msg = ExecuteMsg::RemoveClaim {
            claim_topic: Uint128::one(),
            user_addr: user_addr.clone(),
        };
        app.execute_contract(owner_addr.clone(), contract_addr.clone(), &msg, &[])
            .unwrap();

        // Verify the claim is removed
        let res: StdResult<Binary> = app.wrap().query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::VerifyClaim {
                claim_id: Uint128::one(),
                user_addr: user_addr.clone(),
            },
        );
        assert!(res.is_err());
    }

    #[test]
    fn add_and_query_claims() {
        let mut app = App::default();
        let (owner_addr, owner_secret_key, owner_public_key) = create_wallet(&app);
        let contract_addr = instantiate_contract(&mut app, owner_addr.clone());
        let user_addr = MockApi::default().addr_make("user_addr");

        // Add a claim signer key
        let msg = ExecuteMsg::AddKey {
            key_owner: owner_addr.to_string(),
            key_type: "ClaimSignerKey".to_string(),
        };
        app.execute_contract(owner_addr.clone(), contract_addr.clone(), &msg, &[])
            .unwrap();

        let claim_topics = vec![Uint128::one(), Uint128::new(7777), Uint128::new(88)];

        // Add claims one at a time
        for topic in &claim_topics {
            let claim = Claim {
                topic: topic.clone(),
                issuer: owner_addr.clone(),
                signature: Binary::from(vec![]),
                data: Binary::from(vec![1, 2, 3]),
                uri: "https://example.com".to_string(),
            };

            let message_hash = hash_claim_without_signature(&claim);
            let secp = Secp256k1::new();
            let message = Message::from_slice(&message_hash).unwrap();
            let signature = secp.sign_ecdsa(&message, &owner_secret_key);

            let signed_claim = Claim {
                signature: Binary::from(signature.serialize_compact()),
                ..claim
            };

            let msg = ExecuteMsg::AddClaim {
                claim: signed_claim,
                public_key: Binary::from(owner_public_key.serialize()),
                user_addr: user_addr.clone(),
            };
            app.execute_contract(owner_addr.clone(), contract_addr.clone(), &msg, &[])
                .unwrap();
        }

        // Query and verify each claim
        for (_, topic) in claim_topics.iter().enumerate() {
            let res: bool = app
                .wrap()
                .query_wasm_smart(
                    contract_addr.clone(),
                    &QueryMsg::VerifyClaim {
                        claim_id: *topic,
                        user_addr: user_addr.clone(),
                    },
                )
                .unwrap();
            assert_eq!(res, true);
        }

        // Attempt to add a duplicate claim
        let duplicate_claim = Claim {
            topic: claim_topics[0].clone(),
            issuer: owner_addr.clone(),
            signature: Binary::from(vec![]),
            data: Binary::from(vec![1, 2, 3]),
            uri: "https://example.com".to_string(),
        };
        let message_hash = hash_claim_without_signature(&duplicate_claim);
        let secp = Secp256k1::new();
        let message = Message::from_slice(&message_hash).unwrap();
        let signature = secp.sign_ecdsa(&message, &owner_secret_key);
        let signed_duplicate_claim = Claim {
            signature: Binary::from(signature.serialize_compact()),
            ..duplicate_claim
        };
        let msg = ExecuteMsg::AddClaim {
            claim: signed_duplicate_claim,
            public_key: Binary::from(owner_public_key.serialize()),
            user_addr: user_addr.clone(),
        };
        let err = app
            .execute_contract(owner_addr.clone(), contract_addr.clone(), &msg, &[])
            .unwrap_err();
        assert!(err.to_string().contains("Error"));
    }

    #[test]
    fn add_different_key_types() {
        let mut app = App::default();
        let owner = app.api().addr_make("owner");
        let contract_addr = instantiate_contract(&mut app, owner.clone());

        let key_types = vec!["ExecutionKey", "ClaimSignerKey", "EncryptionKey"];

        // Add keys one at a time
        for key_type in &key_types {
            let msg = ExecuteMsg::AddKey {
                key_owner: owner.to_string(),
                key_type: key_type.to_string(),
            };
            app.execute_contract(owner.clone(), contract_addr.clone(), &msg, &[])
                .unwrap();

            // Query and verify the added key
            let res: Key = app
                .wrap()
                .query_wasm_smart(
                    contract_addr.clone(),
                    &QueryMsg::GetKey {
                        key_owner: owner.to_string(),
                        key_type: key_type.to_string(),
                    },
                )
                .unwrap();
            assert_eq!(res.owner, owner);
            assert_eq!(res.key_type, KeyType::from_str(key_type).unwrap());
        }

        // Attempt to add a duplicate key
        let msg = ExecuteMsg::AddKey {
            key_owner: owner.to_string(),
            key_type: "ManagementKey".to_string(),
        };
        let err = app
            .execute_contract(owner.clone(), contract_addr.clone(), &msg, &[])
            .unwrap_err();
        assert!(err.to_string().contains("Error"));
    }

    #[test]
    fn add_key_to_different_wallet() {
        let mut app = App::default();
        let owner = app.api().addr_make("owner");
        let contract_addr = instantiate_contract(&mut app, owner.clone());

        // Create a different wallet address
        let different_wallet = app.api().addr_make("different_wallet");

        // Add a key for the different wallet
        let msg = ExecuteMsg::AddKey {
            key_owner: different_wallet.to_string(),
            key_type: "ExecutionKey".to_string(),
        };
        app.execute_contract(owner.clone(), contract_addr.clone(), &msg, &[])
            .unwrap();

        // Query the added key
        let res: Key = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetKey {
                    key_owner: different_wallet.to_string(),
                    key_type: "ExecutionKey".to_string(),
                },
            )
            .unwrap();

        // Verify the key details
        assert_eq!(res.owner, different_wallet);
        assert_eq!(res.key_type, KeyType::ExecutionKey);

        // Attempt to add another key with the different wallet (should fail)
        let msg = ExecuteMsg::AddKey {
            key_owner: owner.to_string(),
            key_type: "ManagementKey".to_string(),
        };
        let err = app
            .execute_contract(different_wallet.clone(), contract_addr.clone(), &msg, &[])
            .unwrap_err();
        assert!(err.to_string().contains("Error"));

        // The owner should still be able to add keys
        let msg = ExecuteMsg::AddKey {
            key_owner: owner.to_string(),
            key_type: "EncryptionKey".to_string(),
        };
        app.execute_contract(owner.clone(), contract_addr.clone(), &msg, &[])
            .unwrap();

        // Verify both keys exist
        let res: Key = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetKey {
                    key_owner: different_wallet.to_string(),
                    key_type: "ExecutionKey".to_string(),
                },
            )
            .unwrap();
        assert_eq!(res.owner, different_wallet);
        assert_eq!(res.key_type, KeyType::ExecutionKey);

        let res: Key = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::GetKey {
                    key_owner: owner.to_string(),
                    key_type: "EncryptionKey".to_string(),
                },
            )
            .unwrap();
        assert_eq!(res.owner, owner);
        assert_eq!(res.key_type, KeyType::EncryptionKey);
    }
}
