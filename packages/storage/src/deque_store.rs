//! This is a storage wrapper based on AppendStore called DequeStore.
//! It guarantees constant-cost appending to and popping from a list of items in storage on both directions (front and back).
//!
//! This is achieved by storing each item in a separate storage entry.
//! A special key is reserved for storing the length of the collection so far.
//! Another special key is reserved for storing the offset of the collection.
use std::collections::HashMap;
use std::convert::TryInto;
use std::marker::PhantomData;
use std::sync::Mutex;

use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{StdError, StdResult, Storage};
use cosmwasm_storage::to_length_prefixed;

use secret_toolkit_serialization::{Bincode2, Serde};

const INDEXES: &[u8] = b"indexes";
const LEN_KEY: &[u8] = b"len";
const OFFSET_KEY: &[u8] = b"off";

const DEFAULT_PAGE_SIZE: u32 = 1;

pub struct DequeStore<'a, T, Ser = Bincode2>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// prefix of the newly constructed Storage
    namespace: &'a [u8],
    /// needed if any suffixes were added to the original namespace.
    prefix: Option<Vec<u8>>,
    page_size: u32,
    length: Mutex<Option<u32>>,
    offset: Mutex<Option<u32>>,
    item_type: PhantomData<T>,
    serialization_type: PhantomData<Ser>,
}

impl<'a, T: Serialize + DeserializeOwned, Ser: Serde> DequeStore<'a, T, Ser> {
    /// constructor
    pub const fn new(prefix: &'a [u8]) -> Self {
        Self {
            namespace: prefix,
            prefix: None,
            page_size: DEFAULT_PAGE_SIZE,
            length: Mutex::new(None),
            offset: Mutex::new(None),
            item_type: PhantomData,
            serialization_type: PhantomData,
        }
    }

    /// constructor with indexes size
    pub const fn new_with_page_size(prefix: &'a [u8], page_size: u32) -> Self {
        if page_size == 0 {
            panic!("zero index page size used in deque_store")
        }
        Self {
            namespace: prefix,
            prefix: None,
            page_size,
            length: Mutex::new(None),
            offset: Mutex::new(None),
            item_type: PhantomData,
            serialization_type: PhantomData,
        }
    }

    /// This is used to produce a new DequeStorage. This can be used when you want to associate an AppendListStorage to each user
    /// and you still get to define the DequeStorage as a static constant
    pub fn add_suffix(&self, suffix: &[u8]) -> Self {
        let suffix = to_length_prefixed(suffix);
        let prefix = self.prefix.as_deref().unwrap_or(self.namespace);
        let prefix = [prefix, suffix.as_slice()].concat();
        Self {
            namespace: self.namespace,
            prefix: Some(prefix),
            page_size: self.page_size,
            length: Mutex::new(None),
            offset: Mutex::new(None),
            item_type: self.item_type,
            serialization_type: self.serialization_type,
        }
    }
}

impl<'a, T: Serialize + DeserializeOwned, Ser: Serde> DequeStore<'a, T, Ser> {
    fn as_slice(&self) -> &[u8] {
        if let Some(prefix) = &self.prefix {
            prefix
        } else {
            self.namespace
        }
    }

    /// gets the length from storage, and otherwise sets it to 0
    pub fn len(&self, storage: &dyn Storage) -> StdResult<u32> {
        let mut may_len = self.length.lock().unwrap();
        match *may_len {
            Some(len) => Ok(len),
            None => match self.get_u32(storage, LEN_KEY) {
                Ok(len) => {
                    *may_len = Some(len);
                    Ok(len)
                }
                Err(e) => Err(e),
            },
        }
    }

    /// gets the offset from storage, and otherwise sets it to 0
    pub fn get_off(&self, storage: &dyn Storage) -> StdResult<u32> {
        let mut may_off = self.offset.lock().unwrap();
        match *may_off {
            Some(len) => Ok(len),
            None => match self.get_u32(storage, OFFSET_KEY) {
                Ok(len) => {
                    *may_off = Some(len);
                    Ok(len)
                }
                Err(e) => Err(e),
            },
        }
    }

    /// gets offset or length
    fn get_u32(&self, storage: &dyn Storage, key: &[u8]) -> StdResult<u32> {
        let num_key = [self.as_slice(), key].concat();
        if let Some(num_vec) = storage.get(&num_key) {
            let num_bytes = num_vec
                .as_slice()
                .try_into()
                .map_err(|err| StdError::parse_err("u32", err))?;
            let num = u32::from_be_bytes(num_bytes);
            Ok(num)
        } else {
            Ok(0)
        }
    }

    /// checks if the collection has any elements
    pub fn is_empty(&self, storage: &dyn Storage) -> StdResult<bool> {
        Ok(self.len(storage)? == 0)
    }

    /// gets the element at pos if within bounds
    pub fn get(&self, storage: &dyn Storage, pos: u32) -> StdResult<Option<T>> {
        let len = self.len(storage)?;
        if pos >= len {
            return Ok(None)
        }
        self.get_at_unchecked(storage, pos).map(Some)
    }

    /// Returns the last element of the deque without removing it
    pub fn front(&self, storage: &dyn Storage) -> StdResult<Option<T>> {
        if self.len(storage)? > 0 {
            let item = self.get_at_unchecked(storage, 0);
            item.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Returns the first element of the deque without removing it
    pub fn back(&self, storage: &dyn Storage) -> StdResult<Option<T>> {
        if let Some(len) = self.len(storage)?.checked_sub(1) {
            let item = self.get_at_unchecked(storage, len);
            item.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Used to get the indexes stored in the given page number
    fn get_indexes(&self, storage: &dyn Storage, page: u32) -> StdResult<HashMap<u32, Vec<u8>>> {
        let indexes_key = [self.as_slice(), INDEXES, page.to_be_bytes().as_slice()].concat();
        if self.page_size == 1 {
            let maybe_item_data = storage.get(&indexes_key);
            match maybe_item_data {
                Some(item_data) => {
                    let mut hashmap = HashMap::new();
                    hashmap.insert(0_u32, item_data);
                    Ok(hashmap)
                }
                None => Ok(HashMap::new()),
            }
        } else {
            let maybe_serialized = storage.get(&indexes_key);
            match maybe_serialized {
                Some(serialized) => Bincode2::deserialize(&serialized),
                None => Ok(HashMap::new()),
            }
        }
    }

    /// Set an indexes page
    fn set_indexes_page(
        &self,
        storage: &mut dyn Storage,
        page: u32,
        indexes: &HashMap<u32, Vec<u8>>,
    ) -> StdResult<()> {
        let indexes_key = [self.as_slice(), INDEXES, page.to_be_bytes().as_slice()].concat();
        if self.page_size == 1 {
            if let Some(item_data) = indexes.get(&0_u32) {
                storage.set(&indexes_key, item_data);
            } else {
                storage.remove(&indexes_key);
            }
        } else {
            storage.set(&indexes_key, &Bincode2::serialize(indexes)?);
        }
        Ok(())
    }

    /// tries to get the element at pos
    fn get_at_unchecked(&self, storage: &dyn Storage, pos: u32) -> StdResult<T> {
        let offset_pos = self.get_offset_pos(storage, pos)?;
        let indexes_page = offset_pos / self.page_size;
        let index_pos = offset_pos % self.page_size;
        let indexes = self.get_indexes(storage, indexes_page)?;
        let item_data = indexes
            .get(&index_pos)
            .ok_or_else(|| StdError::generic_err("item not found at this index"))?;
        Ser::deserialize(item_data)
    }

    /// add the offset to the pos
    fn get_offset_pos(&self, storage: &dyn Storage, pos: u32) -> StdResult<u32> {
        let off = self.get_off(storage)?;
        Ok(pos.overflowing_add(off).0)
    }

    /// Set the length of the collection
    fn set_len(&self, storage: &mut dyn Storage, len: u32) {
        let mut may_len = self.length.lock().unwrap();
        *may_len = Some(len);
        self._set_u32(storage, LEN_KEY, len)
    }

    /// Set the offset of the collection
    fn set_off(&self, storage: &mut dyn Storage, off: u32) {
        let mut may_off = self.offset.lock().unwrap();
        *may_off = Some(off);
        self._set_u32(storage, OFFSET_KEY, off)
    }

    /// Set the length or offset of the collection
    fn _set_u32(&self, storage: &mut dyn Storage, key: &[u8], num: u32) {
        let num_key = [self.as_slice(), key].concat();
        storage.set(&num_key, &num.to_be_bytes());
    }

    /// Clear the collection
    pub fn clear(&self, storage: &mut dyn Storage) {
        self.set_len(storage, 0);
        self.set_off(storage, 0);
    }

    /// Replaces data at a position within bounds
    pub fn set_at(&self, storage: &mut dyn Storage, pos: u32, item: &T) -> StdResult<()> {
        let len = self.len(storage)?;
        if pos >= len {
            return Err(StdError::generic_err("deque_store access out of bounds"));
        }
        self.set_at_unchecked(storage, pos, item)
    }

    /// Sets data at a given index
    fn set_at_unchecked(&self, storage: &mut dyn Storage, pos: u32, item: &T) -> StdResult<()> {
        let offset_pos = self.get_offset_pos(storage, pos)?;
        let indexes_page = offset_pos / self.page_size;
        let index_pos = offset_pos % self.page_size;
        let mut indexes = self.get_indexes(storage, indexes_page)?;
        let item_data = Ser::serialize(item)?;
        indexes.insert(index_pos, item_data);
        self.set_indexes_page(storage, indexes_page, &indexes)
    }

    /// Pushes an item to the back
    pub fn push_back(&self, storage: &mut dyn Storage, item: &T) -> StdResult<()> {
        let len = self.len(storage)?;
        self.set_at_unchecked(storage, len, item)?;
        self.set_len(storage, len + 1);
        Ok(())
    }

    /// Pushes an item to the front
    pub fn push_front(&self, storage: &mut dyn Storage, item: &T) -> StdResult<()> {
        let off = self.get_off(storage)?;
        let len = self.len(storage)?;
        self.set_off(storage, off.overflowing_sub(1).0);
        self.set_at_unchecked(storage, 0, item)?;
        self.set_len(storage, len + 1);
        Ok(())
    }

    /// Pops an item from the back
    pub fn pop_back(&self, storage: &mut dyn Storage) -> StdResult<Option<T>> {
        if let Some(len) = self.len(storage)?.checked_sub(1) {
            self.set_len(storage, len);
            self.get_at_unchecked(storage, len).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Pops an item from the front
    pub fn pop_front(&self, storage: &mut dyn Storage) -> StdResult<Option<T>> {
        if let Some(len) = self.len(storage)?.checked_sub(1) {
            let off = self.get_off(storage)?;
            self.set_len(storage, len);
            let item = self.get_at_unchecked(storage, 0);
            self.set_off(storage, off.overflowing_add(1).0);
            item.map(Some)
        } else {
            Ok(None)
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
    pub fn remove(&self, storage: &mut dyn Storage, pos: u32) -> StdResult<T> {
        let off = self.get_off(storage)?;
        let len = self.len(storage)?;
        if pos >= len {
            return Err(StdError::generic_err("deque_store access out of bounds"));
        }
        let res;
        let to_tail = len - pos;
        if to_tail < pos {
            let past_offset_pos = self.get_offset_pos(storage, pos)?;
            let mut past_indexes_page = past_offset_pos / self.page_size;
            let mut past_index_pos = past_offset_pos % self.page_size;
            let mut past_indexes = self.get_indexes(storage, past_indexes_page)?;
            res = Ser::deserialize(
                &past_indexes
                    .remove(&past_index_pos)
                    .ok_or_else(|| StdError::generic_err("item not found at this index"))?,
            );
            // closer to the tail
            for i in (pos + 1)..len {
                let offset_pos = self.get_offset_pos(storage, i)?;
                let current_page = offset_pos / self.page_size;
                let index_pos = offset_pos % self.page_size;
                if current_page != past_indexes_page {
                    let mut indexes = self.get_indexes(storage, current_page)?;
                    let item_data = indexes
                        .remove(&index_pos)
                        .ok_or_else(|| StdError::generic_err("item not found at this index"))?;
                    past_indexes.insert(past_index_pos, item_data);
                    self.set_indexes_page(storage, past_indexes_page, &past_indexes)?;
                    past_indexes = indexes;
                } else {
                    let item_data_move_down = past_indexes
                        .remove(&index_pos)
                        .ok_or_else(|| StdError::generic_err("item not found at this index"))?;
                    past_indexes.insert(past_index_pos, item_data_move_down);
                }
                past_indexes_page = current_page;
                past_index_pos = index_pos;
            }
            self.set_indexes_page(storage, past_indexes_page, &past_indexes)?;
        } else {
            let past_offset_pos = self.get_offset_pos(storage, pos)?;
            let mut past_indexes_page = past_offset_pos / self.page_size;
            let mut past_index_pos = past_offset_pos % self.page_size;
            let mut past_indexes = self.get_indexes(storage, past_indexes_page)?;
            res = Ser::deserialize(
                &past_indexes
                    .remove(&past_index_pos)
                    .ok_or_else(|| StdError::generic_err("item not found at this index"))?,
            );
            // closer to the head
            for i in (0..pos).rev() {
                let offset_pos = self.get_offset_pos(storage, i)?;
                let current_page = offset_pos / self.page_size;
                let index_pos = offset_pos % self.page_size;
                if current_page != past_indexes_page {
                    let mut indexes = self.get_indexes(storage, current_page)?;
                    let item_data = indexes
                        .remove(&index_pos)
                        .ok_or_else(|| StdError::generic_err("item not found at this index"))?;
                    past_indexes.insert(past_index_pos, item_data);
                    self.set_indexes_page(storage, past_indexes_page, &past_indexes)?;
                    past_indexes = indexes;
                } else {
                    let item_data_move_up = past_indexes
                        .remove(&index_pos)
                        .ok_or_else(|| StdError::generic_err("item not found at this index"))?;
                    past_indexes.insert(past_index_pos, item_data_move_up);
                }
                past_indexes_page = current_page;
                past_index_pos = index_pos;
            }
            self.set_indexes_page(storage, past_indexes_page, &past_indexes)?;
            self.set_off(storage, off.overflowing_add(1).0);
        }
        self.set_len(storage, len - 1);
        res
    }

    /// Returns a readonly iterator
    pub fn iter(&self, storage: &'a dyn Storage) -> StdResult<DequeStoreIter<T, Ser>> {
        let len = self.len(storage)?;
        let iter = DequeStoreIter::new(self, storage, 0, len);
        Ok(iter)
    }

    /// does paging with the given parameters
    pub fn paging(&self, storage: &dyn Storage, start_page: u32, size: u32) -> StdResult<Vec<T>> {
        self.iter(storage)?
            .skip((start_page as usize) * (size as usize))
            .take(size as usize)
            .collect()
    }
}

/// An iterator over the contents of the deque store.
pub struct DequeStoreIter<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    deque_store: &'a DequeStore<'a, T, Ser>,
    storage: &'a dyn Storage,
    start: u32,
    end: u32,
    cache: HashMap<u32, HashMap<u32, Vec<u8>>>,
}

impl<'a, T, Ser> DequeStoreIter<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// constructor
    pub fn new(
        deque_store: &'a DequeStore<'a, T, Ser>,
        storage: &'a dyn Storage,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            deque_store,
            storage,
            start,
            end,
            cache: HashMap::new(),
        }
    }
}

impl<'a, T, Ser> Iterator for DequeStoreIter<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    type Item = StdResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        let item;
        match self.deque_store.get_offset_pos(self.storage, self.start) {
            Ok(offset_pos) => {
                let indexes_page = offset_pos / self.deque_store.page_size;
                let index_pos = offset_pos % self.deque_store.page_size;
                match self.cache.get(&indexes_page) {
                    Some(indexes) => {
                        if let Some(item_data) = indexes.get(&index_pos) {
                            item = Ser::deserialize(item_data);
                        } else {
                            item = Err(StdError::generic_err("item not found at this index"));
                        }
                    }
                    None => match self.deque_store.get_indexes(self.storage, indexes_page) {
                        Ok(indexes) => {
                            if let Some(item_data) = indexes.get(&index_pos) {
                                item = Ser::deserialize(item_data);
                            } else {
                                item = Err(StdError::generic_err("item not found at this index"));
                            }
                            self.cache.insert(indexes_page, indexes);
                        }
                        Err(e) => {
                            item = Err(e);
                        }
                    },
                }
            }
            Err(e) => {
                item = Err(e);
            }
        }
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
        // make sure that we don't skip past the end
        if calc_len(self.start, self.end) < n as u32 {
            // mark as empty
            self.start = self.end;
        } else {
            self.start = self.start.saturating_add(n as u32);
        }
        self.next()
    }
}

#[inline]
fn calc_len(head: u32, tail: u32) -> u32 {
    tail.saturating_sub(head)
}

impl<'a, T, Ser> DoubleEndedIterator for DequeStoreIter<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        self.end -= 1;
        let item;
        match self.deque_store.get_offset_pos(self.storage, self.end) {
            Ok(offset_pos) => {
                let indexes_page = offset_pos / self.deque_store.page_size;
                let index_pos = offset_pos % self.deque_store.page_size;
                match self.cache.get(&indexes_page) {
                    Some(indexes) => {
                        if let Some(item_data) = indexes.get(&index_pos) {
                            item = Ser::deserialize(item_data);
                        } else {
                            item = Err(StdError::generic_err("item not found at this index"));
                        }
                    }
                    None => match self.deque_store.get_indexes(self.storage, indexes_page) {
                        Ok(indexes) => {
                            if let Some(item_data) = indexes.get(&index_pos) {
                                item = Ser::deserialize(item_data);
                            } else {
                                item = Err(StdError::generic_err("item not found at this index"));
                            }
                            self.cache.insert(indexes_page, indexes);
                        }
                        Err(e) => {
                            item = Err(e);
                        }
                    },
                }
            }
            Err(e) => {
                item = Err(e);
            }
        }
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
impl<'a, T, Ser> ExactSizeIterator for DequeStoreIter<'a, T, Ser>
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
    fn test_pushs_pops() -> StdResult<()> {
        test_pushs_pops_with_size(1)?;
        test_pushs_pops_with_size(2)?;
        test_pushs_pops_with_size(5)?;
        test_pushs_pops_with_size(13)?;
        Ok(())
    }

    fn test_pushs_pops_with_size(page_size: u32) -> StdResult<()> {
        let mut storage = MockStorage::new();
        let deque_store: DequeStore<i32> = DequeStore::new_with_page_size(b"test", page_size);
        deque_store.push_front(&mut storage, &4)?;
        deque_store.push_back(&mut storage, &5)?;
        deque_store.push_front(&mut storage, &3)?;
        deque_store.push_back(&mut storage, &6)?;
        deque_store.push_front(&mut storage, &2)?;
        deque_store.push_back(&mut storage, &7)?;
        deque_store.push_front(&mut storage, &1)?;
        deque_store.push_back(&mut storage, &8)?;

        assert_eq!(deque_store.pop_front(&mut storage)?, Some(1));
        assert_eq!(deque_store.pop_back(&mut storage)?, Some(8));
        assert_eq!(deque_store.pop_front(&mut storage)?, Some(2));
        assert_eq!(deque_store.pop_back(&mut storage)?, Some(7));
        assert_eq!(deque_store.pop_front(&mut storage)?, Some(3));
        assert_eq!(deque_store.pop_back(&mut storage)?, Some(6));
        assert_eq!(deque_store.pop_front(&mut storage)?, Some(4));
        assert_eq!(deque_store.pop_back(&mut storage)?, Some(5));
        assert!(deque_store.pop_back(&mut storage)?.is_none());
        Ok(())
    }

    #[test]
    fn test_removes() -> StdResult<()> {
        test_removes_with_page_size(1)?;
        test_removes_with_page_size(3)?;
        test_removes_with_page_size(5)?;
        test_removes_with_page_size(7)?;
        test_removes_with_page_size(13)?;

        Ok(())
    }

    #[test]
    fn test_remove() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let deque_store: DequeStore<i32> = DequeStore::new(b"test");
        deque_store.push_front(&mut storage, &2143)?;
        deque_store.push_back(&mut storage, &3412)?;
        deque_store.push_back(&mut storage, &3333)?;
        deque_store.push_front(&mut storage, &1234)?;
        deque_store.push_back(&mut storage, &4321)?;

        assert_eq!(deque_store.len(&storage), Ok(5));

        assert_eq!(deque_store.remove(&mut storage, 3), Ok(3333));

        assert_eq!(deque_store.len(&storage), Ok(4));

        assert_eq!(deque_store.get(&storage, 0)?, Some(1234));
        assert_eq!(deque_store.get(&storage, 1)?, Some(2143));
        assert_eq!(deque_store.get(&storage, 2)?, Some(3412));
        assert_eq!(deque_store.get(&storage, 3)?, Some(4321));

        Ok(())
    }

    fn test_removes_with_page_size(size: u32) -> StdResult<()> {
        let mut storage = MockStorage::new();
        let deque_store: DequeStore<i32> = DequeStore::new_with_page_size(b"test", size);
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
        assert_eq!(deque_store.get(&storage, 6)?, Some(7));
        assert_eq!(deque_store.get(&storage, 5)?, Some(6));
        assert_eq!(deque_store.get(&storage, 4)?, Some(5));
        assert_eq!(deque_store.get(&storage, 3)?, Some(4));
        assert_eq!(deque_store.get(&storage, 2)?, Some(3));
        assert_eq!(deque_store.get(&storage, 1)?, Some(2));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 6), Ok(7));
        assert_eq!(deque_store.get(&storage, 5)?, Some(6));
        assert_eq!(deque_store.get(&storage, 4)?, Some(5));
        assert_eq!(deque_store.get(&storage, 3)?, Some(4));
        assert_eq!(deque_store.get(&storage, 2)?, Some(3));
        assert_eq!(deque_store.get(&storage, 1)?, Some(2));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 3), Ok(4));
        assert_eq!(deque_store.get(&storage, 4)?, Some(6));
        assert_eq!(deque_store.get(&storage, 3)?, Some(5));
        assert_eq!(deque_store.get(&storage, 2)?, Some(3));
        assert_eq!(deque_store.get(&storage, 1)?, Some(2));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(2));
        assert_eq!(deque_store.get(&storage, 3)?, Some(6));
        assert_eq!(deque_store.get(&storage, 2)?, Some(5));
        assert_eq!(deque_store.get(&storage, 1)?, Some(3));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 2), Ok(5));
        assert_eq!(deque_store.get(&storage, 2)?, Some(6));
        assert_eq!(deque_store.get(&storage, 1)?, Some(3));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(3));
        assert_eq!(deque_store.get(&storage, 1)?, Some(6));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(6));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 0), Ok(1));

        assert!(deque_store.remove(&mut storage, 0).is_err());
        Ok(())
    }

    #[test]
    fn test_overwrite() -> StdResult<()> {
        test_overwrite_with_page_size(1)?;
        test_overwrite_with_page_size(6)?;
        test_overwrite_with_page_size(9)?;
        test_overwrite_with_page_size(13)?;
        test_overwrite_with_page_size(27)?;

        Ok(())
    }

    fn test_overwrite_with_page_size(size: u32) -> StdResult<()> {
        let mut storage = MockStorage::new();
        let deque_store: DequeStore<i32> = DequeStore::new_with_page_size(b"test", size);
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
        assert_eq!(deque_store.get(&storage, 6)?, Some(7));
        assert_eq!(deque_store.get(&storage, 5)?, Some(6));
        assert_eq!(deque_store.get(&storage, 4)?, Some(5));
        assert_eq!(deque_store.get(&storage, 3)?, Some(4));
        assert_eq!(deque_store.get(&storage, 2)?, Some(3));
        assert_eq!(deque_store.get(&storage, 1)?, Some(2));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 6), Ok(7));
        assert_eq!(deque_store.get(&storage, 5)?, Some(6));
        assert_eq!(deque_store.get(&storage, 4)?, Some(5));
        assert_eq!(deque_store.get(&storage, 3)?, Some(4));
        assert_eq!(deque_store.get(&storage, 2)?, Some(3));
        assert_eq!(deque_store.get(&storage, 1)?, Some(2));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 3), Ok(4));
        assert_eq!(deque_store.get(&storage, 4)?, Some(6));
        assert_eq!(deque_store.get(&storage, 3)?, Some(5));
        assert_eq!(deque_store.get(&storage, 2)?, Some(3));
        assert_eq!(deque_store.get(&storage, 1)?, Some(2));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));
        assert!(deque_store.get(&storage, 5)?.is_none());

        deque_store.push_back(&mut storage, &5)?;
        assert_eq!(deque_store.get(&storage, 5)?, Some(5));
        assert_eq!(deque_store.get(&storage, 4)?, Some(6));
        assert_eq!(deque_store.get(&storage, 3)?, Some(5));
        assert_eq!(deque_store.get(&storage, 2)?, Some(3));
        assert_eq!(deque_store.get(&storage, 1)?, Some(2));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(2));
        assert_eq!(deque_store.get(&storage, 4)?, Some(5));
        assert_eq!(deque_store.get(&storage, 3)?, Some(6));
        assert_eq!(deque_store.get(&storage, 2)?, Some(5));
        assert_eq!(deque_store.get(&storage, 1)?, Some(3));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 2), Ok(5));
        assert_eq!(deque_store.get(&storage, 3)?, Some(5));
        assert_eq!(deque_store.get(&storage, 2)?, Some(6));
        assert_eq!(deque_store.get(&storage, 1)?, Some(3));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(3));
        assert_eq!(deque_store.get(&storage, 2)?, Some(5));
        assert_eq!(deque_store.get(&storage, 1)?, Some(6));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 1), Ok(6));
        assert_eq!(deque_store.get(&storage, 1)?, Some(5));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

        assert_eq!(deque_store.remove(&mut storage, 0), Ok(1));
        assert_eq!(deque_store.remove(&mut storage, 0), Ok(5));

        assert!(deque_store.remove(&mut storage, 0).is_err());

        deque_store.push_front(&mut storage, &2)?;
        deque_store.push_back(&mut storage, &3)?;
        deque_store.push_back(&mut storage, &4)?;
        deque_store.push_back(&mut storage, &5)?;
        deque_store.push_back(&mut storage, &6)?;
        deque_store.push_front(&mut storage, &1)?;
        deque_store.push_back(&mut storage, &7)?;
        deque_store.push_back(&mut storage, &8)?;

        assert_eq!(deque_store.get(&storage, 7)?, Some(8));
        assert_eq!(deque_store.get(&storage, 6)?, Some(7));
        assert_eq!(deque_store.get(&storage, 5)?, Some(6));
        assert_eq!(deque_store.get(&storage, 4)?, Some(5));
        assert_eq!(deque_store.get(&storage, 3)?, Some(4));
        assert_eq!(deque_store.get(&storage, 2)?, Some(3));
        assert_eq!(deque_store.get(&storage, 1)?, Some(2));
        assert_eq!(deque_store.get(&storage, 0)?, Some(1));

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
        test_reverse_iterator_with_size(1)?;
        test_reverse_iterator_with_size(3)?;
        test_reverse_iterator_with_size(4)?;
        test_reverse_iterator_with_size(5)?;
        test_reverse_iterator_with_size(17)?;
        Ok(())
    }

    fn test_reverse_iterator_with_size(page_size: u32) -> StdResult<()> {
        let mut storage = MockStorage::new();
        let deque_store: DequeStore<i32> = DequeStore::new_with_page_size(b"test", page_size);
        deque_store.push_front(&mut storage, &2143)?;
        deque_store.push_back(&mut storage, &3412)?;
        deque_store.push_back(&mut storage, &3333)?;
        deque_store.push_front(&mut storage, &1234)?;
        deque_store.push_back(&mut storage, &4321)?;

        assert_eq!(deque_store.remove(&mut storage, 3), Ok(3333));

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
        test_serializations_with_page_size(1)?;
        test_serializations_with_page_size(2)?;
        test_serializations_with_page_size(5)?;
        Ok(())
    }

    fn test_serializations_with_page_size(page_size: u32) -> StdResult<()> {
        // Check the default behavior is Bincode2
        let mut storage = MockStorage::new();

        let deque_store: DequeStore<i32> = DequeStore::new_with_page_size(b"test", page_size);
        deque_store.push_back(&mut storage, &1234)?;

        let key = [deque_store.as_slice(), INDEXES, &0_u32.to_be_bytes()].concat();
        if deque_store.page_size == 1 {
            let item_data = storage.get(&key);
            assert_eq!(item_data, Some(Bincode2::serialize(&1234)?));
        } else {
            let bytes = storage.get(&key);
            let mut expected: HashMap<u32, Vec<u8>> = HashMap::new();
            expected.insert(0_u32, Bincode2::serialize(&1234)?);
            assert_eq!(bytes, Some(Bincode2::serialize(&expected)?));
        }

        // Check that overriding the serializer with Json works
        let mut storage = MockStorage::new();
        let json_deque_store: DequeStore<i32, Json> =
            DequeStore::new_with_page_size(b"test2", page_size);
        json_deque_store.push_back(&mut storage, &1234)?;

        let key = [json_deque_store.as_slice(), INDEXES, &0_u32.to_be_bytes()].concat();

        if deque_store.page_size == 1 {
            let item_data = storage.get(&key);
            assert_eq!(item_data, Some(b"1234".to_vec()));
        } else {
            let bytes = storage.get(&key);
            let mut expected: HashMap<u32, Vec<u8>> = HashMap::new();
            expected.insert(0_u32, b"1234".to_vec());
            assert_eq!(bytes, Some(Bincode2::serialize(&expected)?));
        }

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

    #[test]
    fn test_iterator_detect_skip() {
        let deque: DequeStore<u32> = DequeStore::new("test".as_bytes());
        let mut store = MockStorage::new();

        // push some items
        deque.push_back(&mut store, &1).unwrap();
        deque.push_back(&mut store, &2).unwrap();
        deque.push_back(&mut store, &3).unwrap();
        deque.push_back(&mut store, &4).unwrap();

        let items: StdResult<Vec<_>> = deque.iter(&store).unwrap().collect();
        assert_eq!(items.unwrap(), [1, 2, 3, 4]);

        // nth should work correctly
        let mut iter = deque.iter(&store).unwrap();
        assert_eq!(iter.nth(6), None);
        assert_eq!(iter.start, iter.end, "iter should detect skipping too far");
        assert_eq!(iter.next(), None);

        let mut iter = deque.iter(&store).unwrap();
        assert_eq!(iter.nth(1).unwrap().unwrap(), 2);
        assert_eq!(iter.next().unwrap().unwrap(), 3);
    }

    #[test]
    fn front_back() {
        let deque: DequeStore<u64> = DequeStore::new(b"test");
        let mut store = MockStorage::new();

        assert_eq!(deque.back(&store).unwrap(), None);
        deque.push_back(&mut store, &1).unwrap();
        assert_eq!(deque.back(&store).unwrap(), Some(1));
        assert_eq!(deque.front(&store).unwrap(), Some(1));
        deque.push_back(&mut store, &2).unwrap();
        assert_eq!(deque.back(&store).unwrap(), Some(2));
        assert_eq!(deque.front(&store).unwrap(), Some(1));
        deque.push_front(&mut store, &3).unwrap();
        assert_eq!(deque.back(&store).unwrap(), Some(2));
        assert_eq!(deque.front(&store).unwrap(), Some(3));
    }
}
