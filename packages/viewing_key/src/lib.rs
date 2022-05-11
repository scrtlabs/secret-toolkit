extern crate core;

use subtle::ConstantTimeEq;

use cosmwasm_std::{Env, HumanAddr, ReadonlyStorage, StdError, StdResult, Storage};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};

use secret_toolkit_crypto::{sha_256, Prng, SHA256_HASH_SIZE};

pub const VIEWING_KEY_SIZE: usize = SHA256_HASH_SIZE;
pub const VIEWING_KEY_PREFIX: &str = "api_key_";
const SEED_KEY: &[u8] = b"::seed";

/// This is the default implementation of the viewing key store, using the "viewing_keys"
/// storage prefix.
///
/// You can use another storage location by implementing `ViewingKeyStore` for your own type.
pub struct ViewingKey;

impl ViewingKeyStore for ViewingKey {
    const STORAGE_KEY: &'static [u8] = b"viewing_keys";
}

/// A trait describing the interface of a Viewing Key store/vault.
///
/// It includes a default implementation that only requires specifying where in the storage
/// the keys should be held.
pub trait ViewingKeyStore {
    const STORAGE_KEY: &'static [u8];

    /// Set the initial prng seed for the store
    fn set_seed<S: Storage>(storage: &mut S, seed: &[u8]) {
        let mut seed_key = Vec::new();
        seed_key.extend_from_slice(Self::STORAGE_KEY);
        seed_key.extend_from_slice(SEED_KEY);

        storage.set(&seed_key, seed)
    }

    /// Create a new viewing key, save it to storage, and return it.
    ///
    /// The random entropy should be provided from some external source, such as the user.
    fn create<S: Storage>(
        storage: &mut S,
        env: &Env,
        account: &HumanAddr,
        entropy: &[u8],
    ) -> String {
        let mut seed_key = Vec::with_capacity(Self::STORAGE_KEY.len() + SEED_KEY.len());
        seed_key.extend_from_slice(Self::STORAGE_KEY);
        seed_key.extend_from_slice(SEED_KEY);
        let seed = storage.get(&seed_key).unwrap_or_default();

        let (viewing_key, next_seed) = new_viewing_key(env, &seed, entropy);
        let mut balance_store = PrefixedStorage::new(Self::STORAGE_KEY, storage);
        let hashed_key = sha_256(viewing_key.as_bytes());
        balance_store.set(account.as_str().as_bytes(), &hashed_key);

        storage.set(&seed_key, &next_seed);

        viewing_key
    }

    /// Set a new viewing key based on a predetermined value.
    fn set<S: Storage>(storage: &mut S, account: &HumanAddr, viewing_key: &str) {
        let mut balance_store = PrefixedStorage::new(Self::STORAGE_KEY, storage);
        balance_store.set(
            account.as_str().as_bytes(),
            &sha_256(viewing_key.as_bytes()),
        );
    }

    /// Check if a viewing key matches an account.
    fn check<S: ReadonlyStorage>(
        storage: &S,
        account: &HumanAddr,
        viewing_key: &str,
    ) -> StdResult<()> {
        let balance_store = ReadonlyPrefixedStorage::new(Self::STORAGE_KEY, storage);
        let expected_hash = balance_store.get(account.as_str().as_bytes());
        let expected_hash = match &expected_hash {
            Some(hash) => hash.as_slice(),
            None => &[0u8; VIEWING_KEY_SIZE],
        };
        let key_hash = sha_256(viewing_key.as_bytes());
        if ct_slice_compare(&key_hash, expected_hash) {
            Ok(())
        } else {
            Err(StdError::unauthorized())
        }
    }
}

fn new_viewing_key(env: &Env, seed: &[u8], entropy: &[u8]) -> (String, [u8; 32]) {
    // 16 here represents the lengths in bytes of the block height and time.
    let entropy_len = 16 + env.message.sender.len() + entropy.len();
    let mut rng_entropy = Vec::with_capacity(entropy_len);
    rng_entropy.extend_from_slice(&env.block.height.to_be_bytes());
    rng_entropy.extend_from_slice(&env.block.time.to_be_bytes());
    rng_entropy.extend_from_slice(env.message.sender.0.as_bytes());
    rng_entropy.extend_from_slice(entropy);

    let mut rng = Prng::new(seed, &rng_entropy);

    let rand_slice = rng.rand_bytes();

    let key = sha_256(&rand_slice);

    let viewing_key = VIEWING_KEY_PREFIX.to_string() + &base64::encode(&key);
    (viewing_key, rand_slice)
}

fn ct_slice_compare(s1: &[u8], s2: &[u8]) -> bool {
    bool::from(s1.ct_eq(s2))
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    #[test]
    fn test_viewing_keys() {
        let account = HumanAddr("user-1".to_string());

        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env(account.as_str(), &[]);

        // VK not set yet:
        let result = ViewingKey::check(&deps.storage, &account, "fake key");
        assert_eq!(result, Err(StdError::unauthorized()));

        ViewingKey::set_seed(&mut deps.storage, b"seed");
        let viewing_key = ViewingKey::create(&mut deps.storage, &env, &account, b"entropy");

        let result = ViewingKey::check(&deps.storage, &account, &viewing_key);
        assert_eq!(result, Ok(()));

        // Create a key with the same entropy. Check that it's different
        let viewing_key_2 = ViewingKey::create(&mut deps.storage, &env, &account, b"entropy");
        assert_ne!(viewing_key, viewing_key_2);

        // VK set to another key:
        let result = ViewingKey::check(&deps.storage, &account, "fake key");
        assert_eq!(result, Err(StdError::unauthorized()));

        let viewing_key = "custom key";

        ViewingKey::set(&mut deps.storage, &account, viewing_key);

        let result = ViewingKey::check(&deps.storage, &account, viewing_key);
        assert_eq!(result, Ok(()));

        // VK set to another key:
        let result = ViewingKey::check(&deps.storage, &account, "fake key");
        assert_eq!(result, Err(StdError::unauthorized()));
    }
}
