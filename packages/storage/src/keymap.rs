use std::{convert::TryInto};
use std::marker::PhantomData;

use serde::Deserialize;
use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{ReadonlyStorage, StdError, StdResult, Storage};

use secret_toolkit_serialization::{Serde, Bincode2};

use crate::prefixed_typed_storage::PrefixedTypedStorage;

const INDEXES: &[u8] = b"indexes";
const MAP_LENGTH: &[u8] = b"length";

const PAGE_SIZE: u32 = 5;

fn _page_from_position(position: u32) -> u32 {
    position / PAGE_SIZE
}

pub trait Key: Serialize + DeserializeOwned {}
impl<T> Key for T where T: Serialize + DeserializeOwned {}

#[derive(Serialize, Deserialize)]
struct InternalItem<T>
// where
//     T: Serialize + DeserializeOwned,
{
    item: T,
    index_pos: u32,
}

pub struct Keymap<'a, K, T, Ser = Bincode2>
    where
        K: Key,
        T: Serialize + DeserializeOwned,
        Ser: Serde,
{
    /// prefix of the newly constructed Storage
    namespace: &'a [u8],
    /// needed if any suffixes were added to the original namespace.
    /// therefore it is not necessarily same as the namespace.
    prefix: Option<Vec<u8>>,
    key_type: PhantomData<K>,
    item_type: PhantomData<T>,
    serialization_type: PhantomData<Ser>,
}

impl<'a, K: Key, T: Serialize + DeserializeOwned, Ser: Serde> Keymap<'a, K, T, Ser>{
    /// constructor
    pub const fn new(prefix: &'a [u8]) -> Self {
        Self {
            namespace: prefix,
            prefix: None,
            key_type: PhantomData,
            item_type: PhantomData,
            serialization_type: PhantomData,
        }
    }
    /// This is used to produce a new Keymap. This can be used when you want to associate an Keymap to each user
    /// and you still get to define the Keymap as a static constant
    pub fn add_suffix(&self, suffix: &[u8]) -> Self {
        let prefix = if let Some(prefix) = self.prefix.clone() {
            [prefix, suffix.to_vec()].concat()
        } else {
            [self.namespace.to_vec(), suffix.to_vec()].concat()
        };
        Self {
            namespace: self.namespace,
            prefix: Some(prefix),
            key_type: self.key_type,
            item_type: self.item_type,
            serialization_type: self.serialization_type,
        }
    }
}

impl<'a, K: Key, T: Serialize + DeserializeOwned, Ser: Serde> Keymap<'a, K, T, Ser> {
    /// Serialize key
    fn serialize_key(&self, key: &K) -> StdResult<Vec<u8>> {
        Ser::serialize(key)
    }
    /// Deserialize key
    fn deserialize_key(&self, key_data: &[u8]) -> StdResult<K> {
        Ser::deserialize(key_data)
    }
    /// get total number of objects saved
    pub fn get_len<S: ReadonlyStorage>(&self, storage: &S) -> StdResult<u32> {
        let len_key = [self.as_slice(), MAP_LENGTH].concat();
        if let Some(len_vec) = storage.get(&len_key) {
            let len_bytes = len_vec.as_slice().try_into().map_err(|err| StdError::parse_err("u32", err))?;
            let len = u32::from_be_bytes(len_bytes);
            Ok(len)
        } else {
            Ok(0)
        }
    }
    /// checks if the collection has any elements
    pub fn is_empty<S: ReadonlyStorage>(&self, storage: &S) -> StdResult<bool> {
        Ok(self.get_len(storage)? == 0)
    }
    /// set length of the map
    fn set_len<S: Storage>(&self, storage: &mut S, len: u32) -> StdResult<()> {
        let len_key = [self.as_slice(), MAP_LENGTH].concat();
        storage.set(&len_key, &len.to_be_bytes());
        Ok(())
    }
    /// Used to get the indexes stored in the given page number
    fn _get_indexes<S: ReadonlyStorage>(&self, storage: &S, page: u32) -> StdResult<Vec<Vec<u8>>> {
        let indexes_key = [INDEXES, page.to_be_bytes().as_slice()].concat();
        let maybe_serialized = storage.get(&indexes_key);
        match maybe_serialized {
            Some(serialized) => { Bincode2::deserialize(&serialized) },
            None => { Ok(vec![]) },
        }
    }
    /// Set an indexes page
    fn _set_indexes_page<S: Storage>(&self, storage: &mut S, page: u32, indexes: &Vec<Vec<u8>>) -> StdResult<()> {
        let indexes_key = [INDEXES, page.to_be_bytes().as_slice()].concat();
        storage.set(&indexes_key, &Bincode2::serialize(indexes)?);
        Ok(())
    }
    /// user facing get function
    pub fn get<S: ReadonlyStorage>(&self, storage: &S, key: &K) -> Option<T> {
        if let Ok(internal_item) = self._get_from_key(storage, key) {
            Some(internal_item.item)
        } else {
            None
        }
    }
    /// internal item get function
    fn _get_from_key<S: ReadonlyStorage>(&self, storage: &S, key: &K) -> StdResult<InternalItem<T>> {
        let key_vec = self.serialize_key(key)?;
        self.load_impl(storage, &key_vec)
    }
    /// user facing remove function
    pub fn remove<S: Storage>(&self, storage: &mut S, key: &K) -> StdResult<()> {
        let key_vec = self.serialize_key(key)?;
        let removed_pos = self._get_from_key(storage, key)?.index_pos;

        self.remove_impl(storage, &key_vec);
        let page = _page_from_position(removed_pos);

        let mut len = self.get_len(storage)?;
        len -= 1;
        self.set_len(storage, len)?;

        let mut indexes = self._get_indexes(storage, page)?;

        let pos_in_indexes = (removed_pos % PAGE_SIZE) as usize;

        if indexes[pos_in_indexes] != key_vec {
            return Err(StdError::generic_err("Tried to remove, but hash not found - should never happen"));
        }

        // if our object is the last item, then just remove it
        if len == 0 || len == removed_pos {
            indexes.pop();
            self._set_indexes_page(storage, page, &indexes)?;
            return Ok(());
        }

        // max page should use previous_len - 1 which is exactly the current len
        let max_page = _page_from_position(len);
        if max_page == page { // last page indexes is the same as indexes
            let last_key = indexes.pop().ok_or(StdError::generic_err("Last item's key not found - should never happen"))?;
            // modify last item
            let mut last_internal_item = self.load_impl(storage, &last_key)?;
            last_internal_item.index_pos = removed_pos;
            self.save_impl(storage, &last_key, &last_internal_item)?;
            // save to indexes
            indexes[pos_in_indexes] = last_key;
            self._set_indexes_page(storage, page, &indexes)?;
        } else {
            let mut last_page_indexes = self._get_indexes(storage, max_page)?;
            let last_key = last_page_indexes.pop().ok_or(StdError::generic_err("Last item's key not found - should never happen"))?;
            // modify last item
            let mut last_internal_item = self.load_impl(storage, &last_key)?;
            last_internal_item.index_pos = removed_pos;
            self.save_impl(storage, &last_key, &last_internal_item)?;
            // save indexes
            indexes[pos_in_indexes] = last_key;
            self._set_indexes_page(storage, page, &indexes)?;
            self._set_indexes_page(storage, max_page, &last_page_indexes)?;
        }

        Ok(())
    }
    /// user facing insert function
    pub fn insert<S: Storage>(&self, storage: &mut S, key: &K, item: T) -> StdResult<()> {
        let key_vec = self.serialize_key(key)?;
        match self.may_load_impl(storage, &key_vec)? {
            Some(mut existing_internal_item) => { // if item already exists
                existing_internal_item.item = item;
                self.save_impl(storage, &key_vec, &existing_internal_item)
            },
            None => { // not already saved
                let pos = self.get_len(storage)?;
                self.set_len(storage, pos + 1)?;
                let page = _page_from_position(pos);
                // save the item
                let internal_item = InternalItem {
                    item,
                    index_pos: pos,
                };
                self.save_impl(storage, &key_vec, &internal_item)?;
                // add index
                let mut indexes = self._get_indexes(storage, page)?;
                indexes.push(key_vec);
                self._set_indexes_page(storage, page, &indexes)
            },
        }
    }
    /// user facing method that checks if any item is stored with this key.
    pub fn contains<S: ReadonlyStorage>(&self, storage: &S, key: &K) -> bool {
        match self._get_from_key(storage, key) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    /// paginates (key, item) pairs.
    pub fn paging<S: ReadonlyStorage>(&self, storage: &S, start_page: u32, size: u32) -> StdResult<Vec<(K, T)>> {
        let start_pos = start_page * size;
        let mut end_pos = start_pos + size - 1;

        let max_size = self.get_len(storage)?;

        if max_size == 0 {
            return Ok(vec![]);
        }

        if start_pos > max_size {
            return Err(StdError::NotFound {
                kind: "Out of bounds".to_string(),
                backtrace: None,
            });
        } else if end_pos > max_size {
            end_pos = max_size - 1;
        }
        self.get_pairs_at_positions(storage, start_pos, end_pos)
    }
    /// paginates only the keys. More efficient than paginating both items and keys
    pub fn paging_keys<S: ReadonlyStorage>(&self, storage: &S, start_page: u32, size: u32) -> StdResult<Vec<K>> {
        let start_pos = start_page * size;
        let mut end_pos = start_pos + size - 1;

        let max_size = self.get_len(storage)?;

        if max_size == 0 {
            return Ok(vec![]);
        }

        if start_pos > max_size {
            return Err(StdError::NotFound {
                kind: "Out of bounds".to_string(),
                backtrace: None,
            });
        } else if end_pos > max_size {
            end_pos = max_size - 1;
        }
        self.get_keys_at_positions(storage, start_pos, end_pos)
    }
    /// tries to list keys without checking start/end bounds
    fn get_keys_at_positions<S: ReadonlyStorage>(&self, storage: &S, start: u32, end: u32) -> StdResult<Vec<K>> {
        let start_page = _page_from_position(start);
        let end_page = _page_from_position(end);

        let mut res = vec![];

        for page in start_page..=end_page {
            let indexes = self._get_indexes(storage, page)?;
            let start_page_pos = if page == start_page {
                start % PAGE_SIZE
            } else {
                0
            };
            let end_page_pos = if page == end_page {
                end % PAGE_SIZE
            } else {
                PAGE_SIZE - 1
            };
            for i in start_page_pos..=end_page_pos {
                let key_vec = &indexes[i as usize];
                let key = self.deserialize_key(key_vec)?;
                res.push(key);
            }
        }
        Ok(res)
    }
    /// tries to list (key, item) pairs without checking start/end bounds
    fn get_pairs_at_positions<S: ReadonlyStorage>(&self, storage: &S, start: u32, end: u32) -> StdResult<Vec<(K, T)>> {
        let start_page = _page_from_position(start);
        let end_page = _page_from_position(end);

        let mut res = vec![];

        for page in start_page..=end_page {
            let indexes = self._get_indexes(storage, page)?;
            let start_page_pos = if page == start_page {
                start % PAGE_SIZE
            } else {
                0
            };
            let end_page_pos = if page == end_page {
                end % PAGE_SIZE
            } else {
                PAGE_SIZE - 1
            };
            for i in start_page_pos..=end_page_pos {
                let key_vec = &indexes[i as usize];
                let key = self.deserialize_key(key_vec)?;
                let item = self.load_impl(storage, key_vec)?.item;
                res.push((key, item));
            }
        }
        Ok(res)
    }
    /// gets a key from a specific position in indexes
    fn get_key_from_pos<S: ReadonlyStorage>(&self, storage: &S, pos: u32) -> StdResult<K> {
        let page = _page_from_position(pos);
        let indexes = self._get_indexes(storage, page)?;
        let index = pos % PAGE_SIZE;
        let key_vec = &indexes[index as usize];
        self.deserialize_key(key_vec)
    }
    /// gets a key from a specific position in indexes
    fn get_pair_from_pos<S: ReadonlyStorage>(&self, storage: &S, pos: u32) -> StdResult<(K, T)> {
        let page = _page_from_position(pos);
        let indexes = self._get_indexes(storage, page)?;
        let index = pos % PAGE_SIZE;
        let key_vec = &indexes[index as usize];
        let key = self.deserialize_key(key_vec)?;
        let item = self.load_impl(storage, key_vec)?.item;
        Ok((key, item))
    }
    /// Returns a readonly iterator only for keys. More efficient than iter().
    pub fn iter_keys<S: ReadonlyStorage>(&self, storage: &'a S) -> StdResult<KeyIter<K, T, S, Ser>> {
        let len = self.get_len(storage)?;
        let iter = KeyIter::new(self, storage, 0, len);
        Ok(iter)
    }
    /// Returns a readonly iterator for (key-item) pairs
    pub fn iter<S: ReadonlyStorage>(&self, storage: &'a S) -> StdResult<KeyItemIter<K, T, S, Ser>> {
        let len = self.get_len(storage)?;
        let iter = KeyItemIter::new(self, storage, 0, len);
        Ok(iter)
    }
}

impl<'a, K: Key, T: Serialize + DeserializeOwned, Ser: Serde> PrefixedTypedStorage<InternalItem<T>, Ser> for Keymap<'a, K, T, Ser> {
    fn as_slice(&self) -> &[u8] {
        if let Some(prefix) = &self.prefix {
            prefix
        } else {
            self.namespace
        }
    }
}

/// An iterator over the keys of the Keymap.
pub struct KeyIter<'a, K, T, S, Ser>
where
    K: Key,
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{
    keymap: &'a Keymap<'a, K, T, Ser>,
    storage: &'a S,
    start: u32,
    end: u32,
}

impl<'a, K, T, S, Ser> KeyIter<'a, K, T, S, Ser>
    where
        K: Key,
        T: Serialize + DeserializeOwned,
        S: ReadonlyStorage,
        Ser: Serde,
{
    /// constructor
    pub fn new(
        keymap: &'a Keymap<'a, K, T, Ser>,
        storage: &'a S,
        start: u32,
        end: u32
    ) -> Self {
        Self {
            keymap,
            storage,
            start,
            end,
        }
    }
}

impl<'a, K, T, S, Ser> Iterator for KeyIter<'a, K, T, S, Ser>
    where
        K: Key,
        T: Serialize + DeserializeOwned,
        S: ReadonlyStorage,
        Ser: Serde,
{
    type Item = StdResult<K>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        let res: Option<Self::Item>;
        match self.keymap.get_key_from_pos(self.storage, self.start) {
            Ok(key) => { res = Some(Ok(key));},
            Err(_) => { res = None; },
        }
        self.start += 1;
        res
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

impl<'a, K, T, S, Ser> DoubleEndedIterator for KeyIter<'a, K, T, S, Ser>
    where
        K: Key,
        T: Serialize + DeserializeOwned,
        S: ReadonlyStorage,
        Ser: Serde,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        self.end -= 1;
        match self.keymap.get_key_from_pos(self.storage, self.end) {
            Ok(key) => Some(Ok(key)),
            Err(_) => None,
        }
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
impl<'a, K, T, S, Ser> ExactSizeIterator for KeyIter<'a, K, T, S, Ser>
where
    K: Key,
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{}

// ===============================================================================================

/// An iterator over the (key, item) pairs of the Keymap. Less efficient than just iterating over keys.
pub struct KeyItemIter<'a, K, T, S, Ser>
where
    K: Key,
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{
    keymap: &'a Keymap<'a, K, T, Ser>,
    storage: &'a S,
    start: u32,
    end: u32,
}

impl<'a, K, T, S, Ser> KeyItemIter<'a, K, T, S, Ser>
    where
        K: Key,
        T: Serialize + DeserializeOwned,
        S: ReadonlyStorage,
        Ser: Serde,
{
    /// constructor
    pub fn new(
        keymap: &'a Keymap<'a, K, T, Ser>,
        storage: &'a S,
        start: u32,
        end: u32
    ) -> Self {
        Self {
            keymap,
            storage,
            start,
            end,
        }
    }
}

impl<'a, K, T, S, Ser> Iterator for KeyItemIter<'a, K, T, S, Ser>
    where
        K: Key,
        T: Serialize + DeserializeOwned,
        S: ReadonlyStorage,
        Ser: Serde,
{
    type Item = StdResult<(K, T)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        let res: Option<Self::Item>;
        match self.keymap.get_pair_from_pos(self.storage, self.start) {
            Ok(pair) => { res = Some(Ok(pair)); },
            Err(_) => { res = None; },
        }
        self.start += 1;
        res
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

impl<'a, K, T, S, Ser> DoubleEndedIterator for KeyItemIter<'a, K, T, S, Ser>
    where
        K: Key,
        T: Serialize + DeserializeOwned,
        S: ReadonlyStorage,
        Ser: Serde,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        self.end -= 1;
        match self.keymap.get_pair_from_pos(self.storage, self.end) {
            Ok(pair) => Some(Ok(pair)),
            Err(_) => None,
        }
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
impl<'a, K, T, S, Ser> ExactSizeIterator for KeyItemIter<'a, K, T, S, Ser>
where
    K: Key,
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{}


#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use cosmwasm_std::testing::MockStorage;

    use super::*;

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
    struct Foo {
        string: String,
        number: i32,
    }
    #[test]
    fn test_keymap_perf_insert() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let total_items = 1000;

        let keymap: Keymap<Vec<u8>, i32> = Keymap::new(b"test");

        for i in 0..total_items {
            let key: Vec<u8> = (i as i32).to_be_bytes().to_vec();
            keymap.insert(&mut storage, &key, i)?;
        }

        assert_eq!(keymap.get_len(&storage)?, 1000);

        Ok(())
    }

    #[test]
    fn test_keymap_perf_insert_remove() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let total_items = 100;

        let keymap: Keymap<i32, i32> = Keymap::new(b"test");

        for i in 0..total_items {
            keymap.insert(&mut storage, &i, i)?;
        }

        for i in 0..total_items {
            keymap.remove(&mut storage, &i)?;
        }

        assert_eq!(keymap.get_len(&storage)?, 0);

        Ok(())
    }
    
    #[test]
    fn test_keymap_paging() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let page_size: u32 = 5;
        let total_items: u32 = 50;
        let keymap: Keymap<Vec<u8>, u32> = Keymap::new(b"test");

        for i in 0..total_items {
            let key: Vec<u8> = (i as i32).to_be_bytes().to_vec();
            keymap.insert(&mut storage, &key, i)?;
        }

        for i in 0..((total_items / page_size) - 1) {
            let start_page = i;

            let values = keymap.paging(&storage, start_page, page_size)?;

            for (index, (key_value, value)) in values.iter().enumerate() {
                let i = page_size * start_page + index as u32;
                let key: Vec<u8> = (i as i32).to_be_bytes().to_vec();
                assert_eq!(key_value, &key);
                assert_eq!(value, &i);
            }
        }

        Ok(())
    }
    
    #[test]
    fn test_keymap_paging_overflow() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let page_size = 50;
        let total_items = 10;
        let keymap: Keymap<i32, u32> = Keymap::new(b"test");

        for i in 0..total_items {
            keymap.insert(&mut storage, &(i as i32), i)?;
        }

        let values = keymap.paging_keys(&storage, 0, page_size)?;

        assert_eq!(values.len(), total_items as usize);

        for (index, value) in values.iter().enumerate() {
            assert_eq!(value, &(index as i32))
        }

        Ok(())
    }
    
    #[test]
    fn test_keymap_insert_multiple() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keymap: Keymap<Vec<u8>, Foo> = Keymap::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 1111,
        };

        keymap.insert(&mut storage, &b"key1".to_vec(), foo1.clone())?;
        keymap.insert(&mut storage, &b"key2".to_vec(), foo2.clone())?;

        let read_foo1 = keymap.get(&storage, &b"key1".to_vec()).unwrap();
        let read_foo2 = keymap.get(&storage, &b"key2".to_vec()).unwrap();

        assert_eq!(foo1, read_foo1);
        assert_eq!(foo2, read_foo2);
        Ok(())
    }
    
    #[test]
    fn test_keymap_contains() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keymap: Keymap<Vec<u8>, Foo> = Keymap::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };

        keymap.insert(&mut storage, &b"key1".to_vec(), foo1.clone())?;
        let contains_k1 = keymap.contains(&storage, &b"key1".to_vec());

        assert_eq!(contains_k1, true);

        Ok(())
    }

    
    #[test]
    fn test_keymap_iter() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keymap: Keymap<Vec<u8>, Foo> = Keymap::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 1111,
        };

        keymap.insert(&mut storage, &b"key1".to_vec(), foo1.clone())?;
        keymap.insert(&mut storage, &b"key2".to_vec(), foo2.clone())?;

        let mut x = keymap.iter(&storage)?;
        let (len, _) = x.size_hint();
        assert_eq!(len, 2);

        assert_eq!(x.next().unwrap()?, (b"key1".to_vec(), foo1));

        assert_eq!(x.next().unwrap()?, (b"key2".to_vec(), foo2));

        Ok(())
    }

    #[test]
    fn test_keymap_iter_keys() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keymap: Keymap<String, Foo> = Keymap::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 1111,
        };

        let key1 = "key1".to_string();
        let key2 = "key2".to_string();

        keymap.insert(&mut storage, &key1, foo1.clone())?;
        keymap.insert(&mut storage, &key2, foo2.clone())?;

        let mut x = keymap.iter_keys(&storage)?;
        let (len, _) = x.size_hint();
        assert_eq!(len, 2);

        assert_eq!(x.next().unwrap()?, key1);

        assert_eq!(x.next().unwrap()?, key2);

        Ok(())
    }
    
    #[test]
    fn test_keymap_overwrite() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keymap: Keymap<Vec<u8>, Foo> = Keymap::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 2222,
        };

        keymap.insert(&mut storage, &b"key1".to_vec(), foo1.clone())?;
        keymap.insert(&mut storage, &b"key1".to_vec(), foo2.clone())?;

        let foo3 = keymap.get(&storage, &b"key1".to_vec()).unwrap();

        assert_eq!(foo3, foo2);

        Ok(())
    }

    #[test]
    fn test_keymap_suffixed_basics() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let original_keymap: Keymap<String, Foo> = Keymap::new(b"test");
        let keymap = original_keymap.add_suffix(b"test_suffix");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        keymap.insert(&mut storage, &"key1".to_string(), foo1.clone())?;
        keymap.insert(&mut storage, &"key2".to_string(), foo2.clone())?;

        let read_foo1 = keymap.get(&storage, &"key1".to_string()).unwrap();
        let read_foo2 = keymap.get(&storage, &"key2".to_string()).unwrap();

        assert_eq!(original_keymap.get_len(&storage)?, 0);
        assert_eq!(foo1, read_foo1);
        assert_eq!(foo2, read_foo2);

        let alternative_keymap: Keymap<String, Foo> = Keymap::new(b"alternative");
        let alt_same_suffix = alternative_keymap.add_suffix(b"test_suffix");

        assert!(alt_same_suffix.is_empty(&storage)?);

        // show that it loads foo1 before removal
        let before_remove_foo1 = keymap.get(&storage, &"key1".to_string());
        assert!(before_remove_foo1.is_some());
        assert_eq!(foo1, before_remove_foo1.unwrap());
        // and returns None after removal
        keymap.remove(&mut storage, &"key1".to_string())?;
        let removed_foo1 = keymap.get(&storage, &"key1".to_string());
        assert!(removed_foo1.is_none());

        // show what happens when reading from keys that have not been set yet.
        assert!(keymap.get(&storage, &"key3".to_string()).is_none());

        Ok(())
    }

    #[test]
    fn test_keymap_length() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keymap: Keymap<String, Foo> = Keymap::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };

        assert_eq!(keymap.get_len(&storage)?, 0);

        let key1 = "k1".to_string();
        let key2 = "k2".to_string();

        keymap.insert(&mut storage, &key1, foo1.clone())?;
        assert_eq!(keymap.get_len(&storage)?, 1);

        // add another item
        keymap.insert(&mut storage, &key2, foo2.clone())?;
        assert_eq!(keymap.get_len(&storage)?, 2);

        // remove item and check length
        keymap.remove(&mut storage, &key1)?;
        assert_eq!(keymap.get_len(&storage)?, 1);

        // override item (should not change length)
        keymap.insert(&mut storage, &key2, foo1)?;
        assert_eq!(keymap.get_len(&storage)?, 1);

        // remove item and check length
        keymap.remove(&mut storage, &key2)?;
        assert_eq!(keymap.get_len(&storage)?, 0);

        Ok(())
    }
}
