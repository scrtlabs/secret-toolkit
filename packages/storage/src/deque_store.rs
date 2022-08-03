//! This is a storage wrapper based on AppendStore called DequeStore.
//! It guarantees constant-cost appending to and popping from a list of items in storage on both directions (front and back).
//!
//! This is achieved by storing each item in a separate storage entry.
//! A special key is reserved for storing the length of the collection so far.
//! Another special key is reserved for storing the offset of the collection.
use std::{convert::TryInto};
use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{ReadonlyStorage, StdError, StdResult, Storage};

use secret_toolkit_serialization::{Serde, Bincode2};

use crate::prefixed_typed_storage::PrefixedTypedStorage;

const LEN_KEY: &[u8] = b"len";
const OFFSET_KEY: &[u8] = b"off";

pub struct DequeStore<'a, T, Ser = Bincode2>
    where
        T: Serialize + DeserializeOwned,
        Ser: Serde,
{
    /// prefix of the newly constructed Storage
    namespace: &'a [u8],
    /// needed if any suffixes were added to the original namespace.
    /// therefore it is not necessarily same as the namespace.
    prefix: Option<Vec<u8>>,
    item_type: PhantomData<T>,
    serialization_type: PhantomData<Ser>,
}

impl<'a, 'b, T: Serialize + DeserializeOwned, Ser: Serde> DequeStore<'a, T, Ser>{
    /// constructor
    pub const fn new(prefix: &'a [u8]) -> Self {
        Self {
            namespace: prefix,
            prefix: None,
            item_type: PhantomData,
            serialization_type: PhantomData,
        }
    }
    /// This is used to produce a new DequeStorage. This can be used when you want to associate an AppendListStorage to each user
    /// and you still get to define the DequeStorage as a static constant
    pub fn add_suffix(&self, suffix: &[u8]) -> Self {
        let prefix = if let Some(prefix) = self.prefix.clone() {
            [prefix, suffix.to_vec()].concat()
        } else {
            [self.namespace.to_vec(), suffix.to_vec()].concat()
        };
        Self {
            namespace: self.namespace,
            prefix: Some(prefix),
            item_type: self.item_type,
            serialization_type: self.serialization_type,
        }
    }
}

impl<'a, T: Serialize + DeserializeOwned, Ser: Serde> DequeStore<'a, T, Ser> {
    /// gets the length from storage, and otherwise sets it to 0
    pub fn get_len<S: ReadonlyStorage>(&self, storage: &S) -> StdResult<u32> {
        self._get_u32(storage, LEN_KEY)
    }
    /// gets the offset from storage, and otherwise sets it to 0
    pub fn get_off<S: ReadonlyStorage>(&self, storage: &S) -> StdResult<u32> {
        self._get_u32(storage, OFFSET_KEY)
    }
    /// gets offset or length
    fn _get_u32<S: ReadonlyStorage>(&self, storage: &S, key: &[u8]) -> StdResult<u32> {
        let num_key = [self.as_slice(), key].concat();
        if let Some(num_vec) = storage.get(&num_key) {
            let num_bytes = num_vec.as_slice().try_into().map_err(|err| StdError::parse_err("u32", err))?;
            let num = u32::from_be_bytes(num_bytes);
            Ok(num)
        } else {
            Ok(0)
        }
    }
    /// checks if the collection has any elements
    pub fn is_empty<S: ReadonlyStorage>(&self, storage: &S) -> StdResult<bool> {
        Ok(self.get_len(storage)? == 0)
    }
    /// gets the element at pos if within bounds
    pub fn get_at<S: ReadonlyStorage>(&self, storage: &S, pos: u32) -> StdResult<T> {
        let len = self.get_len(storage)?;
        if pos >= len { return Err(StdError::generic_err("DequeStore access out of bounds")); }
        self.get_at_unchecked(storage, pos)
    }
    /// tries to get the element at pos
    fn get_at_unchecked<S: ReadonlyStorage>(&self, storage: &S, pos: u32) -> StdResult<T> {
        self.load_impl(storage, &self._get_offset_pos(storage, pos)?.to_be_bytes())
    }
    /// add the offset to the pos
    fn _get_offset_pos<S: ReadonlyStorage>(&self, storage: &S, pos: u32) -> StdResult<u32> {
        let off = self.get_off(storage)?;
        Ok(pos.overflowing_add(off).0)
    }
    /// Set the length of the collection
    fn set_len<S: Storage>(&self, storage: &mut S, len: u32) {
        self._set_u32(storage, LEN_KEY, len)
    }
    /// Set the offset of the collection
    fn set_off<S: Storage>(&self, storage: &mut S, off: u32) {
        self._set_u32(storage, OFFSET_KEY, off)
    }
    /// Set the length or offset of the collection
    fn _set_u32<S: Storage>(&self, storage: &mut S, key: &[u8], num: u32) {
        let num_key = [self.as_slice(), key].concat();
        storage.set(&num_key, &num.to_be_bytes());
    }
    /// Clear the collection
    pub fn clear<S: Storage>(&self, storage: &mut S) {
        self.set_len(storage, 0);
        self.set_off(storage, 0);
    }
    /// Replaces data at a position within bounds
    pub fn set_at<S: Storage>(&self, storage: &mut S, pos: u32, item: &T) -> StdResult<()> {
        let len = self.get_len(storage)?;
        if pos >= len { return Err(StdError::generic_err("DequeStore access out of bounds")); }
        self.set_at_unchecked(storage, pos, item)
    }
    /// Sets data at a given index
    fn set_at_unchecked<S: Storage>(&self, storage: &mut S, pos: u32, item: &T) -> StdResult<()> {
        self.save_impl(storage, &self._get_offset_pos(storage, pos)?.to_be_bytes(), item)
    }
    /// Pushes an item to the back
    pub fn push_back<S: Storage>(&self, storage: &mut S, item: &T) -> StdResult<()> {
        let len = self.get_len(storage)?;
        self.set_at_unchecked(storage, len, item)?;
        self.set_len(storage, len + 1);
        Ok(())
    }
    /// Pushes an item to the front
    pub fn push_front<S: Storage>(&self, storage: &mut S, item: &T) -> StdResult<()> {
        let off = self.get_off(storage)?;
        let len = self.get_len(storage)?;
        self.set_off(storage, off.overflowing_sub(1).0);
        self.set_at_unchecked(storage, 0, item)?;
        self.set_len(storage, len + 1);
        Ok(())
    }
    /// Pops an item from the back
    pub fn pop_back<S: Storage>(&self, storage: &mut S) -> StdResult<T> {
        if let Some(len) = self.get_len(storage)?.checked_sub(1) { 
            let item = self.get_at_unchecked(storage, len);
            self.set_len(storage, len);
            item
        } else {
            Err(StdError::generic_err("Can not pop from empty DequeStore"))
        }
    }
    /// Pops an item from the front
    pub fn pop_front<S: Storage>(&self, storage: &mut S) -> StdResult<T> {
        if let Some(len) = self.get_len(storage)?.checked_sub(1) {
            let off = self.get_off(storage)?;
            let item = self.get_at_unchecked(storage, 0);
            self.set_len(storage, len);
            self.set_off(storage, off.overflowing_add(1).0);
            item
        } else {
            Err(StdError::generic_err("Can not pop from empty DequeStore"))
        }
    }
    /// Remove an element from the collection at the specified position.
    ///
    /// Removing an element from the head (first) or tail (last) has a constant cost.
    /// The cost of removing from the middle will depend on the proximity to the head or tail.
    /// In that case, all the elements between the closest tip of the collection (head or tail)
    /// and the specified position will be shifted in storage.
    ///
    /// Removing an element from the middle of the collection
    /// has the worst runtime and gas cost.
    pub fn remove<S: Storage>(&self, storage: &mut S, pos: u32) -> StdResult<T> {
        let off = self.get_off(storage)?;
        let len = self.get_len(storage)?;
        if pos >= len {
            return Err(StdError::generic_err("DequeStorage access out of bounds"));
        }
        let item = self.get_at_unchecked(storage, pos);
        let to_tail = len - pos;
        if to_tail < pos {
            // closer to the tail
            for i in pos..(len - 1) {
                let element_to_shift = self.get_at_unchecked(storage, i + 1)?;
                self.set_at_unchecked(storage, i, &element_to_shift)?;
            }
        } else {
            // closer to the head
            for i in (0..pos).rev() {
                let element_to_shift = self.get_at_unchecked(storage, i)?;
                self.set_at_unchecked(storage, i + 1, &element_to_shift)?;
            }
            self.set_off(storage, off.overflowing_add(1).0);
        }
        self.set_len(storage, len - 1);
        item
    }
    /// Returns a readonly iterator
    pub fn iter<S: ReadonlyStorage>(&self, storage: &'a S) -> StdResult<DequeStoreIter<T, S, Ser>> {
        let len = self.get_len(storage)?;
        let iter = DequeStoreIter::new(self, storage, 0, len);
        Ok(iter)
    }
    /// does paging with the given parameters
    pub fn paging<S: ReadonlyStorage>(&self, storage: &S, start_page: u32, size: u32) -> StdResult<Vec<T>> {
        self.iter(storage)?.skip((start_page as usize)*(size as usize)).take(size as usize).collect()
    }
}

impl<'a, T: Serialize + DeserializeOwned, Ser: Serde> PrefixedTypedStorage<T, Ser> for DequeStore<'a, T, Ser> {
    fn as_slice(&self) -> &[u8] {
        if let Some(prefix) = &self.prefix {
            prefix
        } else {
            self.namespace
        }
    }
}

/// An iterator over the contents of the deque store.
pub struct DequeStoreIter<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{
    deque_store: &'a DequeStore<'a, T, Ser>,
    storage: &'a S,
    start: u32,
    end: u32,
}

impl<'a, T, S, Ser> DequeStoreIter<'a, T, S, Ser>
    where
        T: Serialize + DeserializeOwned,
        S: ReadonlyStorage,
        Ser: Serde,
{
    /// constructor
    pub fn new(
        deque_store: &'a DequeStore<'a, T, Ser>,
        storage: &'a S,
        start: u32,
        end: u32
    ) -> Self {
        Self {
            deque_store,
            storage,
            start,
            end,
        }
    }
}

impl<'a, T, S, Ser> Iterator for DequeStoreIter<'a, T, S, Ser>
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
        let item = self.deque_store.get_at(self.storage, self.start);
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

impl<'a, T, S, Ser> DoubleEndedIterator for DequeStoreIter<'a, T, S, Ser>
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
        let item = self.deque_store.get_at(self.storage, self.end);
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
impl<'a, T, S, Ser> ExactSizeIterator for DequeStoreIter<'a, T, S, Ser>
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
        let deque_store: DequeStore<i32> = DequeStore::new(b"test");
        deque_store.push_front(&mut storage, &4)?;
        deque_store.push_back(&mut storage, &5)?;
        deque_store.push_front(&mut storage, &3)?;
        deque_store.push_back(&mut storage, &6)?;
        deque_store.push_front(&mut storage, &2)?;
        deque_store.push_back(&mut storage, &7)?;
        deque_store.push_front(&mut storage, &1)?;
        deque_store.push_back(&mut storage, &8)?;

        assert_eq!(deque_store.pop_front(&mut storage), Ok(1));
        assert_eq!(deque_store.pop_back(&mut storage), Ok(8));
        assert_eq!(deque_store.pop_front(&mut storage), Ok(2));
        assert_eq!(deque_store.pop_back(&mut storage), Ok(7));
        assert_eq!(deque_store.pop_front(&mut storage), Ok(3));
        assert_eq!(deque_store.pop_back(&mut storage), Ok(6));
        assert_eq!(deque_store.pop_front(&mut storage), Ok(4));
        assert_eq!(deque_store.pop_back(&mut storage), Ok(5));
        assert!(deque_store.pop_back(&mut storage).is_err());
        Ok(())
    }

    #[test]
    fn test_removes() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let deque_store: DequeStore<i32> = DequeStore::new(b"test");
        deque_store.push_front(&mut storage, &2)?;
        deque_store.push_back(&mut storage, &3)?;
        deque_store.push_back(&mut storage, &4)?;
        deque_store.push_back(&mut storage, &5)?;
        deque_store.push_back(&mut storage, &6)?;
        deque_store.push_front(&mut storage, &1)?;
        deque_store.push_back(&mut storage, &7)?;
        deque_store.push_back(&mut storage, &8)?;

        assert!(deque_store.remove(&mut storage, 8).is_err());
        assert!(deque_store.remove(&mut storage, 9).is_err());

        assert_eq!(deque_store.remove(&mut storage, 7), Ok(8));
        assert_eq!(deque_store.get_at(&storage, 6), Ok(7));
        assert_eq!(deque_store.get_at(&storage, 5), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 4), Ok(5));
        assert_eq!(deque_store.get_at(&storage, 3), Ok(4));
        assert_eq!(deque_store.get_at(&storage, 2), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(2));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 6), Ok(7));
        assert_eq!(deque_store.get_at(&storage, 5), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 4), Ok(5));
        assert_eq!(deque_store.get_at(&storage, 3), Ok(4));
        assert_eq!(deque_store.get_at(&storage, 2), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(2));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 3), Ok(4));
        assert_eq!(deque_store.get_at(&storage, 4), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 3), Ok(5));
        assert_eq!(deque_store.get_at(&storage, 2), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(2));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(2));
        assert_eq!(deque_store.get_at(&storage, 3), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 2), Ok(5));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 2), Ok(5));
        assert_eq!(deque_store.get_at(&storage, 2), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(3));
        assert_eq!(deque_store.get_at(&storage, 1), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(6));
        assert_eq!(deque_store.get_at(&storage, 0), Ok(1));

        assert_eq!(deque_store.remove(&mut storage, 0), Ok(1));

        assert!(deque_store.remove(&mut storage, 0).is_err());
        Ok(())
    }

    #[test]
    fn test_iterator() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let deque_store: DequeStore<i32> = DequeStore::new(b"test");

        deque_store.push_front(&mut storage, &2143)?;
        deque_store.push_back(&mut storage, &3333)?;
        deque_store.push_back(&mut storage, &3412)?;
        deque_store.push_front(&mut storage, &1234)?;
        deque_store.push_back(&mut storage, &4321)?;

        deque_store.remove(&mut storage, 2)?;

        // iterate twice to make sure nothing changed
        let mut iter = deque_store.iter(&storage)?;
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        let mut iter = deque_store.iter(&storage)?;
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth` doesn't break anything
        let mut iter = deque_store.iter(&storage)?.skip(2);
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_reverse_iterator() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let deque_store: DequeStore<i32> = DequeStore::new(b"test");
        deque_store.push_front(&mut storage, &2143)?;
        deque_store.push_back(&mut storage, &3412)?;
        deque_store.push_back(&mut storage, &3333)?;
        deque_store.push_front(&mut storage, &1234)?;
        deque_store.push_back(&mut storage, &4321)?;

        deque_store.remove(&mut storage, 3)?;

        let mut iter = deque_store.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // iterate twice to make sure nothing changed
        let mut iter = deque_store.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth_back` doesn't break anything
        let mut iter = deque_store.iter(&storage)?.rev().skip(2);
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `ExactSizeIterator` works well
        let mut iter = deque_store.iter(&storage)?.skip(2).rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_serializations() -> StdResult<()> {
        // Check the default behavior is Bincode2
        let mut storage = MockStorage::new();

        let deque_store: DequeStore<i32> = DequeStore::new(b"test");
        deque_store.push_back(&mut storage, &1234)?;

        let key = [deque_store.as_slice(), &0_u32.to_be_bytes()].concat();
        let bytes = storage.get(&key);
        assert_eq!(bytes, Some(vec![210, 4, 0, 0]));

        // Check that overriding the serializer with Json works
        let mut storage = MockStorage::new();
        let json_deque_store: DequeStore<i32, Json> = DequeStore::new(b"test2");
        json_deque_store.push_back(&mut storage, &1234)?;
        
        let key = [json_deque_store.as_slice(), &0_u32.to_be_bytes()].concat();
        let bytes = storage.get(&key);
        assert_eq!(bytes, Some(b"1234".to_vec()));

        Ok(())
    }

    #[test]
    fn test_paging() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let append_store: DequeStore<u32> = DequeStore::new(b"test");

        let page_size: u32 = 5;
        let total_items: u32 = 50;

        for j in 0..total_items {
            let i = total_items - j;
            append_store.push_front(&mut storage, &i)?;
        }

        for i in 0..((total_items / page_size) - 1) {
            let start_page = i;

            let values = append_store.paging(&storage, start_page, page_size)?;

            for (index, value) in values.iter().enumerate() {
                assert_eq!(value, &(page_size * start_page + index as u32 + 1))
            }
        }

        Ok(())
    }
}