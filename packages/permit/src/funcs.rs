use crate::*;
use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Extern, HumanAddr, Querier, StdError, StdResult, Storage,
};
use ripemd160::{Digest, Ripemd160};
use secp256k1::Secp256k1;
use sha2::Sha256;

fn validate_permit<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    permit: &Permit,
    current_token_address: HumanAddr,
) -> StdResult<HumanAddr> {
    if !permit
        .params
        .allowed_tokens
        .contains(&current_token_address)
    {
        return Err(StdError::generic_err(format!(
            "Permit doesn't apply to token {:?}, allowed tokens: {:?}",
            current_token_address.as_str(),
            permit
                .params
                .allowed_tokens
                .iter()
                .map(|a| a.as_str())
                .collect::<Vec<&str>>()
        )));
    }

    // Derive account from pubkey
    let pubkey = &permit.signature.pub_key.value;
    let account = deps.api.human_address(&pubkey_to_account(pubkey))?;

    // Validate permit_name
    let permit_name = &permit.params.permit_name;
    let is_permit_revoked = RevokedPemits::is_permit_revoked(&deps.storage, &account, permit_name);
    if is_permit_revoked {
        return Err(StdError::generic_err(format!(
            "Permit {:?} was revoked by account {:?}",
            permit_name,
            account.as_str()
        )));
    }

    // Validate signature, reference: https://github.com/enigmampc/SecretNetwork/blob/f591ed0cb3af28608df3bf19d6cfb733cca48100/cosmwasm/packages/wasmi-runtime/src/crypto/secp256k1.rs#L49-L82
    let signed_bytes = to_binary(&SignedPermit::from_params(&permit.params))?;
    let signed_bytes_hash = Sha256::digest(signed_bytes.as_slice());
    let secp256k1_msg =
        secp256k1::Message::from_slice(signed_bytes_hash.as_slice()).map_err(|err| {
            StdError::generic_err(format!(
                "Failed to create a secp256k1 message from signed_bytes: {:?}",
                err
            ))
        })?;

    let secp256k1_verifier = Secp256k1::verification_only();

    let secp256k1_signature = secp256k1::Signature::from_compact(&permit.signature.signature.0)
        .map_err(|err| StdError::generic_err(format!("Malformed signature: {:?}", err)))?;
    let secp256k1_pubkey = secp256k1::PublicKey::from_slice(pubkey.0.as_slice())
        .map_err(|err| StdError::generic_err(format!("Malformed pubkey: {:?}", err)))?;

    secp256k1_verifier
        .verify(&secp256k1_msg, &secp256k1_signature, &secp256k1_pubkey)
        .map_err(|err| {
            StdError::generic_err(format!(
                "Failed to verify signatures for the given permit: {:?}",
                err
            ))
        })?;

    Ok(account)
}

fn pubkey_to_account(pubkey: &Binary) -> CanonicalAddr {
    let mut hasher = Ripemd160::new();
    hasher.update(Sha256::digest(&pubkey.0));
    CanonicalAddr(Binary(hasher.finalize().to_vec()))
}
