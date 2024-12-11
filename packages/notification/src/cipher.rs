use chacha20poly1305::{
    aead::{AeadInPlace, KeyInit},
    ChaCha20Poly1305,
};
use cosmwasm_std::{StdError, StdResult};
use generic_array::GenericArray;

pub fn cipher_data(key: &[u8], nonce: &[u8], plaintext: &[u8], aad: &[u8]) -> StdResult<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| StdError::generic_err(format!("{:?}", e)))?;
    let mut buffer: Vec<u8> = plaintext.to_vec();
    cipher
        .encrypt_in_place(GenericArray::from_slice(nonce), aad, &mut buffer)
        .map_err(|e| StdError::generic_err(format!("{:?}", e)))?;
    Ok(buffer)
}

pub fn xor_bytes(vec1: &[u8], vec2: &[u8]) -> Vec<u8> {
    vec1.iter().zip(vec2.iter()).map(|(&a, &b)| a ^ b).collect()
}
