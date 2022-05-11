use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Extern, HumanAddr, Querier, StdError, StdResult, Storage,
};
use ripemd160::{Digest, Ripemd160};

use crate::{Permissions, Permit, RevokedPermits, SignedPermit};
use bech32::{ToBase32, Variant};
use secret_toolkit_crypto::sha_256;

pub fn validate<Permission: Permissions, S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    storage_prefix: &str,
    permit: &Permit<Permission>,
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

    let base32_addr = pubkey_to_account(pubkey).0.as_slice().to_base32();
    let account: String = bech32::encode(account_hrp, &base32_addr, Variant::Bech32).unwrap();

    // Validate permit_name
    let permit_name = &permit.params.permit_name;
    let is_permit_revoked = RevokedPermits::is_permit_revoked(
        &deps.storage,
        storage_prefix,
        &HumanAddr(account.clone()),
        permit_name,
    );
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

    let verified = deps
        .api
        .secp256k1_verify(&signed_bytes_hash, &permit.signature.signature.0, &pubkey.0)
        .map_err(|err| StdError::generic_err(err.to_string()))?;

    if !verified {
        return Err(StdError::generic_err(
            "Failed to verify signatures for the given permit",
        ));
    }

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
    use crate::{PermitParams, PermitSignature, PubKey, TokenPermissions};
    use cosmwasm_std::testing::mock_dependencies;

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
                permissions: vec![TokenPermissions::History]
            },
            signature: PermitSignature {
                pub_key: PubKey {
                    r#type: "tendermint/PubKeySecp256k1".to_string(),
                    value: Binary::from_base64("A5M49l32ZrV+SDsPnoRv8fH7ivNC4gEX9prvd4RwvRaL").unwrap(),
                },
                signature: Binary::from_base64("hw/Mo3ZZYu1pEiDdymElFkuCuJzg9soDHw+4DxK7cL9rafiyykh7VynS+guotRAKXhfYMwCiyWmiznc6R+UlsQ==").unwrap()
            }
        };

        let address = validate(&deps, "test", &permit, token.clone(), Some("secret")).unwrap();

        assert_eq!(
            address,
            "secret1399pyvvk3hvwgxwt3udkslsc5jl3rqv4yshfrl".to_string()
        );

        let address = validate(&deps, "test", &permit, token, Some("cosmos")).unwrap();

        assert_eq!(
            address,
            "cosmos1399pyvvk3hvwgxwt3udkslsc5jl3rqv4x4rq7r".to_string()
        );
    }
}
