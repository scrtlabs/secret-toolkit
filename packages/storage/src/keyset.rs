use std::marker::PhantomData;
use std::sync::Mutex;
use std::{collections::HashMap, convert::TryInto};

use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{StdError, StdResult, Storage};
use cosmwasm_storage::to_length_prefixed;

use secret_toolkit_serialization::{Bincode2, Serde};

use crate::{IterOption, WithIter, WithoutIter};

const INDEXES: &[u8] = b"indexes";
const MAP_LENGTH: &[u8] = b"length";

const DEFAULT_PAGE_SIZE: u32 = 5;

//pub struct WithIter;
//pub struct WithoutIter;
//pub trait IterOption {}

//impl IterOption for WithIter {}
//impl IterOption for WithoutIter {}

pub struct KeysetBuilder<'a, K, Ser = Bincode2, I = WithIter> {
    /// prefix of the newly constructed Storage
    namespace: &'a [u8],
    page_size: u32,
    key_type: PhantomData<K>,
    serialization_type: PhantomData<Ser>,
    iter_option: PhantomData<I>,
}

impl<'a, K, Ser> KeysetBuilder<'a, K, Ser, WithIter>
where
    K: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// Creates a KeysetBuilder with default features
    pub const fn new(namespace: &'a [u8]) -> Self {
        Self {
            namespace,
            page_size: DEFAULT_PAGE_SIZE,
            key_type: PhantomData,
            serialization_type: PhantomData,
            iter_option: PhantomData,
        }
    }
    /// Modifies the number of values stored in one page of indexing, for the iterator
    pub const fn with_page_size(&self, indexes_size: u32) -> Self {
        if indexes_size == 0 {
            panic!("zero index page size used in keyset")
        }
        Self {
            namespace: self.namespace,
            page_size: indexes_size,
            key_type: self.key_type,
            serialization_type: self.serialization_type,
            iter_option: self.iter_option,
        }
    }
    /// Disables the iterator of the keyset, saving at least 4000 gas in each insertion.
    pub const fn without_iter(&self) -> KeysetBuilder<'a, K, Ser, WithoutIter> {
        KeysetBuilder {
            namespace: self.namespace,
            page_size: self.page_size,
            key_type: PhantomData,
            serialization_type: PhantomData,
            iter_option: PhantomData,
        }
    }
    /// Returns a keyset with the given configuration
    pub const fn build(&self) -> Keyset<'a, K, Ser, WithIter> {
        Keyset {
            namespace: self.namespace,
            prefix: None,
            page_size: self.page_size,
            length: Mutex::new(None),
            key_type: self.key_type,
            iter_option: self.iter_option,
            serialization_type: self.serialization_type,
        }
    }
}

// This enables writing `append_store.iter().skip(n).rev()`
impl<'a, K, Ser> KeysetBuilder<'a, K, Ser, WithoutIter>
where
    K: Serialize + DeserializeOwned,
    Ser: Serde,
{
    pub const fn build(&self) -> Keyset<'a, K, Ser, WithoutIter> {
        Keyset {
            namespace: self.namespace,
            prefix: None,
            page_size: self.page_size,
            length: Mutex::new(None),
            key_type: self.key_type,
            iter_option: self.iter_option,
            serialization_type: self.serialization_type,
        }
    }
}

pub struct Keyset<'a, K, Ser = Bincode2, I = WithIter>
where
    K: Serialize + DeserializeOwned,
    Ser: Serde,
    I: IterOption,
{
    /// prefix of the newly constructed Storage
    namespace: &'a [u8],
    /// needed if any suffixes were added to the original namespace.
    prefix: Option<Vec<u8>>,
    page_size: u32,
    length: Mutex<Option<u32>>,
    key_type: PhantomData<K>,
    iter_option: PhantomData<I>,
    serialization_type: PhantomData<Ser>,
}

impl<'a, K: Serialize + DeserializeOwned, Ser: Serde> Keyset<'a, K, Ser> {
    /// constructor
    pub const fn new(prefix: &'a [u8]) -> Self {
        Self {
            namespace: prefix,
            prefix: None,
            page_size: DEFAULT_PAGE_SIZE,
            length: Mutex::new(None),
            key_type: PhantomData,
            serialization_type: PhantomData,
            iter_option: PhantomData,
        }
    }

    /// This is used to produce a new Keyset. This can be used when you want to associate an Keyset to each user
    /// and you still get to define the Keyset as a static constant
    pub fn add_suffix(&self, suffix: &[u8]) -> Self {
        let suffix = to_length_prefixed(suffix);
        let prefix = self.prefix.as_deref().unwrap_or(self.namespace);
        let prefix = [prefix, suffix.as_slice()].concat();
        Self {
            namespace: self.namespace,
            prefix: Some(prefix),
            page_size: self.page_size,
            length: Mutex::new(None),
            key_type: self.key_type,
            serialization_type: self.serialization_type,
            iter_option: self.iter_option,
        }
    }
}

impl<'a, K: Serialize + DeserializeOwned, Ser: Serde> Keyset<'a, K, Ser, WithoutIter> {
    fn as_slice(&self) -> &[u8] {
        if let Some(prefix) = &self.prefix {
            prefix
        } else {
            self.namespace
        }
    }
    /// returns the actual storage key
    fn storage_key(&self, key: &K) -> StdResult<Vec<u8>> {
        let prefix = self.as_slice();
        let key_vec = self.serialize_key(key)?;
        Ok([prefix, key_vec.as_slice()].concat())
    }
    /// Serialize key
    fn serialize_key(&self, key: &K) -> StdResult<Vec<u8>> {
        Ser::serialize(key)
    }

    /// user facing remove function
    pub fn remove(&self, storage: &mut dyn Storage, value: &K) -> StdResult<()> {
        let key_vec = self.storage_key(value)?;
        storage.remove(&key_vec);
        Ok(())
    }

    /// user facing insert function
    pub fn insert(&self, storage: &mut dyn Storage, value: &K) -> StdResult<()> {
        let key_vec = self.storage_key(value)?;
        storage.set(&key_vec, &[0]);
        Ok(())
    }

    /// user facing method that checks if this value is stored.
    pub fn contains(&self, storage: &dyn Storage, value: &K) -> bool {
        match self.storage_key(value) {
            Ok(key_vec) => storage.get(&key_vec).is_some(),
            Err(_) => false,
        }
    }
}

impl<'a, K: Serialize + DeserializeOwned, Ser: Serde> Keyset<'a, K, Ser, WithIter> {
    fn as_slice(&self) -> &[u8] {
        if let Some(prefix) = &self.prefix {
            prefix
        } else {
            self.namespace
        }
    }
    /// returns the actual storage key
    fn storage_key(&self, key: &K) -> StdResult<Vec<u8>> {
        let prefix = self.as_slice();
        let key_vec = self.serialize_key(key)?;
        Ok([prefix, key_vec.as_slice()].concat())
    }
    /// Serialize key
    fn serialize_key(&self, key: &K) -> StdResult<Vec<u8>> {
        Ser::serialize(key)
    }

    /// Deserialize key
    fn deserialize_key(&self, key_data: &[u8]) -> StdResult<K> {
        Ser::deserialize(key_data)
    }

    fn page_from_position(&self, position: u32) -> u32 {
        position / self.page_size
    }

    /// get total number of objects saved
    pub fn get_len(&self, storage: &dyn Storage) -> StdResult<u32> {
        let mut may_len = self.length.lock().unwrap();
        match *may_len {
            Some(length) => Ok(length),
            None => {
                let len_key = [self.as_slice(), MAP_LENGTH].concat();
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

    /// set length of the map
    fn set_len(&self, storage: &mut dyn Storage, len: u32) -> StdResult<()> {
        let len_key = [self.as_slice(), MAP_LENGTH].concat();
        storage.set(&len_key, &len.to_be_bytes());

        let mut may_len = self.length.lock().unwrap();
        *may_len = Some(len);

        Ok(())
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

    /// internal item get function
    fn get_pos(&self, storage: &dyn Storage, key_vec: &[u8]) -> StdResult<u32> {
        match storage.get(key_vec) {
            Some(data) => {
                let pos_bytes = data
                    .as_slice()
                    .try_into()
                    .map_err(|err| StdError::parse_err("u32", err))?;
                Ok(u32::from_be_bytes(pos_bytes))
            }
            None => Err(StdError::NotFound {
                kind: "keyset value not found.".to_string(),
            }),
        }
    }

    /// user facing remove function
    pub fn remove(&self, storage: &mut dyn Storage, value: &K) -> StdResult<()> {
        let prefix = self.as_slice();
        let key_data = self.serialize_key(value)?;
        let key_vec = [prefix, key_data.as_slice()].concat();

        let removed_pos = self.get_pos(storage, &key_vec)?;

        let page = self.page_from_position(removed_pos);

        let mut len = self.get_len(storage)?;
        len -= 1;
        self.set_len(storage, len)?;

        let mut indexes = self.get_indexes(storage, page)?;

        let pos_in_indexes = (removed_pos % self.page_size) as usize;

        if indexes[pos_in_indexes] != key_data {
            return Err(StdError::generic_err(
                "tried to remove from keyset, but value not found in indexes - should never happen",
            ));
        }

        // if our object is the last item, then just remove it
        if len == 0 || len == removed_pos {
            indexes.pop();
            self.set_indexes_page(storage, page, &indexes)?;
            return Ok(());
        }

        // max page should use previous_len - 1 which is exactly the current len
        let max_page = self.page_from_position(len);
        if max_page == page {
            // last page indexes is the same as indexes
            let last_data = indexes.pop().ok_or_else(|| {
                StdError::generic_err("last item's key not found - should never happen")
            })?;
            let last_key = [prefix, last_data.as_slice()].concat();
            // modify last item
            storage.set(&last_key, &removed_pos.to_be_bytes());
            // save to indexes
            indexes[pos_in_indexes] = last_data;
            self.set_indexes_page(storage, page, &indexes)?;
        } else {
            let mut last_page_indexes = self.get_indexes(storage, max_page)?;
            let last_data = last_page_indexes.pop().ok_or_else(|| {
                StdError::generic_err("last item's key not found - should never happen")
            })?;
            let last_key = [prefix, last_data.as_slice()].concat();
            // modify last item
            storage.set(&last_key, &removed_pos.to_be_bytes());
            // save indexes
            indexes[pos_in_indexes] = last_data;
            self.set_indexes_page(storage, page, &indexes)?;
            self.set_indexes_page(storage, max_page, &last_page_indexes)?;
        }

        storage.remove(&key_vec);

        Ok(())
    }

    /// user facing insert function
    ///
    /// returns `Ok(true)` if the set did not previously contain the value
    /// returns `Ok(false)` if the set already contained this value
    /// returns `Err` if the insertion fails due to an error
    pub fn insert(&self, storage: &mut dyn Storage, value: &K) -> StdResult<bool> {
        let prefix = self.as_slice();
        let key_data = self.serialize_key(value)?;
        let key_vec = [prefix, key_data.as_slice()].concat();

        match storage.get(&key_vec) {
            Some(_) => Ok(false),
            None => {
                // not already saved
                let pos = self.get_len(storage)?;
                self.set_len(storage, pos + 1)?;
                let page = self.page_from_position(pos);
                // save the item
                storage.set(&key_vec, &pos.to_be_bytes());
                // add index
                let mut indexes = self.get_indexes(storage, page)?;
                indexes.push(key_data);
                self.set_indexes_page(storage, page, &indexes)?;
                Ok(true)
            }
        }
    }

    /// user facing method that checks if this value is stored.
    pub fn contains(&self, storage: &dyn Storage, value: &K) -> bool {
        match self.storage_key(value) {
            Ok(key_vec) => storage.get(&key_vec).is_some(),
            Err(_) => false,
        }
    }

    /// paginates only the values.
    pub fn paging(&self, storage: &dyn Storage, start_page: u32, size: u32) -> StdResult<Vec<K>> {
        let start_pos = start_page * size;
        let mut end_pos = start_pos + size - 1;

        let max_size = self.get_len(storage)?;

        if max_size == 0 {
            return Ok(vec![]);
        }

        if start_pos > max_size {
            return Err(StdError::NotFound {
                kind: "out of bounds".to_string(),
            });
        } else if end_pos > max_size {
            end_pos = max_size - 1;
        }
        self.get_keys_at_positions(storage, start_pos, end_pos)
    }

    /// tries to list keys without checking start/end bounds
    fn get_keys_at_positions(
        &self,
        storage: &dyn Storage,
        start: u32,
        end: u32,
    ) -> StdResult<Vec<K>> {
        let start_page = self.page_from_position(start);
        let end_page = self.page_from_position(end);

        let mut res = vec![];

        for page in start_page..=end_page {
            let indexes = self.get_indexes(storage, page)?;
            let start_page_pos = if page == start_page {
                start % self.page_size
            } else {
                0
            };
            let end_page_pos = if page == end_page {
                end % self.page_size
            } else {
                self.page_size - 1
            };
            for i in start_page_pos..=end_page_pos {
                let key_vec = &indexes[i as usize];
                let key = self.deserialize_key(key_vec)?;
                res.push(key);
            }
        }
        Ok(res)
    }

    /// Returns a readonly iterator only for values.
    pub fn iter(&self, storage: &'a dyn Storage) -> StdResult<ValueIter<K, Ser>> {
        let len = self.get_len(storage)?;
        let iter = ValueIter::new(self, storage, 0, len);
        Ok(iter)
    }
}

/// An iterator over the keys of the Keyset.
pub struct ValueIter<'a, K, Ser>
where
    K: Serialize + DeserializeOwned,
    Ser: Serde,
{
    keyset: &'a Keyset<'a, K, Ser>,
    storage: &'a dyn Storage,
    start: u32,
    end: u32,
    cache: HashMap<u32, Vec<Vec<u8>>>,
}

impl<'a, K, Ser> ValueIter<'a, K, Ser>
where
    K: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// constructor
    pub fn new(
        keyset: &'a Keyset<'a, K, Ser>,
        storage: &'a dyn Storage,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            keyset,
            storage,
            start,
            end,
            cache: HashMap::new(),
        }
    }
}

impl<'a, K, Ser> Iterator for ValueIter<'a, K, Ser>
where
    K: Serialize + DeserializeOwned,
    Ser: Serde,
{
    type Item = StdResult<K>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }

        let key;
        let page = self.keyset.page_from_position(self.start);
        let indexes_pos = (self.start % self.keyset.page_size) as usize;

        match self.cache.get(&page) {
            Some(indexes) => {
                let key_data = &indexes[indexes_pos];
                key = self.keyset.deserialize_key(key_data);
            }
            None => match self.keyset.get_indexes(self.storage, page) {
                Ok(indexes) => {
                    let key_data = &indexes[indexes_pos];
                    key = self.keyset.deserialize_key(key_data);
                    self.cache.insert(page, indexes);
                }
                Err(e) => key = Err(e),
            },
        }
        self.start += 1;
        Some(key)
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

impl<'a, K, Ser> DoubleEndedIterator for ValueIter<'a, K, Ser>
where
    K: Serialize + DeserializeOwned,
    Ser: Serde,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        self.end -= 1;

        let key;
        let page = self.keyset.page_from_position(self.end);
        let indexes_pos = (self.end % self.keyset.page_size) as usize;

        match self.cache.get(&page) {
            Some(indexes) => {
                let key_data = &indexes[indexes_pos];
                key = self.keyset.deserialize_key(key_data);
            }
            None => match self.keyset.get_indexes(self.storage, page) {
                Ok(indexes) => {
                    let key_data = &indexes[indexes_pos];
                    key = self.keyset.deserialize_key(key_data);
                    self.cache.insert(page, indexes);
                }
                Err(e) => key = Err(e),
            },
        }
        Some(key)
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
impl<'a, K, Ser> ExactSizeIterator for ValueIter<'a, K, Ser>
where
    K: Serialize + DeserializeOwned,
    Ser: Serde,
{
}

#[cfg(test)]
mod tests {
    use secret_toolkit_serialization::Json;
    use serde::{Deserialize, Serialize};

    use cosmwasm_std::testing::MockStorage;

    use super::*;

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
    struct Foo {
        string: String,
        number: i32,
    }
    #[test]
    fn test_keyset_perf_insert() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let total_items = 1000;

        let keyset: Keyset<i32> = Keyset::new(b"test");

        for i in 0..total_items {
            keyset.insert(&mut storage, &i)?;
        }

        assert_eq!(keyset.get_len(&storage)?, 1000);

        Ok(())
    }

    #[test]
    fn test_keyset_perf_insert_remove() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let total_items = 100;

        let keyset: Keyset<i32> = Keyset::new(b"test");

        for i in 0..total_items {
            keyset.insert(&mut storage, &i)?;
        }

        for i in 0..total_items {
            keyset.remove(&mut storage, &i)?;
        }

        assert_eq!(keyset.get_len(&storage)?, 0);

        Ok(())
    }

    #[test]
    fn test_keyset_paging() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let page_size: u32 = 5;
        let total_items: u32 = 50;
        let keyset: Keyset<u32> = Keyset::new(b"test");

        for i in 0..total_items {
            keyset.insert(&mut storage, &i)?;
        }

        for i in 0..((total_items / page_size) - 1) {
            let start_page = i;

            let values = keyset.paging(&storage, start_page, page_size)?;

            for (index, value) in values.iter().enumerate() {
                let i = page_size * start_page + index as u32;
                assert_eq!(value, &i);
            }
        }

        Ok(())
    }

    #[test]
    fn test_keyset_paging_overflow() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let page_size = 50;
        let total_items = 10;
        let keyset: Keyset<u32> = Keyset::new(b"test");

        for i in 0..total_items {
            keyset.insert(&mut storage, &i)?;
        }

        let values = keyset.paging(&storage, 0, page_size)?;

        assert_eq!(values.len(), total_items as usize);

        for (index, value) in values.iter().enumerate() {
            assert_eq!(value, &(index as u32))
        }

        Ok(())
    }

    #[test]
    fn test_keyset_insert_multiple() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keyset: Keyset<Foo> = Keyset::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 1111,
        };

        assert!(keyset.insert(&mut storage, &foo1)?);
        assert!(keyset.insert(&mut storage, &foo2)?);

        assert!(keyset.contains(&storage, &foo1));
        assert!(keyset.contains(&storage, &foo2));

        assert!(!keyset.insert(&mut storage, &foo2)?);

        Ok(())
    }

    #[test]
    fn test_keyset_iter() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keyset: Keyset<Foo> = Keyset::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 1111,
        };

        keyset.insert(&mut storage, &foo1)?;
        keyset.insert(&mut storage, &foo2)?;

        let mut x = keyset.iter(&storage)?;
        let (len, _) = x.size_hint();
        assert_eq!(len, 2);

        assert_eq!(x.next().unwrap()?, foo1);

        assert_eq!(x.next().unwrap()?, foo2);

        Ok(())
    }

    #[test]
    fn test_keyset_iter_keys() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keyset: Keyset<Foo> = Keyset::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 1111,
        };

        keyset.insert(&mut storage, &foo1)?;
        keyset.insert(&mut storage, &foo2)?;

        let mut x = keyset.iter(&storage)?;
        let (len, _) = x.size_hint();
        assert_eq!(len, 2);

        assert_eq!(x.next().unwrap()?, foo1);

        assert_eq!(x.next().unwrap()?, foo2);

        Ok(())
    }

    #[test]
    fn test_keyset_suffixed_basics() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let original_keyset: Keyset<Foo> = Keyset::new(b"test");
        let keyset = original_keyset.add_suffix(b"test_suffix");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 1111,
        };
        keyset.insert(&mut storage, &foo1)?;
        keyset.insert(&mut storage, &foo2)?;

        let read_foo1 = keyset.contains(&storage, &foo1);
        let read_foo2 = keyset.contains(&storage, &foo2);

        assert_eq!(original_keyset.get_len(&storage)?, 0);
        assert!(read_foo1);
        assert!(read_foo2);

        let alternative_keyset: Keyset<Foo> = Keyset::new(b"alternative");
        let alt_same_suffix = alternative_keyset.add_suffix(b"test_suffix");

        assert!(alt_same_suffix.is_empty(&storage)?);

        // show that it loads foo1 before removal
        let before_remove_foo1 = keyset.contains(&storage, &foo1);
        assert!(before_remove_foo1);
        // and returns None after removal
        keyset.remove(&mut storage, &foo1)?;
        let removed_foo1 = keyset.contains(&storage, &foo1);
        assert!(!removed_foo1);

        let foo3 = Foo {
            string: "string three".to_string(),
            number: 1111,
        };
        // show what happens when reading from keys that have not been set yet.
        assert!(!keyset.contains(&storage, &foo3));

        Ok(())
    }

    #[test]
    fn test_keyset_length() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keyset: Keyset<Foo> = Keyset::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            // same as foo1
            string: "string one".to_string(),
            number: 1111,
        };
        let foo3 = Foo {
            string: "string three".to_string(),
            number: 1111,
        };

        assert!(keyset.length.lock().unwrap().eq(&None));
        assert_eq!(keyset.get_len(&storage)?, 0);
        assert!(keyset.length.lock().unwrap().eq(&Some(0)));

        keyset.insert(&mut storage, &foo1)?;
        assert_eq!(keyset.get_len(&storage)?, 1);
        assert!(keyset.length.lock().unwrap().eq(&Some(1)));

        // add same item
        keyset.insert(&mut storage, &foo2)?;
        assert_eq!(keyset.get_len(&storage)?, 1);
        assert!(keyset.length.lock().unwrap().eq(&Some(1)));

        // add another item
        keyset.insert(&mut storage, &foo3)?;
        assert_eq!(keyset.get_len(&storage)?, 2);
        assert!(keyset.length.lock().unwrap().eq(&Some(2)));

        // remove item and check length
        keyset.remove(&mut storage, &foo1)?;
        assert_eq!(keyset.get_len(&storage)?, 1);
        assert!(keyset.length.lock().unwrap().eq(&Some(1)));

        // remove item and check length
        keyset.remove(&mut storage, &foo3)?;
        assert_eq!(keyset.get_len(&storage)?, 0);
        assert!(keyset.length.lock().unwrap().eq(&Some(0)));

        Ok(())
    }

    #[test]
    fn test_keyset_without_iter() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keyset: Keyset<Foo, Json, _> = KeysetBuilder::new(b"test").without_iter().build();

        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 1111,
        };
        keyset.insert(&mut storage, &foo1)?;
        keyset.insert(&mut storage, &foo2)?;

        let read_foo1 = keyset.contains(&storage, &foo1);
        let read_foo2 = keyset.contains(&storage, &foo2);

        assert!(read_foo1);
        assert!(read_foo2);

        keyset.remove(&mut storage, &foo1)?;

        let read_foo1 = keyset.contains(&storage, &foo1);
        let read_foo2 = keyset.contains(&storage, &foo2);

        assert!(!read_foo1);
        assert!(read_foo2);

        Ok(())
    }

    #[test]
    fn test_keyset_custom_paging() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let page_size: u32 = 5;
        let total_items: u32 = 50;
        let keyset: Keyset<u32> = KeysetBuilder::new(b"test").with_page_size(13).build();

        for i in 0..total_items {
            keyset.insert(&mut storage, &i)?;
        }

        for i in 0..((total_items / page_size) - 1) {
            let start_page = i;

            let values = keyset.paging(&storage, start_page, page_size)?;

            for (index, value) in values.iter().enumerate() {
                let i = page_size * start_page + index as u32;
                assert_eq!(value, &i);
            }
        }

        Ok(())
    }

    #[test]
    fn test_keyset_custom_paging_overflow() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let page_size = 50;
        let total_items = 10;
        let keyset: Keyset<u32> = KeysetBuilder::new(b"test").with_page_size(3).build();

        for i in 0..total_items {
            keyset.insert(&mut storage, &i)?;
        }

        let values = keyset.paging(&storage, 0, page_size)?;

        assert_eq!(values.len(), total_items as usize);

        for (index, value) in values.iter().enumerate() {
            assert_eq!(value, &(index as u32))
        }

        Ok(())
    }

    #[test]
    fn test_keyset_custom_page_iter() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let keyset: Keyset<Foo> = KeysetBuilder::new(b"test").with_page_size(2).build();
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 1111,
        };
        let foo3 = Foo {
            string: "string three".to_string(),
            number: 1111,
        };

        keyset.insert(&mut storage, &foo1)?;
        keyset.insert(&mut storage, &foo2)?;
        keyset.insert(&mut storage, &foo3)?;

        let mut x = keyset.iter(&storage)?;
        let (len, _) = x.size_hint();
        assert_eq!(len, 3);

        assert_eq!(x.next().unwrap()?, foo1);

        assert_eq!(x.next().unwrap()?, foo2);

        assert_eq!(x.next().unwrap()?, foo3);

        assert_eq!(x.next(), None);

        Ok(())
    }

    #[test]
    fn test_reverse_iter() -> StdResult<()> {
        test_keyset_custom_page_reverse_iterator(1)?;
        test_keyset_custom_page_reverse_iterator(2)?;
        test_keyset_custom_page_reverse_iterator(13)?;
        Ok(())
    }

    fn test_keyset_custom_page_reverse_iterator(page_size: u32) -> StdResult<()> {
        let mut storage = MockStorage::new();
        let keymap: Keyset<i32> = KeysetBuilder::new(b"test")
            .with_page_size(page_size)
            .build();
        keymap.insert(&mut storage, &1234)?;
        keymap.insert(&mut storage, &2143)?;
        keymap.insert(&mut storage, &3412)?;
        keymap.insert(&mut storage, &4321)?;

        let mut iter = keymap.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // iterate twice to make sure nothing changed
        let mut iter = keymap.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth_back` doesn't break anything
        let mut iter = keymap.iter(&storage)?.rev().skip(2);
        assert_eq!(iter.next(), Some(Ok(2143)));
        assert_eq!(iter.next(), Some(Ok(1234)));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `ExactSizeIterator` works well
        let mut iter = keymap.iter(&storage)?.skip(2).rev();
        assert_eq!(iter.next(), Some(Ok(4321)));
        assert_eq!(iter.next(), Some(Ok(3412)));
        assert_eq!(iter.next(), None);

        Ok(())
    }
}
