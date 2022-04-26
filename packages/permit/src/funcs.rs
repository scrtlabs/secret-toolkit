use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Extern, HumanAddr, Querier, StdError, StdResult, Storage,
};
use ripemd160::{Digest, Ripemd160};

use crate::{Permit, RevokedPermits, SignedPermit};

pub fn validate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    storage_prefix: &str,
    permit: &Permit,
    current_token_address: HumanAddr,
) -> StdResult<HumanAddr> {
    if !permit.check_token(&current_token_address) {
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
    let is_permit_revoked =
        RevokedPermits::is_permit_revoked(&deps.storage, storage_prefix, &account, permit_name);
    if is_permit_revoked {
        return Err(StdError::generic_err(format!(
            "Permit {:?} was revoked by account {:?}",
            permit_name,
            account.as_str()
        )));
    }

    // Verify signature
    let signed_bytes = to_binary(&SignedPermit::from_params(&permit.params))?;
    let signed_bytes_hash = secret_toolkit_crypto::sha_256(signed_bytes.as_slice());
    let verified = deps.api.secp256k1_verify(
        signed_bytes_hash.as_slice(),
        permit.signature.signature.0.as_slice(),
        pubkey.as_slice(),
    ).map_err(|err| {
        StdError::generic_err(format!("{:?}", err))
    })?;

    if !verified {
        return Err(StdError::generic_err(format!("Permit {:?} signature was not verified", permit_name)));
    }
    Ok(account)
}

pub fn pubkey_to_account(pubkey: &Binary) -> CanonicalAddr {
    let mut hasher = Ripemd160::new();
    hasher.update(secret_toolkit_crypto::sha_256(&pubkey.0));
    CanonicalAddr(Binary(hasher.finalize().to_vec()))
}
