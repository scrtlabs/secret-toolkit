use cosmwasm_std::{Binary, CanonicalAddr, StdResult};
use secret_toolkit_crypto::{sha_256, hkdf_sha_256, HmacSha256};
use hkdf::hmac::Mac;
use crate::cipher_data;

/// default notification block size in bytes
pub const NOTIFICATION_BLOCK_SIZE: usize = 36;
pub const SEED_LEN: usize = 32; // 256 bits

///
/// fn notification_id
///
///   Returns a notification id for the given address and channel id.
///
pub fn notification_id(seed: &Binary, channel: &str, tx_hash: &String) -> StdResult<Binary> {
    // compute notification ID for this event
    let material = [channel.as_bytes(), ":".as_bytes(), tx_hash.to_ascii_uppercase().as_bytes()].concat();

    let mut mac: HmacSha256 = HmacSha256::new_from_slice(seed.0.as_slice()).unwrap();
    mac.update(material.as_slice());
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    Ok(Binary::from(code_bytes.as_slice()))
}

///
/// fn encrypt_notification_data
///
///   Returns encrypted bytes given plaintext bytes, address, and channel id. 
///   Optionally, can set block size (default 36).
///
pub fn encrypt_notification_data(
    block_height: &u64,
    tx_hash: &String,
    seed: &Binary,
    channel: &str,
    plaintext: Vec<u8>,
    block_size: Option<usize>,
) -> StdResult<Binary> {
    let mut padded_plaintext = plaintext.clone();
    zero_pad(&mut padded_plaintext, block_size.unwrap_or(NOTIFICATION_BLOCK_SIZE));

    let channel_id_bytes = sha_256(channel.as_bytes())[..12].to_vec();
    let salt_bytes = hex::decode(tx_hash).unwrap()[..12].to_vec();
    let nonce: Vec<u8> = channel_id_bytes
        .iter()
        .zip(salt_bytes.iter())
        .map(|(&b1, &b2)| b1 ^ b2)
        .collect();
    let aad = format!("{}:{}", block_height, tx_hash);

    // encrypt notification data for this event
    let tag_ciphertext = cipher_data(
        seed.0.as_slice(),
        nonce.as_slice(),
        padded_plaintext.as_slice(),
        aad.as_bytes(),
    )?;

    Ok(Binary::from(tag_ciphertext.clone()))
}

/// get the seed for a secret and given address
pub fn get_seed(addr: &CanonicalAddr, secret: &[u8]) -> StdResult<Binary> {
    let seed = hkdf_sha_256(
        &None,
        secret,
        addr.as_slice(),
        SEED_LEN,
    )?;
    Ok(Binary::from(seed))
}

/// Take a Vec<u8> and pad it up to a multiple of `block_size`, using 0x00 at the end.
fn zero_pad(message: &mut Vec<u8>, block_size: usize) -> &mut Vec<u8> {
    let len = message.len();
    let surplus = len % block_size;
    if surplus == 0 {
        return message;
    }

    let missing = block_size - surplus;
    message.reserve(missing);
    message.extend(std::iter::repeat(0x00).take(missing));
    message
}

