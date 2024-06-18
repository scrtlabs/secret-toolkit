//! An "append store" is a storage wrapper that guarantees constant-cost appending to and popping
//! from a list of items in storage.
//!
//! This is achieved by storing each item in a separate storage entry. A special key is reserved
//! for storing the length of the collection so far.
use std::marker::PhantomData;
use std::sync::Mutex;
use std::{collections::HashMap, convert::TryInto};

use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{StdError, StdResult, Storage};
use cosmwasm_storage::to_length_prefixed;

use secret_toolkit_serialization::{Bincode2, Serde};

const INDEXES: &[u8] = b"indexes";
const LEN_KEY: &[u8] = b"len";

const DEFAULT_PAGE_SIZE: u32 = 1;

pub struct AppendStore<'a, T, Ser = Bincode2>
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
    item_type: PhantomData<T>,
    serialization_type: PhantomData<Ser>,
}

impl<'a, T: Serialize + DeserializeOwned, Ser: Serde> AppendStore<'a, T, Ser> {
    /// constructor
    pub const fn new(namespace: &'a [u8]) -> Self {
        Self {
            namespace,
            prefix: None,
            page_size: DEFAULT_PAGE_SIZE,
            length: Mutex::new(None),
            item_type: PhantomData,
            serialization_type: PhantomData,
        }
    }

    pub const fn new_with_page_size(namespace: &'a [u8], page_size: u32) -> Self {
        if page_size == 0 {
            panic!("zero index page size used in append_store")
        }
        Self {
            namespace,
            prefix: None,
            page_size,
            length: Mutex::new(None),
            item_type: PhantomData,
            serialization_type: PhantomData,
        }
    }

    /// This is used to produce a new AppendListStorage. This can be used when you want to associate an AppendListStorage to each user
    /// and you still get to define the AppendListStorage as a static constant
    pub fn add_suffix(&self, suffix: &[u8]) -> Self {
        let suffix = to_length_prefixed(suffix);
        let prefix = self.prefix.as_deref().unwrap_or(self.namespace);
        let prefix = [prefix, suffix.as_slice()].concat();
        Self {
            namespace: self.namespace,
            prefix: Some(prefix),
            page_size: self.page_size,
            length: Mutex::new(None),
            item_type: self.item_type,
            serialization_type: self.serialization_type,
        }
    }
}

impl<'a, T: Serialize + DeserializeOwned, Ser: Serde> AppendStore<'a, T, Ser> {
    fn as_slice(&self) -> &[u8] {
        if let Some(prefix) = &self.prefix {
            prefix
        } else {
            self.namespace
        }
    }

    fn page_from_position(&self, position: u32) -> u32 {
        position / self.page_size
    }

    /// Used to get the indexes stored in the given page number
    fn get_indexes(&self, storage: &dyn Storage, page: u32) -> StdResult<Vec<Vec<u8>>> {
        let indexes_key = [self.as_slice(), INDEXES, page.to_be_bytes().as_slice()].concat();
        if self.page_size == 1 {
            let maybe_item_data = storage.get(&indexes_key);
            match maybe_item_data {
                Some(item_data) => Ok(vec![item_data]),
                None => Ok(vec![]),
            }
        } else {
            let maybe_serialized = storage.get(&indexes_key);
            match maybe_serialized {
                Some(serialized) => Bincode2::deserialize(&serialized),
                None => Ok(vec![]),
            }
        }
    }

    /// Set an indexes page
    fn set_indexes_page(
        &self,
        storage: &mut dyn Storage,
        page: u32,
        indexes: &Vec<Vec<u8>>,
    ) -> StdResult<()> {
        let indexes_key = [self.as_slice(), INDEXES, page.to_be_bytes().as_slice()].concat();
        if self.page_size == 1 {
            if let Some(item_data) = indexes.first() {
                storage.set(&indexes_key, item_data);
            } else {
                storage.remove(&indexes_key);
            }
        } else {
            storage.set(&indexes_key, &Bincode2::serialize(indexes)?);
        }
        Ok(())
    }

    /// gets the length from storage, and otherwise sets it to 0
    pub fn get_len(&self, storage: &dyn Storage) -> StdResult<u32> {
        let mut may_len = self.length.lock().unwrap();
        match *may_len {
            Some(len) => Ok(len),
            None => {
                let len_key = [self.as_slice(), LEN_KEY].concat();
                if let Some(len_vec) = storage.get(&len_key) {
                    let len_bytes = len_vec
                        .as_slice()
                        .try_into()
                        .map_err(|err| StdError::parse_err("u32", err))?;
                    let len = u32::from_be_bytes(len_bytes);
                    *may_len = Some(len);
                    Ok(len)
                } else {
                    *may_len = Some(0);
                    Ok(0)
                }
            }
        }
    }

    /// checks if the collection has any elements
    pub fn is_empty(&self, storage: &dyn Storage) -> StdResult<bool> {
        Ok(self.get_len(storage)? == 0)
    }

    /// gets the element at pos if within bounds
    pub fn get_at(&self, storage: &dyn Storage, pos: u32) -> StdResult<T> {
        let len = self.get_len(storage)?;
        if pos >= len {
            return Err(StdError::generic_err("append_store access out of bounds"));
        }
        self.get_at_unchecked(storage, pos)
    }

    /// tries to get the element at pos
    fn get_at_unchecked(&self, storage: &dyn Storage, pos: u32) -> StdResult<T> {
        let page = self.page_from_position(pos);
        let indexes = self.get_indexes(storage, page)?;
        let index_pos = (pos % self.page_size) as usize;
        let item_data = &indexes[index_pos];
        Ser::deserialize(item_data)
    }

    /// Set the length of the collection
    fn set_len(&self, storage: &mut dyn Storage, len: u32) {
        let len_key = [self.as_slice(), LEN_KEY].concat();
        storage.set(&len_key, &len.to_be_bytes());

        let mut may_len = self.length.lock().unwrap();
        *may_len = Some(len);
    }

    /// Clear the collection
    pub fn clear(&self, storage: &mut dyn Storage) {
        self.set_len(storage, 0);
    }

    /// Replaces data at a position within bounds
    pub fn set_at(&self, storage: &mut dyn Storage, pos: u32, item: &T) -> StdResult<()> {
        let len = self.get_len(storage)?;
        if pos >= len {
            return Err(StdError::generic_err("append_store access out of bounds"));
        }
        self.set_at_unchecked(storage, pos, item)
    }

    /// Sets data at a given index
    fn set_at_unchecked(&self, storage: &mut dyn Storage, pos: u32, item: &T) -> StdResult<()> {
        let page = self.page_from_position(pos);
        let mut indexes = self.get_indexes(storage, page)?;
        let index_pos = (pos % self.page_size) as usize;
        let item_data = Ser::serialize(item)?;
        if indexes.len() > index_pos {
            indexes[index_pos] = item_data
        } else {
            indexes.push(item_data)
        }
        self.set_indexes_page(storage, page, &indexes)
    }

    /// Pushes an item to AppendStorage
    pub fn push(&self, storage: &mut dyn Storage, item: &T) -> StdResult<()> {
        let len = self.get_len(storage)?;
        self.set_at_unchecked(storage, len, item)?;
        self.set_len(storage, len + 1);
        Ok(())
    }

    /// Pops an item from AppendStore
    pub fn pop(&self, storage: &mut dyn Storage) -> StdResult<T> {
        if let Some(len) = self.get_len(storage)?.checked_sub(1) {
            self.set_len(storage, len);
            self.get_at_unchecked(storage, len)
        } else {
            Err(StdError::generic_err("cannot pop from empty append_store"))
        }
    }

    /// Remove an element from the collection at the specified position.
    ///
    /// Removing the last element has a constant cost.
    /// The cost of removing from the middle/start will depend on the proximity to tail of the list.
    /// All elements above the specified position will be shifted in storage.
    ///
    /// Removing an element from the start (head) of the collection
    /// has the worst runtime and gas cost.
    pub fn remove(&self, storage: &mut dyn Storage, pos: u32) -> StdResult<T> {
        let len = self.get_len(storage)?;

        if pos >= len {
            return Err(StdError::generic_err("append_store access out of bounds"));
        }
        let max_pos = len - 1;
        let max_page = self.page_from_position(max_pos);
        let pos_page = self.page_from_position(pos);

        match pos_page.cmp(&max_page) {
            std::cmp::Ordering::Less => {
                // shift items from indexes to indexes
                let mut past_indexes: Vec<Vec<u8>> = self.get_indexes(storage, pos_page)?;
                let item_data = past_indexes.remove((pos % self.page_size) as usize);
                // loop on
                for page in (pos_page + 1)..=max_page {
                    let mut indexes: Vec<Vec<u8>> = self.get_indexes(storage, page)?;
                    let next_item_data = indexes.remove(0);
                    past_indexes.push(next_item_data);
                    self.set_indexes_page(storage, page - 1, &past_indexes)?;
                    past_indexes = indexes;
                }
                // here past_indexes will have become the max_page indexes
                self.set_indexes_page(storage, max_page, &past_indexes)?;
                self.set_len(storage, max_pos);
                Ser::deserialize(&item_data)
            }
            std::cmp::Ordering::Equal => {
                // if the pos is in the last indexes page
                let mut indexes = self.get_indexes(storage, pos_page)?;
                let item_data = indexes.remove((pos % self.page_size) as usize);
                self.set_indexes_page(storage, pos_page, &indexes)?;
                self.set_len(storage, max_pos);
                Ser::deserialize(&item_data)
            }
            std::cmp::Ordering::Greater => {
                Err(StdError::generic_err("append_store access out of bounds"))
            }
        }
    }

    /// Returns a readonly iterator
    pub fn iter(&self, storage: &'a dyn Storage) -> StdResult<AppendStoreIter<T, Ser>> {
        let len = self.get_len(storage)?;
        let iter = AppendStoreIter::new(self, storage, 0, len);
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

/// An iterator over the contents of the append store.
pub struct AppendStoreIter<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    append_store: &'a AppendStore<'a, T, Ser>,
    storage: &'a dyn Storage,
    start: u32,
    end: u32,
    cache: HashMap<u32, Vec<Vec<u8>>>,
}

impl<'a, T, Ser> AppendStoreIter<'a, T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// constructor
    pub fn new(
        append_store: &'a AppendStore<'a, T, Ser>,
        storage: &'a dyn Storage,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            append_store,
            storage,
            start,
            end,
            cache: HashMap::new(),
        }
    }
}

impl<'a, T, Ser> Iterator for AppendStoreIter<'a, T, Ser>
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
        let page = self.append_store.page_from_position(self.start);
        let indexes_pos = (self.start % self.append_store.page_size) as usize;

        match self.cache.get(&page) {
            Some(indexes) => {
                let item_data = &indexes[indexes_pos];
                item = Ser::deserialize(item_data);
            }
            None => match self.append_store.get_indexes(self.storage, page) {
                Ok(indexes) => {
                    let item_data = &indexes[indexes_pos];
                    item = Ser::deserialize(item_data);
                    self.cache.insert(page, indexes);
                }
                Err(e) => {
                    item = Err(e);
                }
            },
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
    // `append_store.iter().skip(start).take(length).collect()`
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.start = self.start.saturating_add(n as u32);
        self.next()
    }
}

impl<'a, T, Ser> DoubleEndedIterator for AppendStoreIter<'a, T, Ser>
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
        let page = self.append_store.page_from_position(self.end);
        let indexes_pos = (self.end % self.append_store.page_size) as usize;
        match self.cache.get(&page) {
            Some(indexes) => {
                let item_data = &indexes[indexes_pos];
                item = Ser::deserialize(item_data);
            }
            None => match self.append_store.get_indexes(self.storage, page) {
                Ok(indexes) => {
                    let item_data = &indexes[indexes_pos];
                    item = Ser::deserialize(item_data);
                    self.cache.insert(page, indexes);
                }
                Err(e) => {
                    item = Err(e);
                }
            },
        }
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
impl<'a, T, Ser> ExactSizeIterator for AppendStoreIter<'a, T, Ser>
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
        let append_store: AppendStore<i32> = AppendStore::new(b"test");
        append_store.push(&mut storage, &1234)?;
        append_store.push(&mut storage, &2143)?;
        append_store.push(&mut storage, &3412)?;
        append_store.push(&mut storage, &4321)?;

        assert_eq!(append_store.pop(&mut storage), Ok(4321));
        assert_eq!(append_store.pop(&mut storage), Ok(3412));
        assert_eq!(append_store.pop(&mut storage), Ok(2143));
        assert_eq!(append_store.pop(&mut storage), Ok(1234));
        assert!(append_store.pop(&mut storage).is_err());

        Ok(())
    }

    #[test]
    fn test_length() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let append_store: AppendStore<i32> = AppendStore::new_with_page_size(b"test", 3);

        assert!(append_store.length.lock().unwrap().eq(&None));
        assert_eq!(append_store.get_len(&storage)?, 0);
        assert!(append_store.length.lock().unwrap().eq(&Some(0)));

        append_store.push(&mut storage, &1234)?;
        append_store.push(&mut storage, &2143)?;
        append_store.push(&mut storage, &3412)?;
        append_store.push(&mut storage, &4321)?;
        assert!(append_store.length.lock().unwrap().eq(&Some(4)));
        assert_eq!(append_store.get_len(&storage)?, 4);

        assert_eq!(append_store.pop(&mut storage), Ok(4321));
        assert_eq!(append_store.pop(&mut storage), Ok(3412));
        assert!(append_store.length.lock().unwrap().eq(&Some(2)));
        assert_eq!(append_store.get_len(&storage)?, 2);

        assert_eq!(append_store.pop(&mut storage), Ok(2143));
        assert_eq!(append_store.pop(&mut storage), Ok(1234));
        assert!(append_store.length.lock().unwrap().eq(&Some(0)));
        assert_eq!(append_store.get_len(&storage)?, 0);

        assert!(append_store.pop(&mut storage).is_err());
        assert!(append_store.length.lock().unwrap().eq(&Some(0)));
        assert_eq!(append_store.get_len(&storage)?, 0);

        Ok(())
    }

    #[test]
    fn test_iterator() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let append_store: AppendStore<i32> = AppendStore::new(b"test");
        append_store.push(&mut storage, &1234)?;
        append_store.push(&mut storage, &2143)?;
        append_store.push(&mut storage, &3412)?;
        append_store.push(&mut storage, &4321)?;

        // iterate twice to make sure nothing changed
        let mut iter = append_store.iter(&storage)?;
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        let mut iter = append_store.iter(&storage)?;
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth` doesn't break anything
        let mut iter = append_store.iter(&storage)?.skip(2);
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_reverse_iterator() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let append_store: AppendStore<i32> = AppendStore::new(b"test");
        append_store.push(&mut storage, &1234)?;
        append_store.push(&mut storage, &2143)?;
        append_store.push(&mut storage, &3412)?;
        append_store.push(&mut storage, &4321)?;

        let mut iter = append_store.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // iterate twice to make sure nothing changed
        let mut iter = append_store.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth_back` doesn't break anything
        let mut iter = append_store.iter(&storage)?.rev().skip(2);
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `ExactSizeIterator` works well
        let mut iter = append_store.iter(&storage)?.skip(2).rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_json_push_pop() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let append_store: AppendStore<i32, Json> = AppendStore::new(b"test");
        append_store.push(&mut storage, &1234)?;
        append_store.push(&mut storage, &2143)?;
        append_store.push(&mut storage, &3412)?;
        append_store.push(&mut storage, &4321)?;

        assert_eq!(append_store.pop(&mut storage), Ok(4321));
        assert_eq!(append_store.pop(&mut storage), Ok(3412));
        assert_eq!(append_store.pop(&mut storage), Ok(2143));
        assert_eq!(append_store.pop(&mut storage), Ok(1234));
        assert!(append_store.pop(&mut storage).is_err());

        Ok(())
    }

    #[test]
    fn test_suffixed_pop() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let suffix: &[u8] = b"test_suffix";
        let original_store: AppendStore<i32> = AppendStore::new(b"test");
        let append_store = original_store.add_suffix(suffix);
        append_store.push(&mut storage, &1234)?;
        append_store.push(&mut storage, &2143)?;
        append_store.push(&mut storage, &3412)?;
        append_store.push(&mut storage, &4321)?;

        assert_eq!(append_store.pop(&mut storage), Ok(4321));
        assert_eq!(append_store.pop(&mut storage), Ok(3412));
        assert_eq!(append_store.pop(&mut storage), Ok(2143));
        assert_eq!(append_store.pop(&mut storage), Ok(1234));
        assert!(append_store.pop(&mut storage).is_err());

        Ok(())
    }

    #[test]
    fn test_suffixed_reverse_iter() -> StdResult<()> {
        test_suffixed_reverse_iter_with_size(1)?;
        test_suffixed_reverse_iter_with_size(3)?;
        test_suffixed_reverse_iter_with_size(5)?;
        Ok(())
    }

    fn test_suffixed_reverse_iter_with_size(page_size: u32) -> StdResult<()> {
        let mut storage = MockStorage::new();
        let suffix: &[u8] = b"test_suffix";
        let original_store: AppendStore<i32> = AppendStore::new_with_page_size(b"test", page_size);
        let append_store = original_store.add_suffix(suffix);

        append_store.push(&mut storage, &1234)?;
        append_store.push(&mut storage, &2143)?;
        append_store.push(&mut storage, &3412)?;
        append_store.push(&mut storage, &4321)?;

        assert_eq!(original_store.get_len(&storage)?, 0);

        let mut iter = append_store.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // iterate twice to make sure nothing changed
        let mut iter = append_store.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth_back` doesn't break anything
        let mut iter = append_store.iter(&storage)?.rev().skip(2);
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `ExactSizeIterator` works well
        let mut iter = append_store.iter(&storage)?.skip(2).rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_suffix_iter() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let suffix: &[u8] = b"test_suffix";
        let original_store: AppendStore<i32> = AppendStore::new(b"test");
        let append_store = original_store.add_suffix(suffix);

        append_store.push(&mut storage, &1234)?;
        append_store.push(&mut storage, &2143)?;
        append_store.push(&mut storage, &3412)?;
        append_store.push(&mut storage, &4321)?;

        // iterate twice to make sure nothing changed
        let mut iter = append_store.iter(&storage)?;
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        let mut iter = append_store.iter(&storage)?;
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth` doesn't break anything
        let mut iter = append_store.iter(&storage)?.skip(2);
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_serializations() -> StdResult<()> {
        test_serializations_with_page_size(1)?;
        test_serializations_with_page_size(3)?;
        test_serializations_with_page_size(5)?;
        Ok(())
    }

    fn test_serializations_with_page_size(page_size: u32) -> StdResult<()> {
        // Check the default behavior is Bincode2
        let mut storage = MockStorage::new();

        let append_store: AppendStore<i32> = AppendStore::new_with_page_size(b"test", page_size);
        append_store.push(&mut storage, &1234)?;

        let key = [append_store.as_slice(), INDEXES, &0_u32.to_be_bytes()].concat();
        if append_store.page_size == 1 {
            let item_data = storage.get(&key);
            let expected_data = Bincode2::serialize(&1234)?;
            assert_eq!(item_data, Some(expected_data));
        } else {
            let bytes = storage.get(&key);
            let expected = Bincode2::serialize(&vec![Bincode2::serialize(&1234)?])?;
            assert_eq!(bytes, Some(expected));
        }

        // Check that overriding the serializer with Json works
        let mut storage = MockStorage::new();
        let json_append_store: AppendStore<i32, Json> =
            AppendStore::new_with_page_size(b"test2", page_size);
        json_append_store.push(&mut storage, &1234)?;

        let key = [json_append_store.as_slice(), INDEXES, &0_u32.to_be_bytes()].concat();
        if json_append_store.page_size == 1 {
            let item_data = storage.get(&key);
            let expected_data = b"1234".to_vec();
            assert_eq!(item_data, Some(expected_data));
        } else {
            let bytes = storage.get(&key);
            let expected = Bincode2::serialize(&vec![b"1234".to_vec()])?;
            assert_eq!(bytes, Some(expected));
        }

        Ok(())
    }

    #[test]
    fn test_removes() -> StdResult<()> {
        test_removes_with_size(1)?;
        test_removes_with_size(2)?;
        test_removes_with_size(7)?;
        test_removes_with_size(8)?;
        test_removes_with_size(13)?;

        Ok(())
    }

    fn test_removes_with_size(page_size: u32) -> StdResult<()> {
        let mut storage = MockStorage::new();
        let deque_store: AppendStore<i32> = AppendStore::new_with_page_size(b"test", page_size);
        deque_store.push(&mut storage, &1)?;
        deque_store.push(&mut storage, &2)?;
        deque_store.push(&mut storage, &3)?;
        deque_store.push(&mut storage, &4)?;
        deque_store.push(&mut storage, &5)?;
        deque_store.push(&mut storage, &6)?;
        deque_store.push(&mut storage, &7)?;
        deque_store.push(&mut storage, &8)?;

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
    fn test_paging() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let append_store: AppendStore<u32> = AppendStore::new(b"test");

        let page_size: u32 = 5;
        let total_items: u32 = 50;

        for i in 0..total_items {
            append_store.push(&mut storage, &i)?;
        }

        for i in 0..((total_items / page_size) - 1) {
            let start_page = i;

            let values = append_store.paging(&storage, start_page, page_size)?;

            for (index, value) in values.iter().enumerate() {
                assert_eq!(value, &(page_size * start_page + index as u32))
            }
        }

        Ok(())
    }

    #[test]
    fn test_paging_last_page() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let append_store: AppendStore<u32> = AppendStore::new(b"test");

        let total_items: u32 = 20;

        for i in 0..total_items {
            append_store.push(&mut storage, &i)?;
        }

        assert_eq!(append_store.paging(&storage, 0, 23)?.len(), 20);
        assert_eq!(append_store.paging(&storage, 2, 8)?.len(), 4);
        assert_eq!(append_store.paging(&storage, 2, 7)?.len(), 6);

        Ok(())
    }
}
