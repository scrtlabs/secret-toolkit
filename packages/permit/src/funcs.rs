use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Extern, HumanAddr, Querier, StdError, StdResult, Storage,
};
use ripemd160::{Digest, Ripemd160};
use secp256k1::{PublicKey, Message, ecdsa::Signature as Signature, Secp256k1};
use sha2::Sha256;
// use secp256k1::{PublicKey, Message as SecpMessage, Signature, verify};
use bech32::{ToBase32, Variant};
use crate::{Permit, RevokedPermits, SignedPermit};


pub fn sha_256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_slice());
    result
}

pub fn validate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    storage_prefix: &str,
    permit: &Permit,
    current_token_address: HumanAddr,
    hrp: Option<&str>,
) -> StdResult<String> {

    let account_hrp = hrp.unwrap_or("secret");

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
    // let account = deps.api.human_address(&pubkey_to_account(pubkey))?.0;
    let account: String = bech32::encode(account_hrp, &pubkey_to_account(pubkey).0.as_slice().to_base32(), Variant::Bech32).unwrap();
    // Validate permit_name
    let permit_name = &permit.params.permit_name;
    let is_permit_revoked =
        RevokedPermits::is_permit_revoked(&deps.storage, storage_prefix, &HumanAddr(account.clone()), permit_name);
    if is_permit_revoked {
        return Err(StdError::generic_err(format!(
            "Permit {:?} was revoked by account {:?}",
            permit_name,
            account.as_str()
        )));
    }

    // Validate signature, reference: https://github.com/enigmampc/SecretNetwork/blob/f591ed0cb3af28608df3bf19d6cfb733cca48100/cosmwasm/packages/wasmi-runtime/src/crypto/secp256k1.rs#L49-L82
    let signed_bytes = to_binary(&SignedPermit::from_params(&permit.params))?;
    let signed_bytes_hash = sha_256(signed_bytes.as_slice());
    //let secp256k1_msg = SecpMessage::parse_slice(&signed_bytes_hash).map_err(|err| {
    let secp256k1_msg = Message::from_slice(&signed_bytes_hash).map_err(|err| {
        StdError::generic_err(format!(
            "Failed to create a secp256k1 message from signed_bytes: {:?}",
            err
        ))
    })?;

    let secp256k1_verifier = Secp256k1::verification_only();

    //let secp256k1_signature = Signature::parse_standard_slice(&permit.signature.signature.0)
    let secp256k1_signature = Signature::from_compact(&permit.signature.signature.0)
        .map_err(|err| StdError::generic_err(format!("Malformed signature: {:?}", err)))?;
    //let secp256k1_pubkey = PublicKey::parse_slice(pubkey.0.as_slice(), None)
    let secp256k1_pubkey = PublicKey::from_slice(pubkey.0.as_slice())
        .map_err(|err| StdError::generic_err(format!("Malformed pubkey: {:?}", err)))?;

    // if !verify(&secp256k1_msg, &secp256k1_signature, &secp256k1_pubkey) {
    //     return Err(StdError::generic_err(format!(
    //         "Failed to verify signatures for the given permit",
    //     )));
    // }
    secp256k1_verifier.verify_ecdsa(&secp256k1_msg, &secp256k1_signature, &secp256k1_pubkey)
        .map_err(|err| {
            StdError::generic_err(format!(
                "Failed to verify signatures for the given permit: {:?}",
                err
            ))
        })?;

    Ok(account)
}

pub fn pubkey_to_account(pubkey: &Binary) -> CanonicalAddr {
    let mut hasher = Ripemd160::new();
    hasher.update(sha_256(&pubkey.0));
    CanonicalAddr(Binary(hasher.finalize().to_vec()))
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use crate::{Permission, PermitParams, PermitSignature, PubKey};

    #[test]
    fn test_verify_permit() {
        let deps = mock_dependencies(20, &[]);

        //{"permit": {"params":{"chain_id":"pulsar-2","permit_name":"memo_secret1rf03820fp8gngzg2w02vd30ns78qkc8rg8dxaq","allowed_tokens":["secret1rf03820fp8gngzg2w02vd30ns78qkc8rg8dxaq"],"permissions":["history"]},"signature":{"pub_key":{"type":"tendermint/PubKeySecp256k1","value":"A5M49l32ZrV+SDsPnoRv8fH7ivNC4gEX9prvd4RwvRaL"},"signature":"hw/Mo3ZZYu1pEiDdymElFkuCuJzg9soDHw+4DxK7cL9rafiyykh7VynS+guotRAKXhfYMwCiyWmiznc6R+UlsQ=="}}}

        let token = HumanAddr("secret1rf03820fp8gngzg2w02vd30ns78qkc8rg8dxaq".to_string());

        let permit: Permit = Permit{
            params: PermitParams {
                allowed_tokens: vec![token.clone()],
                permit_name: "memo_secret1rf03820fp8gngzg2w02vd30ns78qkc8rg8dxaq".to_string(),
                chain_id: "pulsar-2".to_string(),
                permissions: vec![Permission::History]
            },
            signature: PermitSignature {
                pub_key: PubKey {
                    r#type: "tendermint/PubKeySecp256k1".to_string(),
                    value: Binary::from_base64("A5M49l32ZrV+SDsPnoRv8fH7ivNC4gEX9prvd4RwvRaL").unwrap(),
                },
                signature: Binary::from_base64("hw/Mo3ZZYu1pEiDdymElFkuCuJzg9soDHw+4DxK7cL9rafiyykh7VynS+guotRAKXhfYMwCiyWmiznc6R+UlsQ==").unwrap()
            }
        };

        let address = validate(
            &deps,
            "test",
            &permit,
            token.clone(),
            Some("secret")).unwrap();

        assert_eq!(address, "secret1399pyvvk3hvwgxwt3udkslsc5jl3rqv4yshfrl".to_string());

        let address = validate(
            &deps,
            "test",
            &permit,
            token,
            Some("cosmos")).unwrap();

        assert_eq!(address, "cosmos1399pyvvk3hvwgxwt3udkslsc5jl3rqv4x4rq7r".to_string());
    }
}