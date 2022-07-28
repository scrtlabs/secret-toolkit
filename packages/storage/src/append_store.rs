//! An "append store" is a storage wrapper that guarantees constant-cost appending to and popping
//! from a list of items in storage.
//!
//! This is achieved by storing each item in a separate storage entry. A special key is reserved
//! for storing the length of the collection so far.
use std::convert::TryInto;
use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{ReadonlyStorage, StdError, StdResult, Storage};
use cosmwasm_storage::PrefixedStorage;

use secret_toolkit_serialization::{Bincode2, Serde};

const LEN_KEY: &[u8] = b"len";

// Mutable append-store

/// A type allowing both reads from and writes to the append store at a given storage location.
// #[derive(Debug)]
pub struct AppendStoreMut<'a, T, Ser = Bincode2>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    storage: Box<dyn Storage + 'a>,
    item_type: PhantomData<*const T>,
    serialization_type: PhantomData<*const Ser>,
    len: u32,
}

impl<'a, T> AppendStoreMut<'a, T, Bincode2>
where
    T: Serialize + DeserializeOwned,
{
    /// Try to use the provided storage as an AppendStore. If it doesn't seem to be one, then
    /// initialize it as one.
    ///
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_or_create<S: Storage>(storage: &'a mut S) -> StdResult<Self> {
        AppendStoreMut::attach_or_create_with_serialization(storage, Bincode2)
    }

    /// Try to use the provided storage as an AppendStore.
    ///
    /// Returns None if the provided storage doesn't seem like an AppendStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach<S: Storage>(storage: &'a mut S) -> Option<StdResult<Self>> {
        AppendStoreMut::attach_with_serialization(storage, Bincode2)
    }

    pub fn prefixed<S: Storage>(namespace: &[u8], storage: &'a mut S) -> StdResult<Self> {
        let storage = Box::new(PrefixedStorage::new(namespace, storage));
        Self::attach_or_create_with_serialization_impl(storage, Bincode2)
    }
}

impl<'a, T, Ser> AppendStoreMut<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// Try to use the provided storage as an AppendStore. If it doesn't seem to be one, then
    /// initialize it as one. This method allows choosing the serialization format you want to use.
    ///
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_or_create_with_serialization(
        storage: &'a mut impl Storage,
        _ser: Ser,
    ) -> StdResult<Self> {
        let storage = Box::new(storage);
        Self::attach_or_create_with_serialization_impl(storage, _ser)
    }

    pub fn attach_or_create_with_serialization_impl(
        mut storage: Box<dyn Storage + 'a>,
        _ser: Ser,
    ) -> StdResult<Self> {
        if let Some(len_vec) = storage.get(LEN_KEY) {
            Self::new(storage, &len_vec)
        } else {
            let len_vec = 0_u32.to_be_bytes();
            storage.set(LEN_KEY, &len_vec);
            Self::new(storage, &len_vec)
        }
    }

    /// Try to use the provided storage as an AppendStore.
    /// This method allows choosing the serialization format you want to use.
    ///
    /// Returns None if the provided storage doesn't seem like an AppendStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_with_serialization(
        storage: &'a mut impl Storage,
        _ser: Ser,
    ) -> Option<StdResult<Self>> {
        let storage = Box::new(storage);
        Self::attach_with_serialization_impl(storage, _ser)
    }

    pub fn attach_with_serialization_impl(
        storage: Box<dyn Storage + 'a>,
        _ser: Ser,
    ) -> Option<StdResult<Self>> {
        let len_vec = storage.get(LEN_KEY)?;
        Some(Self::new(storage, &len_vec))
    }

    fn new(storage: Box<dyn Storage + 'a>, len_vec: &[u8]) -> StdResult<Self> {
        let len_array = len_vec
            .try_into()
            .map_err(|err| StdError::parse_err("u32", err))?;
        let len = u32::from_be_bytes(len_array);

        Ok(Self {
            storage,
            item_type: PhantomData,
            serialization_type: PhantomData,
            len,
        })
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn storage(&mut self) -> &mut dyn Storage {
        self.storage.as_mut()
    }

    pub fn readonly_storage(&self) -> &dyn Storage {
        self.storage.as_ref()
    }

    /// Return an iterator over the items in the collection
    pub fn iter(&self) -> Iter<T, Ser> {
        self.as_readonly().iter()
    }

    /// Get the value stored at a given position.
    ///
    /// # Errors
    /// Will return an error if pos is out of bounds or if an item is not found.
    pub fn get_at(&self, pos: u32) -> StdResult<T> {
        self.as_readonly().get_at(pos)
    }

    fn get_at_unchecked(&self, pos: u32) -> StdResult<T> {
        self.as_readonly().get_at_unchecked(pos)
    }

    /// Set the value of the item stored at a given position.
    ///
    /// # Errors
    /// Will return an error if the position is out of bounds
    pub fn set_at(&mut self, pos: u32, item: &T) -> StdResult<()> {
        if pos >= self.len {
            return Err(StdError::generic_err("AppendStorage access out of bounds"));
        }
        self.set_at_unchecked(pos, item)
    }

    fn set_at_unchecked(&mut self, pos: u32, item: &T) -> StdResult<()> {
        let serialized = Ser::serialize(item)?;
        self.storage.set(&pos.to_be_bytes(), &serialized);
        Ok(())
    }

    /// Append an item to the end of the collection.
    ///
    /// This operation has a constant cost.
    pub fn push(&mut self, item: &T) -> StdResult<()> {
        self.set_at_unchecked(self.len, item)?;
        self.set_length(self.len + 1);
        Ok(())
    }

    /// Pop the last item off the collection
    pub fn pop(&mut self) -> StdResult<T> {
        if let Some(len) = self.len.checked_sub(1) {
            let item = self.get_at_unchecked(len);
            self.set_length(len);
            item
        } else {
            Err(StdError::generic_err("Can not pop from empty AppendStore"))
        }
    }

    /// Clear the collection
    pub fn clear(&mut self) {
        self.set_length(0);
    }

    /// Set the length of the collection
    fn set_length(&mut self, len: u32) {
        self.storage.set(LEN_KEY, &len.to_be_bytes());
        self.len = len;
    }

    /// Gain access to the implementation of the immutable methods
    fn as_readonly(&self) -> AppendStore<T, Ser> {
        AppendStore {
            storage: Box::new(&self.storage),
            item_type: self.item_type,
            serialization_type: self.serialization_type,
            len: self.len,
        }
    }
}

// Doing this is fundamentally flawed because it would theoretically permanently turn the `&mut S`
// into a `&S`, preventing any further mutation of the entire storage.
// In practice this just gave annoying lifetime errors either here or at `AppendStoreMut::as_readonly`.
/*
impl<'a, T, S> IntoIterator for AppendStoreMut<'a, T, S>
where
    T: 'a + Serialize + DeserializeOwned,
    S: Storage,
{
    type Item = StdResult<T>;
    type IntoIter = Iter<'a, T, S>;

    fn into_iter(self) -> Iter<'a, T, S> {
        Iter {
            storage: self.as_readonly(),
            start: 0,
            end: self.len,
        }
    }
}
*/

// Readonly append-store

/// A type allowing only reads from an append store. useful in the context_, u8 of queries.
// #[derive(Debug)]
pub struct AppendStore<'a, T, Ser = Bincode2>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    storage: Box<dyn ReadonlyStorage + 'a>,
    item_type: PhantomData<*const T>,
    serialization_type: PhantomData<*const Ser>,
    len: u32,
}

impl<'a, T> AppendStore<'a, T, Bincode2>
where
    T: Serialize + DeserializeOwned,
{
    /// Try to use the provided storage as an AppendStore.
    ///
    /// Returns None if the provided storage doesn't seem like an AppendStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach(storage: &'a impl ReadonlyStorage) -> Option<StdResult<Self>> {
        AppendStore::attach_with_serialization(storage, Bincode2)
    }
}

impl<'a, T, Ser> AppendStore<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// Try to use the provided storage as an AppendStore.
    /// This method allows choosing the serialization format you want to use.
    ///
    /// Returns None if the provided storage doesn't seem like an AppendStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_with_serialization(
        storage: &'a impl ReadonlyStorage,
        _ser: Ser,
    ) -> Option<StdResult<Self>> {
        let storage = Box::new(storage);
        Self::attach_with_serialization_impl(storage, _ser)
    }

    pub fn attach_with_serialization_impl(
        storage: Box<dyn ReadonlyStorage + 'a>,
        _ser: Ser,
    ) -> Option<StdResult<Self>> {
        let len_vec = storage.get(LEN_KEY)?;
        Some(AppendStore::new(storage, len_vec))
    }

    fn new(storage: Box<dyn ReadonlyStorage + 'a>, len_vec: Vec<u8>) -> StdResult<Self> {
        let len_array = len_vec
            .as_slice()
            .try_into()
            .map_err(|err| StdError::parse_err("u32", err))?;
        let len = u32::from_be_bytes(len_array);

        Ok(Self {
            storage,
            item_type: PhantomData,
            serialization_type: PhantomData,
            len,
        })
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn readonly_storage(&self) -> &dyn ReadonlyStorage {
        self.storage.as_ref()
    }

    /// Return an iterator over the items in the collection
    pub fn iter(&self) -> Iter<'a, T, Ser> {
        Iter {
            storage: AppendStore::clone(self),
            start: 0,
            end: self.len,
        }
    }

    /// Get the value stored at a given position.
    ///
    /// # Errors
    /// Will return an error if pos is out of bounds or if an item is not found.
    pub fn get_at(&self, pos: u32) -> StdResult<T> {
        if pos >= self.len {
            return Err(StdError::generic_err("AppendStorage access out of bounds"));
        }
        self.get_at_unchecked(pos)
    }

    fn get_at_unchecked(&self, pos: u32) -> StdResult<T> {
        let serialized = self.storage.get(&pos.to_be_bytes()).ok_or_else(|| {
            StdError::generic_err(format!("No item in AppendStorage at position {}", pos))
        })?;
        Ser::deserialize(&serialized)
    }
}

impl<'a, T, Ser> IntoIterator for AppendStore<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    type Item = StdResult<T>;
    type IntoIter = Iter<'a, T, Ser>;

    fn into_iter(self) -> Iter<'a, T, Ser> {
        let end = self.len;
        Iter {
            storage: self,
            start: 0,
            end,
        }
    }
}

// // Manual `Clone` implementation because the default one tries to clone the Storage??
// impl<'a, T, Ser> Clone for AppendStore<'a, T, Ser>
// where
//     T: Serialize + DeserializeOwned,
//     Ser: Serde,
// {
//     fn clone(&self) -> Self {
//         Self {
//             storage: self.storage.clone(),
//             item_type: self.item_type,
//             serialization_type: self.serialization_type,
//             len: self.len,
//         }
//     }
// }

// Owning iterator

/// An iterator over the contents of the append store.
// #[derive(Debug)]
pub struct Iter<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    storage: AppendStore<'a, T, Ser>,
    start: u32,
    end: u32,
}

impl<'a, T, Ser> Iterator for Iter<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    type Item = StdResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        let item = self.storage.get_at(self.start);
        self.start += 1;
        Some(item)
    }

    // This needs to be implemented correctly for `ExactSizeIterator` to work.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.end - self.start) as usize;
        (len, Some(len))
    }

    // I implement `nth` manually because it is used in the standard library whenever
    // it wants to skip over elements, but the default implementation repeatedly calls next.
    // because that is very expensive in this case, and the items are just discarded, we wan
    // do better here.
    // In practice, this enables cheap paging over the storage by calling:
    // `append_store.iter().skip(start).take(length).collect()`
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.start = self.start.saturating_add(n as u32);
        self.next()
    }
}

impl<'a, T, Ser> DoubleEndedIterator for Iter<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        self.end -= 1;
        let item = self.storage.get_at(self.end);
        Some(item)
    }

    // I implement `nth_back` manually because it is used in the standard library whenever
    // it wants to skip over elements, but the default implementation repeatedly calls next_back.
    // because that is very expensive in this case, and the items are just discarded, we wan
    // do better here.
    // In practice, this enables cheap paging over the storage by calling:
    // `append_store.iter().skip(start).take(length).collect()`
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.end = self.end.saturating_sub(n as u32);
        self.next_back()
    }
}

// This enables writing `append_store.iter().skip(n).rev()`
impl<'a, T, Ser> ExactSizeIterator for Iter<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::MockStorage;

    use secret_toolkit_serialization::Json;

    use super::*;

    #[test]
    fn test_push_pop() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let mut append_store = AppendStoreMut::attach_or_create(&mut storage)?;
        append_store.push(&1234)?;
        append_store.push(&2143)?;
        append_store.push(&3412)?;
        append_store.push(&4321)?;

        assert_eq!(append_store.pop(), Ok(4321));
        assert_eq!(append_store.pop(), Ok(3412));
        assert_eq!(append_store.pop(), Ok(2143));
        assert_eq!(append_store.pop(), Ok(1234));
        assert!(append_store.pop().is_err());

        Ok(())
    }

    #[test]
    fn test_iterator() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let mut append_store = AppendStoreMut::attach_or_create(&mut storage)?;
        append_store.push(&1234)?;
        append_store.push(&2143)?;
        append_store.push(&3412)?;
        append_store.push(&4321)?;

        // iterate twice to make sure nothing changed
        let mut iter = append_store.iter();
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        let mut iter = append_store.iter();
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth` doesn't break anything
        let mut iter = append_store.iter().skip(2);
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_reverse_iterator() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let mut append_store = AppendStoreMut::attach_or_create(&mut storage)?;
        append_store.push(&1234)?;
        append_store.push(&2143)?;
        append_store.push(&3412)?;
        append_store.push(&4321)?;

        let mut iter = append_store.iter().rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // iterate twice to make sure nothing changed
        let mut iter = append_store.iter().rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth_back` doesn't break anything
        let mut iter = append_store.iter().rev().skip(2);
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `ExactSizeIterator` works well
        let mut iter = append_store.iter().skip(2).rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_attach_to_wrong_location() {
        let mut storage = MockStorage::new();
        assert!(AppendStore::<u8, _>::attach(&storage).is_none());
        assert!(AppendStoreMut::<u8, _>::attach(&mut storage).is_none());
    }

    #[test]
    fn test_serializations() -> StdResult<()> {
        // Check the default behavior is Bincode2
        let mut storage = MockStorage::new();

        let mut append_store = AppendStoreMut::attach_or_create(&mut storage)?;
        append_store.push(&1234)?;

        let bytes = append_store.readonly_storage().get(&0_u32.to_be_bytes());
        assert_eq!(bytes, Some(vec![210, 4, 0, 0]));

        // Check that overriding the serializer with Json works
        let mut storage = MockStorage::new();
        let mut append_store =
            AppendStoreMut::attach_or_create_with_serialization(&mut storage, Json)?;
        append_store.push(&1234)?;
        let bytes = append_store.readonly_storage().get(&0_u32.to_be_bytes());
        assert_eq!(bytes, Some(b"1234".to_vec()));

        Ok(())
    }
}
