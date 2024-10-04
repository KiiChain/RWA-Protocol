use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Addr};
use crate::state::{Key, Claim};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    AddKey {
        key_owner: String,
        key_type: String,
    },
    RevokeKey {
        key_owner: String,
        key_type: String,
    },
    AddClaim {
        claim: Claim,
        issuer_signature: Binary,
    },
    RemoveClaim {
        claim_id: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Key)]
    GetKey {
        key_owner: String,
        key_type: String,
    },
    #[returns(Claim)]
    GetClaim {
        key_owner: String,
        claim_id: String,
    },
    #[returns(Vec<String>)]
    GetClaimIdsByTopic {
        key_owner: String,
        topic: String,
    },
    #[returns(Vec<Claim>)]
    GetClaimsByIssuer {
        key_owner: String,
        issuer: String,
    },
    #[returns(bool)]
    VerifyClaim {
        key_owner: String,
        claim_id: String,
        trusted_issuers_registry: String,
    },
    #[returns(Addr)]
    GetOwner {},
}
