use cosmwasm_std::{StdResult, Storage};
use secret_toolkit_serialization::{Bincode2, Serde};
use serde::{de::DeserializeOwned, Serialize};

use crate::Item;

pub struct SecureItem<'a, T, Ser = Bincode2>
where
    T: Serialize + DeserializeOwned + Copy,
    Ser: Serde,
{
    item: Item<'a, T, Ser>,
    storage: &'a mut dyn Storage,
}

impl<'a, T: Serialize + DeserializeOwned + Copy, Ser: Serde> SecureItem<'a, T, Ser> {
    pub fn new(item: Item<'a, T, Ser>, storage: &'a mut dyn Storage) -> Self {
        Self { item, storage }
    }

    pub fn add_suffix(&'a mut self, suffix: &[u8]) -> Self {
        Self {
            item: self.item.add_suffix(suffix),
            storage: self.storage,
        }
    }
}

impl<'a, T, Ser> Drop for SecureItem<'a, T, Ser>
where
    T: Serialize + DeserializeOwned + Copy,
    Ser: Serde,
{
    fn drop(&mut self) {
        self.update(|data| Ok(data)).unwrap(); // This is not ideal but can't return `StdResult`
    }
}

impl<'a, T, Ser> SecureItem<'a, T, Ser>
where
    T: Serialize + DeserializeOwned + Copy,
    Ser: Serde,
{
    /// save will serialize the model and store, returns an error on serialization issues
    pub fn save(&mut self, data: &T) -> StdResult<()> {
        self.item.save(self.storage, data)
    }

    /// userfacing remove function
    pub fn remove(&mut self) {
        self.item.remove(self.storage)
    }

    /// load will return an error if no data is set at the given key, or on parse error
    pub fn load(&self) -> StdResult<T> {
        self.item.load(self.storage)
    }

    /// may_load will parse the data stored at the key if present, returns `Ok(None)` if no data there.
    /// returns an error on issues parsing
    pub fn may_load(&self) -> StdResult<Option<T>> {
        self.item.may_load(self.storage)
    }

    /// efficient way to see if any object is currently saved.
    pub fn is_empty(&self) -> bool {
        self.item.is_empty(self.storage)
    }

    /// Loads the data, perform the specified action, and store the result
    /// in the database. This is shorthand for some common sequences, which may be useful.
    ///
    /// It assumes, that data was initialized before, and if it doesn't exist, `Err(StdError::NotFound)`
    /// is returned.
    pub fn update<A>(&mut self, action: A) -> StdResult<T>
    where
        A: FnOnce(T) -> StdResult<T>,
    {
        self.item.update(self.storage, action)
    }
}
