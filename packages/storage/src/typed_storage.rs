//! This is a basic storage trait that implements storage reading/writing from/to a fixed storage key.
//! Other storage are meant to build on this. For ex. ItemStore
//! 
use std::{any::type_name};

use cosmwasm_std::{ReadonlyStorage, StdResult, StdError, Storage};
use secret_toolkit_serialization::Serde;
use serde::{Serialize, de::DeserializeOwned};

pub trait TypedStorage<T: Serialize + DeserializeOwned, Ser: Serde> {
    /// Returns the storage key
    fn as_slice(&self) -> &[u8];

    /// Returns StdResult<T> from retrieving the item with the specified key.  Returns a
    /// StdError::NotFound if there is no item with that key
    ///
    /// # Arguments
    ///
    /// * `storage` - a reference to the storage this item is in
    fn load_impl<S: ReadonlyStorage>(&self, storage: &S) -> StdResult<T> {
        Ser::deserialize(
            &storage
                .get(self.as_slice())
                .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
        )
    }

    /// Returns StdResult<Option<T>> from retrieving the item with the specified key.  Returns a
    /// None if there is no item with that key
    ///
    /// # Arguments
    ///
    /// * `storage` - a reference to the storage this item is in
    fn may_load_impl<S: ReadonlyStorage>(&self, storage: &S) -> StdResult<Option<T>> {
        match storage.get(self.as_slice()) {
            Some(value) => Ser::deserialize(&value).map(Some),
            None => Ok(None),
        }
    }

    /// Returns StdResult<()> resulting from saving an item to storage
    ///
    /// # Arguments
    ///
    /// * `storage` - a mutable reference to the storage this item should go to
    /// * `value` - a reference to the item to store
    fn save_impl<S: Storage>(&self, storage: &mut S, value: &T) -> StdResult<()> {
        storage.set(self.as_slice(), &Ser::serialize(value)?);
        Ok(())
    }

    /// Removes an item from storage
    ///
    /// # Arguments
    ///
    /// * `storage` - a mutable reference to the storage this item is in
    fn remove_impl<S: Storage>(&self, storage: &mut S) {
        storage.remove(self.as_slice());
    }
}