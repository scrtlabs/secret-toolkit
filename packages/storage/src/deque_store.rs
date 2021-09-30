//! This is a storage wrapper based on AppendStore called DequeStore.
//! It guarantees constant-cost appending to and popping from a list of items in storage on both directions (front and back).
//!
//! This is achieved by storing each item in a separate storage entry.
//! A special key is reserved for storing the length of the collection so far.
//! Another special key is reserved for storing the offset of the collection.
use std::{convert::TryInto, marker::PhantomData};

use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{ReadonlyStorage, StdError, StdResult, Storage};

use secret_toolkit_serialization::{Bincode2, Serde};

const LEN_KEY: &[u8] = b"len";
const OFFSET_KEY: &[u8] = b"off";
// Mutable deque_store

/// A type allowing both reads from and writes to the deque store at a given storage location.
#[derive(Debug)]
pub struct DequeStoreMut<'a, T, S, Ser = Bincode2>
where
    T: Serialize + DeserializeOwned,
    S: Storage,
    Ser: Serde,
{
    storage: &'a mut S,
    item_type: PhantomData<*const T>,
    serialization_type: PhantomData<*const Ser>,
    len: u32,
    off: u32,
}

impl<'a, T, S> DequeStoreMut<'a, T, S, Bincode2>
where
    T: Serialize + DeserializeOwned,
    S: Storage,
{
    /// Try to use the provided storage as an DequeStore. If it doesn't seem to be one, then
    /// initialize it as one.
    ///
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_or_create(storage: &'a mut S) -> StdResult<Self> {
        DequeStoreMut::attach_or_create_with_serialization(storage, Bincode2)
    }

    /// Try to use the provided storage as an DequeStore.
    ///
    /// Returns None if the provided storage doesn't seem like an DequeStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach(storage: &'a mut S) -> Option<StdResult<Self>> {
        DequeStoreMut::attach_with_serialization(storage, Bincode2)
    }
}

impl<'a, T, S, Ser> DequeStoreMut<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned,
    S: Storage,
    Ser: Serde,
{
    /// Try to use the provided storage as an DequeStore. If it doesn't seem to be one, then
    /// initialize it as one. This method allows choosing the serialization format you want to use.
    ///
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_or_create_with_serialization(storage: &'a mut S, _ser: Ser) -> StdResult<Self> {
        if let (Some(len_vec), Some(off_vec)) = (storage.get(LEN_KEY), (storage.get(OFFSET_KEY))) {
            Self::new(storage, &len_vec, &off_vec)
        } else {
            let len_vec = 0_u32.to_be_bytes();
            storage.set(LEN_KEY, &len_vec);
            let off_vec = 0_u32.to_be_bytes();
            storage.set(OFFSET_KEY, &off_vec);
            Self::new(storage, &len_vec, &off_vec)
        }
    }

    /// Try to use the provided storage as an DequeStore.
    /// This method allows choosing the serialization format you want to use.
    ///
    /// Returns None if the provided storage doesn't seem like an DequeStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_with_serialization(storage: &'a mut S, _ser: Ser) -> Option<StdResult<Self>> {
        let len_vec = storage.get(LEN_KEY)?;
        let off_vec = storage.get(OFFSET_KEY)?;
        Some(Self::new(storage, &len_vec, &off_vec))
    }

    fn new(storage: &'a mut S, len_vec: &[u8], off_vec: &[u8]) -> StdResult<Self> {
        let len_array = len_vec
            .try_into()
            .map_err(|err| StdError::parse_err("u32", err))?;
        let len = u32::from_be_bytes(len_array);
        let off_array = off_vec
            .try_into()
            .map_err(|err| StdError::parse_err("u32", err))?;
        let off = u32::from_be_bytes(off_array);

        Ok(Self {
            storage,
            item_type: PhantomData,
            serialization_type: PhantomData,
            len,
            off,
        })
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn storage(&mut self) -> &mut S {
        self.storage
    }

    pub fn readonly_storage(&self) -> &S {
        self.storage
    }

    /// Return an iterator over the items in the collection
    pub fn iter(&self) -> Iter<T, S, Ser> {
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
            return Err(StdError::generic_err("DequeStorage access out of bounds"));
        }
        self.set_at_unchecked(pos, item)
    }

    fn set_at_unchecked(&mut self, pos: u32, item: &T) -> StdResult<()> {
        let serialized = Ser::serialize(item)?;
        self.storage.set(
            &(pos.overflowing_add(self.off).0).to_be_bytes(),
            &serialized,
        );
        Ok(())
    }

    /// Append an item to the end of the collection.
    ///
    /// This operation has a constant cost.
    pub fn push_back(&mut self, item: &T) -> StdResult<()> {
        self.set_at_unchecked(self.len, item)?;
        self.set_length(self.len + 1);
        Ok(())
    }

    /// Add an item to the begining of the collection.
    ///
    /// This operation has a constant cost.
    pub fn push_front(&mut self, item: &T) -> StdResult<()> {
        self.set_offset(self.off.overflowing_sub(1).0);
        self.set_at_unchecked(0, item)?;
        self.set_length(self.len + 1);
        Ok(())
    }

    /// Pop the last item off the collection
    ///
    /// This operation has a constant cost.
    pub fn pop_back(&mut self) -> StdResult<T> {
        if let Some(len) = self.len.checked_sub(1) {
            let item = self.get_at_unchecked(len);
            self.set_length(len);
            item
        } else {
            Err(StdError::generic_err("Can not pop from empty DequeStore"))
        }
    }

    /// Pop the first item off the collection
    ///
    /// This operation has a constant cost.
    pub fn pop_front(&mut self) -> StdResult<T> {
        if let Some(len) = self.len.checked_sub(1) {
            let item = self.get_at_unchecked(0);
            self.set_length(len);
            self.set_offset(self.off.overflowing_add(1).0);
            item
        } else {
            Err(StdError::generic_err("Can not pop from empty DequeStore"))
        }
    }

    /// Remove an element off the collection at the position provided
    ///
    /// Removing an element from the head (first) or tail (last) has a constant cost.
    /// Removing from the middle the cost will depend on the proximity to the head or tail.
    /// In this case, all the elements between the closest tip of the collection (head or tail)
    /// and the remove position will be shifting positions.
    ///
    /// Worst case scenario, in terms of cost, will be if the element position is
    /// exactly in the middle of the collection.
    pub fn remove(&mut self, pos: u32) -> StdResult<T> {
        if pos >= self.len {
            return Err(StdError::generic_err("DequeStorage access out of bounds"));
        }
        let item = self.get_at_unchecked(pos);
        let to_tail = self.len - pos;
        if to_tail < pos {
            // closer to the tail
            for i in pos..self.len - 1 {
                let element_to_shift = self.get_at_unchecked(i + 1)?;
                self.set_at_unchecked(i, &element_to_shift)?;
            }
        } else {
            // closer to the head
            for i in (0..pos).rev() {
                let element_to_shift = self.get_at_unchecked(i)?;
                self.set_at_unchecked(i + 1, &element_to_shift)?;
            }
            self.set_offset(self.off.overflowing_add(1).0);
        }
        self.set_length(self.len - 1);
        item
    }

    /// Set the length of the collection
    fn set_length(&mut self, len: u32) {
        self.storage.set(LEN_KEY, &len.to_be_bytes());
        self.len = len;
    }

    /// Set the offset of the collection
    fn set_offset(&mut self, off: u32) {
        self.storage.set(OFFSET_KEY, &off.to_be_bytes());
        self.off = off;
    }

    /// Gain access to the implementation of the immutable methods
    fn as_readonly(&self) -> DequeStore<T, S, Ser> {
        DequeStore {
            storage: self.storage,
            item_type: self.item_type,
            serialization_type: self.serialization_type,
            len: self.len,
            off: self.off,
        }
    }
}

// Readonly deque-store

/// A type allowing only reads from an deque store. useful in the context_, u8 of queries.
#[derive(Debug)]
pub struct DequeStore<'a, T, S, Ser = Bincode2>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{
    storage: &'a S,
    item_type: PhantomData<*const T>,
    serialization_type: PhantomData<*const Ser>,
    len: u32,
    off: u32,
}

impl<'a, T, S> DequeStore<'a, T, S, Bincode2>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
{
    /// Try to use the provided storage as an DequeStore.
    ///
    /// Returns None if the provided storage doesn't seem like an DequeStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach(storage: &'a S) -> Option<StdResult<Self>> {
        DequeStore::attach_with_serialization(storage, Bincode2)
    }
}

impl<'a, T, S, Ser> DequeStore<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{
    /// Try to use the provided storage as an DequeStore.
    /// This method allows choosing the serialization format you want to use.
    ///
    /// Returns None if the provided storage doesn't seem like an DequeStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_with_serialization(storage: &'a S, _ser: Ser) -> Option<StdResult<Self>> {
        let len_vec = storage.get(LEN_KEY)?;
        let off_vec = storage.get(OFFSET_KEY)?;
        Some(DequeStore::new(storage, len_vec, off_vec))
    }

    fn new(storage: &'a S, len_vec: Vec<u8>, off_vec: Vec<u8>) -> StdResult<Self> {
        let len_array = len_vec
            .as_slice()
            .try_into()
            .map_err(|err| StdError::parse_err("u32", err))?;
        let len = u32::from_be_bytes(len_array);
        let off_array = off_vec
            .as_slice()
            .try_into()
            .map_err(|err| StdError::parse_err("u32", err))?;
        let off = u32::from_be_bytes(off_array);

        Ok(Self {
            storage,
            item_type: PhantomData,
            serialization_type: PhantomData,
            len,
            off,
        })
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn readonly_storage(&self) -> &S {
        self.storage
    }

    /// Return an iterator over the items in the collection
    pub fn iter(&self) -> Iter<'a, T, S, Ser> {
        Iter {
            storage: DequeStore::clone(self),
            start: 0_u32,
            end: self.len,
        }
    }

    /// Get the value stored at a given position.
    ///
    /// # Errors
    /// Will return an error if pos is out of bounds or if an item is not found.
    pub fn get_at(&self, pos: u32) -> StdResult<T> {
        if pos >= self.len {
            return Err(StdError::generic_err("DequeStorage access out of bounds"));
        }
        self.get_at_unchecked(pos)
    }

    fn get_at_unchecked(&self, pos: u32) -> StdResult<T> {
        let serialized = self
            .storage
            .get(&(pos.overflowing_add(self.off).0).to_be_bytes())
            .ok_or_else(|| {
                StdError::generic_err(format!(
                    "No item in DequeStorage at position {}",
                    pos.overflowing_add(self.off).0
                ))
            })?;
        Ser::deserialize(&serialized)
    }
}

impl<'a, T, S, Ser> IntoIterator for DequeStore<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{
    type Item = StdResult<T>;
    type IntoIter = Iter<'a, T, S, Ser>;

    fn into_iter(self) -> Iter<'a, T, S, Ser> {
        let end = self.len;
        Iter {
            storage: self,
            start: 0_u32,
            end,
        }
    }
}

// Manual `Clone` implementation because the default one tries to clone the Storage??
impl<'a, T, S, Ser> Clone for DequeStore<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{
    fn clone(&self) -> Self {
        Self {
            storage: &self.storage,
            item_type: self.item_type,
            serialization_type: self.serialization_type,
            len: self.len,
            off: self.off,
        }
    }
}

// Owning iterator

/// An iterator over the contents of the deque store.
#[derive(Debug)]
pub struct Iter<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{
    storage: DequeStore<'a, T, S, Ser>,
    start: u32,
    end: u32,
}

impl<'a, T, S, Ser> Iterator for Iter<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
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
    // `deque_store.iter().skip(start).take(length).collect()`
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.start = self.start.saturating_add(n as u32);
        self.next()
    }
}

impl<'a, T, S, Ser> DoubleEndedIterator for Iter<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
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
    // `deque_store.iter().skip(start).take(length).collect()`
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.end = self.end.saturating_sub(n as u32);
        self.next_back()
    }
}

// This enables writing `deque_store.iter().skip(n).rev()`
impl<'a, T, S, Ser> ExactSizeIterator for Iter<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::MockStorage;

    use secret_toolkit_serialization::Json;

    use super::*;

    #[test]
    fn test_pushs_pops() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let mut deque_store = DequeStoreMut::attach_or_create(&mut storage)?;
        deque_store.push_front(&4)?;
        deque_store.push_back(&5)?;
        deque_store.push_front(&3)?;
        deque_store.push_back(&6)?;
        deque_store.push_front(&2)?;
        deque_store.push_back(&7)?;
        deque_store.push_front(&1)?;
        deque_store.push_back(&8)?;

        assert_eq!(deque_store.pop_front(), Ok(1));
        assert_eq!(deque_store.pop_back(), Ok(8));
        assert_eq!(deque_store.pop_front(), Ok(2));
        assert_eq!(deque_store.pop_back(), Ok(7));
        assert_eq!(deque_store.pop_front(), Ok(3));
        assert_eq!(deque_store.pop_back(), Ok(6));
        assert_eq!(deque_store.pop_front(), Ok(4));
        assert_eq!(deque_store.pop_back(), Ok(5));
        assert!(deque_store.pop_back().is_err());
        Ok(())
    }

    #[test]
    fn test_removes() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let mut deque_store = DequeStoreMut::attach_or_create(&mut storage)?;
        deque_store.push_front(&2)?;
        deque_store.push_back(&3)?;
        deque_store.push_back(&4)?;
        deque_store.push_back(&5)?;
        deque_store.push_back(&6)?;
        deque_store.push_front(&1)?;
        deque_store.push_back(&7)?;
        deque_store.push_back(&8)?;

        assert!(deque_store.remove(8).is_err());
        assert!(deque_store.remove(9).is_err());

        assert_eq!(deque_store.remove(7), Ok(8));
        assert_eq!(deque_store.get_at(6), Ok(7));
        assert_eq!(deque_store.get_at(5), Ok(6));
        assert_eq!(deque_store.get_at(4), Ok(5));
        assert_eq!(deque_store.get_at(3), Ok(4));
        assert_eq!(deque_store.get_at(2), Ok(3));
        assert_eq!(deque_store.get_at(1), Ok(2));
        assert_eq!(deque_store.get_at(0), Ok(1));

        assert_eq!(deque_store.remove(6), Ok(7));
        assert_eq!(deque_store.get_at(5), Ok(6));
        assert_eq!(deque_store.get_at(4), Ok(5));
        assert_eq!(deque_store.get_at(3), Ok(4));
        assert_eq!(deque_store.get_at(2), Ok(3));
        assert_eq!(deque_store.get_at(1), Ok(2));
        assert_eq!(deque_store.get_at(0), Ok(1));

        assert_eq!(deque_store.remove(3), Ok(4));
        assert_eq!(deque_store.get_at(4), Ok(6));
        assert_eq!(deque_store.get_at(3), Ok(5));
        assert_eq!(deque_store.get_at(2), Ok(3));
        assert_eq!(deque_store.get_at(1), Ok(2));
        assert_eq!(deque_store.get_at(0), Ok(1));

        assert_eq!(deque_store.remove(1), Ok(2));
        assert_eq!(deque_store.get_at(3), Ok(6));
        assert_eq!(deque_store.get_at(2), Ok(5));
        assert_eq!(deque_store.get_at(1), Ok(3));
        assert_eq!(deque_store.get_at(0), Ok(1));

        assert_eq!(deque_store.remove(2), Ok(5));
        assert_eq!(deque_store.get_at(2), Ok(6));
        assert_eq!(deque_store.get_at(1), Ok(3));
        assert_eq!(deque_store.get_at(0), Ok(1));

        assert_eq!(deque_store.remove(1), Ok(3));
        assert_eq!(deque_store.get_at(1), Ok(6));
        assert_eq!(deque_store.get_at(0), Ok(1));

        assert_eq!(deque_store.remove(1), Ok(6));
        assert_eq!(deque_store.get_at(0), Ok(1));

        assert_eq!(deque_store.remove(0), Ok(1));

        assert!(deque_store.remove(0).is_err());
        Ok(())
    }

    #[test]
    fn test_iterator() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let mut deque_store = DequeStoreMut::attach_or_create(&mut storage)?;

        deque_store.push_front(&2143)?;
        deque_store.push_back(&3333)?;
        deque_store.push_back(&3412)?;
        deque_store.push_front(&1234)?;
        deque_store.push_back(&4321)?;

        deque_store.remove(2)?;

        // iterate twice to make sure nothing changed
        let mut iter = deque_store.iter();
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        let mut iter = deque_store.iter();
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth` doesn't break anything
        let mut iter = deque_store.iter().skip(2);
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_reverse_iterator() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let mut deque_store = DequeStoreMut::attach_or_create(&mut storage)?;
        deque_store.push_front(&2143)?;
        deque_store.push_back(&3412)?;
        deque_store.push_back(&3333)?;
        deque_store.push_front(&1234)?;
        deque_store.push_back(&4321)?;

        deque_store.remove(3)?;

        let mut iter = deque_store.iter().rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // iterate twice to make sure nothing changed
        let mut iter = deque_store.iter().rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth_back` doesn't break anything
        let mut iter = deque_store.iter().rev().skip(2);
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `ExactSizeIterator` works well
        let mut iter = deque_store.iter().skip(2).rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_attach_to_wrong_location() {
        let mut storage = MockStorage::new();
        assert!(DequeStore::<u8, _>::attach(&storage).is_none());
        assert!(DequeStoreMut::<u8, _>::attach(&mut storage).is_none());
    }

    #[test]
    fn test_serializations() -> StdResult<()> {
        // Check the default behavior is Bincode2
        let mut storage = MockStorage::new();

        let mut deque_store = DequeStoreMut::attach_or_create(&mut storage)?;
        deque_store.push_back(&1234)?;

        let bytes = deque_store.readonly_storage().get(&0_u32.to_be_bytes());
        assert_eq!(bytes, Some(vec![210, 4, 0, 0]));

        // Check that overriding the serializer with Json works
        let mut storage = MockStorage::new();
        let mut deque_store =
            DequeStoreMut::attach_or_create_with_serialization(&mut storage, Json)?;
        deque_store.push_back(&1234)?;
        let bytes = deque_store.readonly_storage().get(&0_u32.to_be_bytes());
        assert_eq!(bytes, Some(b"1234".to_vec()));

        Ok(())
    }
}
