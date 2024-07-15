use cosmwasm_std::{StdError, StdResult};
use hkdf::{hmac::Hmac, Hkdf};
use sha2::{Sha256, Sha512};

// Create alias for HMAC-SHA256
pub type HmacSha256 = Hmac<Sha256>;

pub fn hkdf_sha_256(
    salt: &Option<Vec<u8>>,
    ikm: &[u8],
    info: &[u8],
    length: usize,
) -> StdResult<Vec<u8>> {
    let hk: Hkdf<Sha256> = Hkdf::<Sha256>::new(salt.as_deref().map(|s| s), ikm);
    let mut zero_bytes = vec![0u8; length];
    let mut okm = zero_bytes.as_mut_slice();
    match hk.expand(info, &mut okm) {
        Ok(_) => Ok(okm.to_vec()),
        Err(e) => {
            return Err(StdError::generic_err(format!("{:?}", e)));
        }
    }
}

pub fn hkdf_sha_512(
    salt: &Option<Vec<u8>>,
    ikm: &[u8],
    info: &[u8],
    length: usize,
) -> StdResult<Vec<u8>> {
    let hk: Hkdf<Sha512> = Hkdf::<Sha512>::new(salt.as_deref().map(|s| s), ikm);
    let mut zero_bytes = vec![0u8; length];
    let mut okm = zero_bytes.as_mut_slice();
    match hk.expand(info, &mut okm) {
        Ok(_) => Ok(okm.to_vec()),
        Err(e) => {
            return Err(StdError::generic_err(format!("{:?}", e)));
        }
    }
}