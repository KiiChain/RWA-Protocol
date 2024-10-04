#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, Deps, DepsMut, Env, MessageInfo, Response, Addr, StdResult
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::key_management::{add_key, remove_key};
use crate::claim_management::{add_claim, remove_claim};
use crate::state::{Identity, Key, KeyType, IDENTITY, OWNER};

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
    
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    
    // Generate a unique key for the management key
    let key = Key {
        key_type: KeyType::ManagementKey,
        owner: msg.owner.clone(),
    };

    let identity = Identity {
        address: msg.owner.clone(),
        keys: vec![key.clone()],
        claims: vec![],
    };

    IDENTITY.save(deps.storage, &msg.owner, &identity)?;
    OWNER.save(deps.storage, &msg.owner)?;
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
        ExecuteMsg::AddKey { key_owner, key_type } => add_key(deps, info, key_owner, key_type),
        ExecuteMsg::RevokeKey { key_owner, key_type } => remove_key(deps, info, key_owner, key_type),
        ExecuteMsg::AddClaim { claim, issuer_signature } => add_claim(deps, info, claim, issuer_signature),
        ExecuteMsg::RemoveClaim { claim_id } => remove_claim(deps, info, claim_id),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Response, ContractError> {
    match msg {
        QueryMsg::GetKey { key_type } => query_key(deps, key_type),
        QueryMsg::GetClaim { claim_id } => query_claim(deps, claim_id),
        QueryMsg::GetClaimIdsByTopic { topic } => query_claim_ids_by_topic(deps, topic),
        QueryMsg::GetClaimsByIssuer { issuer } => query_claims_by_issuer(deps, issuer),
        QueryMsg::VerifyClaim { claim_id, trusted_issuers_registry } => 
            verify_claim(deps, claim_id, trusted_issuers_registry),
        QueryMsg::GetOwner {} => query_owner(deps),
    }
}

fn query_key(deps: Deps, key_owner: String, key_type: String) -> Result<Response, ContractError> {
    let key_type = KeyType::from_str(&key_type)?;
    let identity = IDENTITY.load(deps.storage, &owner)?;
    let key = identity.keys.iter().find(|k| k.key_type == key_type)
        .ok_or(ContractError::KeyNotFound {})?;
    Ok(Response::new().add_attribute("key", serde_json::to_string(&key)?))
}

fn query_claim(deps: Deps, key_owner: String, claim_id: String) -> Result<Response, ContractError> {
    let identity = IDENTITY.load(deps.storage, &owner)?;
    let claim = identity.claims.iter()
        .find(|c| c.id.as_ref() == Some(&claim_id))
        .ok_or(ContractError::ClaimNotFound {})?;
    Ok(Response::new().add_attribute("claim", serde_json::to_string(&claim)?))
}

fn query_claim_ids_by_topic(deps: Deps, key_owner: String, topic: String) -> Result<Response, ContractError> {
    let topic = ClaimTopic::from_str(&topic)?;
    let identity = IDENTITY.load(deps.storage, &owner)?;
    let claim_ids: Vec<String> = identity.claims.iter()
        .filter(|c| c.topic == topic)
        .filter_map(|c| c.id.clone())
        .collect();
    Ok(Response::new().add_attribute("claim_ids", serde_json::to_string(&claim_ids)?))
}

fn query_claims_by_issuer(deps: Deps, key_owner: String, issuer: String) -> Result<Response, ContractError> {
    let issuer_addr = deps.api.addr_validate(&issuer)?;
    let identity = IDENTITY.load(deps.storage, &owner)?;
    let claims: Vec<&Claim> = identity.claims.iter()
        .filter(|c| c.issuer == issuer_addr)
        .collect();
    Ok(Response::new().add_attribute("claims", serde_json::to_string(&claims)?))
}

fn verify_claim(deps: Deps, key_owner: String, claim_id: String, trusted_issuers_registry: String) -> Result<Response, ContractError> {
    let identity = IDENTITY.load(deps.storage, &owner)?;
    let claim = identity.claims.iter()
        .find(|c| c.id.as_ref() == Some(&claim_id))
        .ok_or(ContractError::ClaimNotFound {})?;
    
    // Here you would typically check if the claim issuer is in the trusted issuers registry
    // For this example, we'll just check if the issuer matches the provided registry
    // In a real implementation, you'd want to query an actual registry contract
    
    let is_verified = claim.issuer == deps.api.addr_validate(&trusted_issuers_registry)?;
    Ok(Response::new().add_attribute("is_verified", is_verified.to_string()))
}

fn query_owner(deps: Deps) -> StdResult<Addr> {
    let owner = OWNER.load(deps.storage)?;
    Ok(owner)
}

#[cfg(test)]
mod tests {}
