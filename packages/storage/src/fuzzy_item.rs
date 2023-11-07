use cosmwasm_std::{StdResult, Storage};
use secret_toolkit_serialization::{Bincode2, Serde};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::Item;

#[derive(Serialize, Deserialize)]
pub struct FuzzyData<T> {
    data: T,
    fuzz: u8,
}

pub struct FuzzyItem<'a, T, Ser = Bincode2>
where
    T: Serialize + DeserializeOwned + Copy,
    Ser: Serde,
{
    item: Item<'a, FuzzyData<T>, Ser>,
    storage: &'a mut dyn Storage,
}

impl<'a, T: Serialize + DeserializeOwned + Copy, Ser: Serde> FuzzyItem<'a, T, Ser> {
    pub fn new(item: Item<'a, FuzzyData<T>, Ser>, storage: &'a mut dyn Storage) -> Self {
        Self { item, storage }
    }

    pub fn add_suffix(&'a mut self, suffix: &[u8]) -> Self {
        Self {
            item: self.item.add_suffix(suffix),
            storage: self.storage,
        }
    }
}

impl<'a, T, Ser> Drop for FuzzyItem<'a, T, Ser>
where
    T: Serialize + DeserializeOwned + Copy,
    Ser: Serde,
{
    fn drop(&mut self) {
        self.update(|fd| Ok(fd)).unwrap(); // This is not ideal but can't return `StdResult`
    }
}

impl<'a, T, Ser> FuzzyItem<'a, T, Ser>
where
    T: Serialize + DeserializeOwned + Copy,
    Ser: Serde,
{
    /// save will serialize the model and store, returns an error on serialization issues
    pub fn save(&mut self, data: &T) -> StdResult<()> {
        let new_data = match self.item.may_load(self.storage)? {
            Some(fd) => FuzzyData {
                data: *data,
                fuzz: fd.fuzz.wrapping_add(1),
            },
            None => FuzzyData {
                data: *data,
                fuzz: 0,
            },
        };

        self.item.save(self.storage, &new_data)
    }

    /// userfacing remove function
    pub fn remove(&mut self) {
        self.item.remove(self.storage)
    }

    /// load will return an error if no data is set at the given key, or on parse error
    pub fn load(&self) -> StdResult<T> {
        let fuzzy_data = self.item.load(self.storage)?;
        Ok(fuzzy_data.data)
    }

    /// may_load will parse the data stored at the key if present, returns `Ok(None)` if no data there.
    /// returns an error on issues parsing
    pub fn may_load(&self) -> StdResult<Option<T>> {
        let maybe_fuzzy_data = self.item.may_load(self.storage)?;
        Ok(maybe_fuzzy_data.map(|fd| fd.data))
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
        let mut fuzzy_data = self.item.load(self.storage)?;
        let input = fuzzy_data.data;
        fuzzy_data.data = action(input)?;
        fuzzy_data.fuzz = fuzzy_data.fuzz.wrapping_add(1);
        self.item.save(self.storage, &fuzzy_data)?;
        Ok(fuzzy_data.data)
    }
}
