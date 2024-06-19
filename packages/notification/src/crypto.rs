use chacha20poly1305::{
    aead::{AeadInPlace, KeyInit},
    ChaCha20Poly1305,
};
use cosmwasm_std::{StdError, StdResult};
use generic_array::GenericArray;
use hkdf::{hmac::Hmac, Hkdf};
use sha2::{Sha256, Sha512};

// Create alias for HMAC-SHA256
pub type HmacSha256 = Hmac<Sha256>;

pub fn cipher_data(key: &[u8], nonce: &[u8], plaintext: &[u8], aad: &[u8]) -> StdResult<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| StdError::generic_err(format!("{:?}", e)))?;
    let mut buffer: Vec<u8> = plaintext.to_vec();
    cipher
        .encrypt_in_place(GenericArray::from_slice(nonce), aad, &mut buffer)
        .map_err(|e| StdError::generic_err(format!("{:?}", e)))?;
    Ok(buffer)
}

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

pub fn xor_bytes(vec1: &[u8], vec2: &[u8]) -> Vec<u8> {
    vec1.iter().zip(vec2.iter()).map(|(&a, &b)| a ^ b).collect()
}
