use cosmwasm_std::{DepsMut, MessageInfo, Response, Binary, Deps, Addr};
use crate::error::ContractError;
use crate::state::{Claim, ClaimTopic, KeyType, IDENTITY};
use crate::utils::check_key_authorization;
use sha2::{Sha256, Digest};

pub fn add_claim(
    deps: DepsMut,
    info: MessageInfo,
    mut claim: Claim,
    issuer_signature: Binary,
) -> Result<Response, ContractError> {
    // Check if the sender is authorized to add claims (must have a MANAGEMENT_KEY)
    check_key_authorization(&deps, &info.sender, KeyType::ManagementKey)?;

    // Verify the issuer's signature (must be signed by a CLAIM_SIGNER_KEY)
    verify_claim_signature(&deps, &claim, &issuer_signature)?;

    let mut identity = IDENTITY.load(deps.storage, &info.sender)?;
    
    // Generate and set the claim ID
    generate_claim_id(&mut claim);
    
    // Check if the claim already exists
    if identity.claims.iter().any(|c| c.id == claim.id) {
        return Err(ContractError::ClaimAlreadyExists {});
    }

    identity.claims.push(claim.clone());
    IDENTITY.save(deps.storage, &info.sender, &identity)?;

    Ok(Response::new()
        .add_attribute("method", "add_claim")
        .add_attribute("claim_id", claim.id.unwrap_or_default()))
}

pub fn remove_claim(
    deps: DepsMut,
    info: MessageInfo,
    claim_id: String,
) -> Result<Response, ContractError> {
    // Check if the sender is authorized to remove claims (must have a MANAGEMENT_KEY)
    check_key_authorization(&deps, &info.sender, KeyType::ManagementKey)?;

    let mut identity = IDENTITY.load(deps.storage, &info.sender)?;

    // Find and remove the claim
    identity.claims.retain(|c| c.id.as_ref() != Some(&claim_id));

    IDENTITY.save(deps.storage, &info.sender, &identity)?;

    Ok(Response::new()
        .add_attribute("method", "remove_claim")
        .add_attribute("claim_id", claim_id))
}

fn verify_claim_signature(deps: &DepsMut, claim: &Claim, signature: &Binary) -> Result<(), ContractError> {
    let issuer_identity = IDENTITY.load(deps.storage, &claim.issuer)?;
    
    // Find a CLAIM_SIGNER_KEY for the issuer
    let claim_signer_key = issuer_identity.keys.iter()
        .find(|k| k.key_type == KeyType::ClaimSignerKey)
        .ok_or(ContractError::Unauthorized {})?;

    // Serialize the claim data
    let claim_data = serde_json::to_vec(claim).map_err(|_| ContractError::SerializationError {})?;

    // Hash the claim data
    let message_hash = Sha256::digest(&claim_data);

    // Verify the signature using the CLAIM_SIGNER_KEY
    let public_key = &claim_signer_key.owner.as_bytes();
    let signature = signature.as_slice();

    // Use cosmwasm_std::secp256k1_verify for signature verification
    let valid = deps.api.secp256k1_verify(message_hash.as_slice(), signature, public_key)
        .map_err(|_| ContractError::InvalidIssuerSignature {})?;

    if !valid {
        return Err(ContractError::InvalidIssuerSignature {});
    }

    Ok(())
}

pub fn verify_claim(
    deps: Deps,
    identity: Addr,
    claim_topic: ClaimTopic,
) -> Result<bool, ContractError> {
    let identity = IDENTITY.load(deps.storage, &identity)?;
    
    for claim in &identity.claims {
        if claim.topic == claim_topic {
            // Here you might want to add additional verification steps,
            // such as checking if the claim is still valid (not expired)
            return Ok(true);
        }
    }

    Ok(false)
}

// Helper function to generate a unique claim ID and set it in the claim
fn generate_claim_id(claim: &mut Claim) {
    let mut hasher = Sha256::new();
    hasher.update(claim.topic.to_string().as_bytes());
    hasher.update(&claim.issuer.as_bytes());
    hasher.update(&claim.signature);
    hasher.update(&claim.data);
    hasher.update(&claim.uri.to_string().as_bytes());
    let claim_id = hex::encode(hasher.finalize());
    claim.id = Some(claim_id);
}
