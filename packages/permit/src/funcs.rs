use std::u64;

use cosmwasm_std::{to_binary, Binary, CanonicalAddr, Deps, Env, StdError, StdResult, Timestamp};
use ripemd::{Digest, Ripemd160};
use secret_toolkit_utils::iso8601_utc0_to_timestamp;

use crate::{Permissions, Permit, RevokedPermits, RevokedPermitsStore, SignedPermit, BLANKET_PERMIT_TOKEN};
use bech32::{ToBase32, Variant};
use secret_toolkit_crypto::sha_256;

pub fn validate<Permission: Permissions>(
    deps: Deps,
    env: Env,
    permit: &Permit<Permission>,
    current_token_address: String,
    hrp: Option<&str>,
) -> StdResult<String> {
    let account_hrp = hrp.unwrap_or("secret");

    if permit.params.allowed_tokens.contains(&BLANKET_PERMIT_TOKEN.to_string()) {
        // using blanket permit
        
        // assert allowed_tokens list has an exact length of 1
        if permit.params.allowed_tokens.len() != 1 {
            return Err(StdError::generic_err("Blanket permits cannot contain other allowed tokens"));
        }

        // assert created field is specified
        if permit.params.created.is_none() {
            return Err(StdError::generic_err("Blanket permits must have a `created` time"));
        }
    } else if !permit.check_token(&current_token_address) {
        // check that current token address is in allowed tokens
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

    // Convert the permit created field to a Timestamp
    let created_timestamp = permit.params.created.clone()
        .map(|created| 
            iso8601_utc0_to_timestamp(&created)
        )
        .transpose()?;

    if let Some(created) = created_timestamp {
        // Verify that the permit was not created after the current block time
        if created > env.block.time {
            return Err(StdError::generic_err("Permit `created` after current block time"));
        }
    }

    // Convert the permit expires field to a Timestamp
    let expires_timestamp = permit.params.expires.clone()
        .map(|created| 
            iso8601_utc0_to_timestamp(&created)
        )
        .transpose()?;

    if let Some(expires) = expires_timestamp {
        // Verify that the permit did not expire before the current block time
        if expires <= env.block.time {
            return Err(StdError::generic_err("Permit has expired"))
        }
    }

    // Derive account from pubkey
    let pubkey = &permit.signature.pub_key.value;

    let base32_addr = pubkey_to_account(pubkey).0.as_slice().to_base32();
    let account: String = bech32::encode(account_hrp, base32_addr, Variant::Bech32).unwrap();

    // Get the list of all revocations for this address
    let revocations = RevokedPermits::list_revocations(deps.storage, &account)?;

    // Check if there are any revocation intervals blocking all permits
    //   TODO: An interval or segment tree might be preferable to make this more efficient for cases 
    //         when the number of revocations is allowed to grow to a large amount.
    for revocation in revocations {
        // If this revocation has no `created_before` or `created_after`, then reject all permit queries
        if revocation.interval.created_before.is_none() && revocation.interval.created_after.is_none() {
            return Err(StdError::generic_err(
                format!("Permits revoked by {:?}", account.as_str())
            ));
        }

        // If the permit has a `created` field
        if let Some(created) = created_timestamp {
            // Revocation created before field, default 0
            let created_before = revocation.interval.created_before.unwrap_or(Timestamp::from_nanos(0));

            // Revocation created after field, default max u64
            let created_after = revocation.interval.created_after.unwrap_or(Timestamp::from_nanos(u64::MAX));

            // If the permit's `created` field falls in between created after and created before, then reject it
            if created > created_after || created < created_before {
                return Err(StdError::generic_err(
                    format!("Permits created at {:?} revoked by account {:?}", created, account.as_str())
                ));                
            }         
        }
    }

    // Validate permit_name
    let permit_name = &permit.params.permit_name;
    let is_permit_revoked =
        RevokedPermits::is_permit_revoked(deps.storage, &account, permit_name);
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
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    #[test]
    fn test_verify_permit() {
        let deps = mock_dependencies();

        //{"permit": {"params":{"chain_id":"pulsar-2","permit_name":"memo_secret1rf03820fp8gngzg2w02vd30ns78qkc8rg8dxaq","allowed_tokens":["secret1rf03820fp8gngzg2w02vd30ns78qkc8rg8dxaq"],"permissions":["history"]},"signature":{"pub_key":{"type":"tendermint/PubKeySecp256k1","value":"A5M49l32ZrV+SDsPnoRv8fH7ivNC4gEX9prvd4RwvRaL"},"signature":"hw/Mo3ZZYu1pEiDdymElFkuCuJzg9soDHw+4DxK7cL9rafiyykh7VynS+guotRAKXhfYMwCiyWmiznc6R+UlsQ=="}}}

        let token = "secret1rf03820fp8gngzg2w02vd30ns78qkc8rg8dxaq".to_string();

        let permit: Permit = Permit{
            params: PermitParams {
                allowed_tokens: vec![token.clone()],
                permit_name: "memo_secret1rf03820fp8gngzg2w02vd30ns78qkc8rg8dxaq".to_string(),
                chain_id: "pulsar-2".to_string(),
                permissions: vec![TokenPermissions::History],
                created: None,
                expires: None,
            },
            signature: PermitSignature {
                pub_key: PubKey {
                    r#type: "tendermint/PubKeySecp256k1".to_string(),
                    value: Binary::from_base64("A5M49l32ZrV+SDsPnoRv8fH7ivNC4gEX9prvd4RwvRaL").unwrap(),

                },
                signature: Binary::from_base64("hw/Mo3ZZYu1pEiDdymElFkuCuJzg9soDHw+4DxK7cL9rafiyykh7VynS+guotRAKXhfYMwCiyWmiznc6R+UlsQ==").unwrap()                
            }
        };

        let env = mock_env();

        let address = validate::<_>(
            deps.as_ref(),
            env,
            &permit,
            token.clone(),
            Some("secret"),
        )
        .unwrap();

        assert_eq!(
            address,
            "secret1399pyvvk3hvwgxwt3udkslsc5jl3rqv4yshfrl".to_string()
        );

        let env = mock_env();

        let address = validate::<_>(
            deps.as_ref(), 
            env, 
            &permit, 
            token, 
            Some("cosmos")
        ).unwrap();

        assert_eq!(
            address,
            "cosmos1399pyvvk3hvwgxwt3udkslsc5jl3rqv4x4rq7r".to_string()
        );
    }

    #[test]
    fn test_verify_permit_created_expires() {
        let deps = mock_dependencies();

        // test both created and expired set

        //{"permit": {"params":{"chain_id":"secret-4","permit_name":"test","allowed_tokens":["secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg"],"permissions":["balance"],"created":"2024-12-17T16:59:00.000Z","expires":"2024-12-20T06:59:30.333Z"},"signature":{"pub_key":{"type":"tendermint/PubKeySecp256k1","value":"AwSFyMndr25JX03rGSlXQ5oSO6F+9GoqQILZu/DytRrr"},"signature":"mFDn5w59gaDTHZ5UzEA6l+sUOtlWDx/HcSi1NpZM13YuamMehIi3mseqXcQy4loE63N0hYhyXiVZdzrPM28A+g=="}}}

        let token = "secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg".to_string();

        let permit: Permit = Permit{
            params: PermitParams {
                allowed_tokens: vec![token.clone()],
                permit_name: "test".to_string(),
                chain_id: "secret-4".to_string(),
                permissions: vec![TokenPermissions::Balance],
                created: Some("2024-12-17T16:59:00.000Z".to_string()),
                expires: Some("2024-12-20T06:59:30.333Z".to_string()),
            },
            // {"pub_key":{"type":"tendermint/PubKeySecp256k1","value":"AwSFyMndr25JX03rGSlXQ5oSO6F+9GoqQILZu/DytRrr"},"signature":"mFDn5w59gaDTHZ5UzEA6l+sUOtlWDx/HcSi1NpZM13YuamMehIi3mseqXcQy4loE63N0hYhyXiVZdzrPM28A+g=="}
            signature: PermitSignature {
                pub_key: PubKey {
                    r#type: "tendermint/PubKeySecp256k1".to_string(),
                    value: Binary::from_base64("AwSFyMndr25JX03rGSlXQ5oSO6F+9GoqQILZu/DytRrr").unwrap(),
                },
                signature: Binary::from_base64("mFDn5w59gaDTHZ5UzEA6l+sUOtlWDx/HcSi1NpZM13YuamMehIi3mseqXcQy4loE63N0hYhyXiVZdzrPM28A+g==").unwrap()
            }
        };

        let created_seconds: u64 = 1734454740;
        let expires_seconds: u64 = 1734677970;

        // validate after created, before expires

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(created_seconds + 100);

        // secret16v498l7d335wlzxpzg0mwkucrszdlza008dhc9
        let address = validate::<_>(
            deps.as_ref(),
            env,
            &permit,
            token.clone(),
            Some("secret"),
        ).unwrap();

        assert_eq!(
            address,
            "secret16v498l7d335wlzxpzg0mwkucrszdlza008dhc9".to_string()
        );

        // validate before created

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(created_seconds - 100);

        let address = validate::<_>(
            deps.as_ref(),
            env,
            &permit,
            token.clone(),
            Some("secret"),
        );

        assert!(address.is_err(), "validated before created");

        // validate after expires

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(expires_seconds + 100);

        let address = validate::<_>(
            deps.as_ref(),
            env,
            &permit,
            token.clone(),
            Some("secret"),
        );

        assert!(address.is_err(), "validated after expires");

    }

    #[test]
    fn test_verify_blanket_permit() {
        let deps = mock_dependencies();

        // blanket permit

        //{"permit": {"params":{"chain_id":"secret-4","permit_name":"test","allowed_tokens":["ANY_TOKEN"],"permissions":["balance"],"created":"2024-12-17T16:59:00.000Z","expires":"2024-12-20T06:59:30.333Z"},"signature":{"pub_key":{"type":"tendermint/PubKeySecp256k1","value":"AwSFyMndr25JX03rGSlXQ5oSO6F+9GoqQILZu/DytRrr"},"signature":"Qte1iS54RsyRCN3rmOjA96yXQTn+eg4YaEUAU/Q5mLVGU9mOCEw6LMZjU2owLB4ogcziWrMkLOL3dtOrj3dL4Q=="}}}

        let token = BLANKET_PERMIT_TOKEN.to_string();

        let permit: Permit = Permit{
            params: PermitParams {
                allowed_tokens: vec![token.clone()],
                permit_name: "test".to_string(),
                chain_id: "secret-4".to_string(),
                permissions: vec![TokenPermissions::Balance],
                created: Some("2024-12-17T16:59:00.000Z".to_string()),
                expires: Some("2024-12-20T06:59:30.333Z".to_string()),
            },
            signature: PermitSignature {
                pub_key: PubKey {
                    r#type: "tendermint/PubKeySecp256k1".to_string(),
                    value: Binary::from_base64("AwSFyMndr25JX03rGSlXQ5oSO6F+9GoqQILZu/DytRrr").unwrap(),
                },
                signature: Binary::from_base64("Qte1iS54RsyRCN3rmOjA96yXQTn+eg4YaEUAU/Q5mLVGU9mOCEw6LMZjU2owLB4ogcziWrMkLOL3dtOrj3dL4Q==").unwrap()
            }
        };

        let created_seconds: u64 = 1734454740;
        let expires_seconds: u64 = 1734677970;

        // validate after created, before expires

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(created_seconds + 100);

        // secret16v498l7d335wlzxpzg0mwkucrszdlza008dhc9
        let address = validate::<_>(
            deps.as_ref(),
            env,
            &permit,
            token.clone(),
            Some("secret"),
        ).unwrap();

        assert_eq!(
            address,
            "secret16v498l7d335wlzxpzg0mwkucrszdlza008dhc9".to_string()
        );

        // validate before created

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(created_seconds - 100);

        let address = validate::<_>(
            deps.as_ref(),
            env,
            &permit,
            token.clone(),
            Some("secret"),
        );

        assert!(address.is_err(), "validated before created");

        // validate after expires

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(expires_seconds + 100);

        let address = validate::<_>(
            deps.as_ref(),
            env,
            &permit,
            token.clone(),
            Some("secret"),
        );

        assert!(address.is_err(), "validated after expires");

        // blanket permit invalid with another token in addition to ANY_TOKEN

        //{"permit": {"params":{"chain_id":"secret-4","permit_name":"test","allowed_tokens":["secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg","ANY_TOKEN"],"permissions":["balance"],"created":"2024-12-17T16:59:00.000Z","expires":"2024-12-20T06:59:30.333Z"},"signature":{"pub_key":{"type":"tendermint/PubKeySecp256k1","value":"AwSFyMndr25JX03rGSlXQ5oSO6F+9GoqQILZu/DytRrr"},"signature":"vc36PM85beBIOmimreAD428O3ldyyUqNHxmzUYlsHaJ+560Ce8G5ibJR7KCvHJitRuds/3TvGX4dPp6l6xfrUg=="}}}

        let token = BLANKET_PERMIT_TOKEN.to_string();

        let permit: Permit = Permit{
            params: PermitParams {
                allowed_tokens: vec!["secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg".to_string(), token.clone()],
                permit_name: "test".to_string(),
                chain_id: "secret-4".to_string(),
                permissions: vec![TokenPermissions::Balance],
                created: Some("2024-12-17T16:59:00.000Z".to_string()),
                expires: Some("2024-12-20T06:59:30.333Z".to_string()),
            },
            signature: PermitSignature {
                pub_key: PubKey {
                    r#type: "tendermint/PubKeySecp256k1".to_string(),
                    value: Binary::from_base64("AwSFyMndr25JX03rGSlXQ5oSO6F+9GoqQILZu/DytRrr").unwrap(),
                },
                signature: Binary::from_base64("vc36PM85beBIOmimreAD428O3ldyyUqNHxmzUYlsHaJ+560Ce8G5ibJR7KCvHJitRuds/3TvGX4dPp6l6xfrUg==").unwrap()
            }
        };

        let created_seconds: u64 = 1734454740;

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(created_seconds + 100);

        // secret16v498l7d335wlzxpzg0mwkucrszdlza008dhc9
        let address = validate::<_>(
            deps.as_ref(),
            env,
            &permit,
            token.clone(),
            Some("secret"),
        );

        assert!(address.is_err(), "passed with second token in addition to ANY_TOKEN");

        // blanket permit invalid with no created

        //{"permit": {"params":{"chain_id":"secret-4","permit_name":"test","allowed_tokens":["ANY_TOKEN"],"permissions":["balance"],"expires":"2024-12-20T06:59:30.333Z"},"signature":{"pub_key":{"type":"tendermint/PubKeySecp256k1","value":"AwSFyMndr25JX03rGSlXQ5oSO6F+9GoqQILZu/DytRrr"},"signature":"k2tdjChWUeIfs63qcHwzUdt1C92gQ5lwvEPS4fv7GpM2geaWGpUsy6Ne+m0pda0AJEpdbiZ38KjiKNlU3CmkOw=="}}}

        let token = BLANKET_PERMIT_TOKEN.to_string();

        let permit: Permit = Permit{
            params: PermitParams {
                allowed_tokens: vec![token.clone()],
                permit_name: "test".to_string(),
                chain_id: "secret-4".to_string(),
                permissions: vec![TokenPermissions::Balance],
                created: None,
                expires: Some("2024-12-20T06:59:30.333Z".to_string()),
            },
            signature: PermitSignature {
                pub_key: PubKey {
                    r#type: "tendermint/PubKeySecp256k1".to_string(),
                    value: Binary::from_base64("AwSFyMndr25JX03rGSlXQ5oSO6F+9GoqQILZu/DytRrr").unwrap(),
                },
                signature: Binary::from_base64("k2tdjChWUeIfs63qcHwzUdt1C92gQ5lwvEPS4fv7GpM2geaWGpUsy6Ne+m0pda0AJEpdbiZ38KjiKNlU3CmkOw==").unwrap()
            }
        };

        let created_seconds: u64 = 1734454740;

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(created_seconds + 100);

        // secret16v498l7d335wlzxpzg0mwkucrszdlza008dhc9
        let address = validate::<_>(
            deps.as_ref(),
            env,
            &permit,
            token.clone(),
            Some("secret"),
        );

        assert!(address.is_err(), "blanket permit passed with no created field");
    }
}
