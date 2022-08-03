use std::marker::PhantomData;

use cosmwasm_std::{Storage, ReadonlyStorage, StdResult};
use secret_toolkit_serialization::{Serde, Bincode2};
use serde::{Serialize, de::DeserializeOwned};

use crate::typed_storage::TypedStorage;

/// This storage struct is based on Item from cosmwasm-storage-plus
pub struct Item<'a, T, Ser = Bincode2>
    where
        T: Serialize + DeserializeOwned,
        Ser: Serde,
{
    storage_key: &'a [u8],
    item_type: PhantomData<T>,
    serialization_type: PhantomData<Ser>,
}

impl<'a, T: Serialize + DeserializeOwned, Ser: Serde> Item<'a, T, Ser> {
    pub const fn new(key: &'a [u8]) -> Self {
        Self {
            storage_key: key,
            item_type: PhantomData,
            serialization_type: PhantomData,
        }
    }
}

impl<'a, T: Serialize + DeserializeOwned, Ser: Serde> TypedStorage<T, Ser> for Item<'a, T, Ser> {
    fn as_slice(&self) -> &[u8] {
        self.storage_key
    }
}

impl<'a, T, Ser> Item<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde
{
    /// save will serialize the model and store, returns an error on serialization issues
    pub fn save<S: Storage>(&self, storage: &mut S, data: &T) -> StdResult<()> {
        self.save_impl(storage, data)
    }

    pub fn remove<S: Storage>(&self, storage: &mut S) {
        self.remove_impl(storage);
    }

    /// load will return an error if no data is set at the given key, or on parse error
    pub fn load<S: ReadonlyStorage>(&self, storage: &S) -> StdResult<T> {
        self.load_impl(storage)
    }

    /// may_load will parse the data stored at the key if present, returns `Ok(None)` if no data there.
    /// returns an error on issues parsing
    pub fn may_load<S: ReadonlyStorage>(&self, storage: &S) -> StdResult<Option<T>> {
        self.may_load_impl(storage)
    }

    /// Loads the data, perform the specified action, and store the result
    /// in the database. This is shorthand for some common sequences, which may be useful.
    ///
    /// It assumes, that data was initialized before, and if it doesn't exist, `Err(StdError::NotFound)`
    /// is returned.
    pub fn update<S, A>(&self, storage: &mut S, action: A) -> StdResult<T>
    where
        S: Storage,
        A: FnOnce(T) -> StdResult<T>
    {
        let input = self.load_impl(storage)?;
        let output = action(input)?;
        self.save_impl(storage, &output)?;
        Ok(output)
    }
}
