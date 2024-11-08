#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use roles::owner_roles::msg::OwnerRole;
// use cw2::set_contract_version;

use crate::claim_topics::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::claim_topics::ContractError;

use super::state::OWNER_ROLES_ADDRESS;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:claim_topics";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    OWNER_ROLES_ADDRESS.save(deps.storage, &msg.owner_roles_address)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // checking with the owner role contract to ensure only authorized personnel
    // with a role of ClaimRegistryManager are allowed to execute the functions
    execute::check_role(deps.as_ref(), info.sender, OwnerRole::ClaimRegistryManager)?;

    match msg {
        ExecuteMsg::AddClaimTopic { topic } => execute::add_claim_topic(deps, topic),
        ExecuteMsg::RemoveClaimTopic { topic } => execute::remove_claim_topic(deps, topic),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsClaimTopicValid { topic } => {
            to_json_binary(&query::is_claim_topic_valid(deps, topic)?)
        }
    }
}

pub mod execute {
    use crate::claim_topics::state::CLAIM_TOPICS;

    use super::*;
    use cosmwasm_std::{to_json_binary, Addr, QueryRequest, Uint128, WasmQuery};
    use roles::owner_roles::{msg::OwnerRole, QueryMsg};

    pub fn check_role(deps: Deps, owner: Addr, role: OwnerRole) -> Result<(), ContractError> {
        let owner_roles = OWNER_ROLES_ADDRESS.load(deps.storage)?;
        let msg = QueryMsg::IsOwner { role, owner };

        let query = QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: owner_roles.to_string(),
            msg: to_json_binary(&msg)?,
        });
        let has_role: bool = deps.querier.query(&query)?;
        if !has_role {
            return Err(ContractError::Unauthorized {});
        }
        Ok(())
    }

    pub fn add_claim_topic(deps: DepsMut, claim_topic: Uint128) -> Result<Response, ContractError> {
        if CLAIM_TOPICS.has(deps.storage, claim_topic.into()) {
            return Err(ContractError::ClaimTopicsExists {});
        }
        CLAIM_TOPICS.save(deps.storage, claim_topic.into(), &true)?;

        Ok(Response::new().add_attribute("action", "add_claim_topic"))
    }

    pub fn remove_claim_topic(
        deps: DepsMut,
        claim_topic: Uint128,
    ) -> Result<Response, ContractError> {
        if !CLAIM_TOPICS.has(deps.storage, claim_topic.into()) {
            return Err(ContractError::ClaimTopicsNotFound {});
        }
        CLAIM_TOPICS.remove(deps.storage, claim_topic.into());

        Ok(Response::new().add_attribute("action", "remove_claim_topic"))
    }
}
pub mod query {
    use cosmwasm_std::Uint128;

    use crate::claim_topics::state::CLAIM_TOPICS;

    use super::*;
    pub fn is_claim_topic_valid(deps: Deps, topic: Uint128) -> StdResult<bool> {
        Ok(CLAIM_TOPICS.has(deps.storage, topic.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{from_json, Addr, ContractResult, SystemResult, Uint128};
    use roles::owner_roles::msg::OwnerRole;
    use roles::owner_roles::QueryMsg;

    fn setup_contract(deps: DepsMut) -> Addr {
        let owner_roles_address = Addr::unchecked("owner_roles_contract");
        let msg = InstantiateMsg {
            owner_roles_address: owner_roles_address.clone(),
        };
        let info = message_info(&Addr::unchecked("creator"), &[]);
        let _ = instantiate(deps, mock_env(), info, msg).unwrap();
        owner_roles_address
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let owner_roles_address = setup_contract(deps.as_mut());

        // Check if the owner_roles_address is set correctly
        let stored_address = OWNER_ROLES_ADDRESS.load(&deps.storage).unwrap();
        assert_eq!(stored_address, owner_roles_address);
    }

    #[test]
    fn add_claim_topic() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Mock the owner roles contract query
        deps.querier.update_wasm(|query| match query {
            cosmwasm_std::WasmQuery::Smart { msg, .. } => {
                let parsed: QueryMsg = from_json(msg).unwrap();
                match parsed {
                    QueryMsg::IsOwner { role, .. } => {
                        if role == OwnerRole::ClaimRegistryManager {
                            SystemResult::Ok(ContractResult::Ok(to_json_binary(&true).unwrap()))
                        } else {
                            panic!("Unexpected role query")
                        }
                    }
                }
            }
            _ => panic!("Unexpected query type"),
        });

        let info = message_info(&Addr::unchecked("authorized_user"), &[]);
        let msg = ExecuteMsg::AddClaimTopic {
            topic: Uint128::new(1),
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes, vec![("action", "add_claim_topic")]);

        // Verify the topic was added
        let msg = crate::claim_topics::QueryMsg::IsClaimTopicValid {
            topic: Uint128::new(1),
        };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let is_valid: bool = from_json(res).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn remove_claim_topic() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Mock the owner roles contract query
        deps.querier.update_wasm(|query| match query {
            cosmwasm_std::WasmQuery::Smart { msg, .. } => {
                let parsed: QueryMsg = from_json(msg).unwrap();
                match parsed {
                    QueryMsg::IsOwner { role, .. } => {
                        if role == OwnerRole::ClaimRegistryManager {
                            SystemResult::Ok(ContractResult::Ok(to_json_binary(&true).unwrap()))
                        } else {
                            panic!("Unexpected role query")
                        }
                    }
                }
            }
            _ => panic!("Unexpected query type"),
        });

        let info = message_info(&Addr::unchecked("authorized_user"), &[]);

        // First, add a claim topic
        let msg = ExecuteMsg::AddClaimTopic {
            topic: Uint128::new(1),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Now remove the claim topic
        let msg = ExecuteMsg::RemoveClaimTopic {
            topic: Uint128::new(1),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes, vec![("action", "remove_claim_topic")]);

        // Verify the topic was removed
        let msg = crate::claim_topics::QueryMsg::IsClaimTopicValid {
            topic: Uint128::new(1),
        };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let is_valid: bool = from_json(res).unwrap();
        assert!(!is_valid);
    }
}
