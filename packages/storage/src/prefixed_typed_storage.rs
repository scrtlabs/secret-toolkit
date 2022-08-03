//! This is a basic storage trait that implements storage reading/writing by apppending a prefix in front of the keys.
//! Other storage are meant to build on this. For ex. ItemStore
//! 

use std::{any::type_name};

use cosmwasm_std::{ReadonlyStorage, StdResult, StdError, Storage};
use secret_toolkit_serialization::Serde;
use serde::{Serialize, de::DeserializeOwned};

pub trait PrefixedTypedStorage<T: Serialize + DeserializeOwned, Ser: Serde> {
    fn as_slice(&self) -> &[u8];

    /// Returns StdResult<T> from retrieving the item with the specified key.  Returns a
    /// StdError::NotFound if there is no item with that key
    ///
    /// # Arguments
    ///
    /// * `storage` - a reference to the storage this item is in
    /// * `key` - a byte slice representing the key to access the stored item
    fn load_impl<S: ReadonlyStorage>(&self, storage: &S, key: &[u8]) -> StdResult<T> {
        let prefixed_key = [self.as_slice(), key].concat();
        Ser::deserialize(
            &storage
                .get(&prefixed_key)
                .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
        )
    }

    /// Returns StdResult<Option<T>> from retrieving the item with the specified key.  Returns a
    /// None if there is no item with that key
    ///
    /// # Arguments
    ///
    /// * `storage` - a reference to the storage this item is in
    /// * `key` - a byte slice representing the key to access the stored item
    fn may_load_impl<S: ReadonlyStorage>(&self, storage: &S, key: &[u8]) -> StdResult<Option<T>> {
        let prefixed_key = [self.as_slice(), key].concat();
        match storage.get(&prefixed_key) {
            Some(value) => Ser::deserialize(&value).map(Some),
            None => Ok(None),
        }
    }

    /// Returns StdResult<()> resulting from saving an item to storage
    ///
    /// # Arguments
    ///
    /// * `storage` - a mutable reference to the storage this item should go to
    /// * `key` - a byte slice representing the key to access the stored item
    /// * `value` - a reference to the item to store
    fn save_impl<S: Storage>(&self, storage: &mut S, key: &[u8], value: &T) -> StdResult<()> {
        let prefixed_key = [self.as_slice(), key].concat();
        storage.set(&prefixed_key, &Ser::serialize(value)?);
        Ok(())
    }

    /// Removes an item from storage
    ///
    /// # Arguments
    ///
    /// * `storage` - a mutable reference to the storage this item is in
    /// * `key` - a byte slice representing the key to access the stored item
    fn remove_impl<S: Storage>(&self, storage: &mut S, key: &[u8]) {
        let prefixed_key = [self.as_slice(), key].concat();
        storage.remove(&prefixed_key);
    }
}