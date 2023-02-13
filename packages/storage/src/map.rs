use std::any::type_name;
use std::collections::HashMap;
use std::convert::TryInto;
use std::marker::PhantomData;
use std::sync::Mutex;

use serde::Deserialize;
use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{StdError, StdResult, Storage};
use cosmwasm_storage::to_length_prefixed;

use secret_toolkit_serialization::{Bincode2, Serde};

use crate::{IterOption, WithIter, WithoutIter};

const INDEXES: &[u8] = b"indexes";
const MAP_LENGTH: &[u8] = b"length";

const DEFAULT_PAGE_SIZE: u32 = 1;

#[derive(Serialize, Deserialize)]
struct InternalItem<T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    item_vec: Vec<u8>,
    // only Some if we enabled iterator
    index_pos: Option<u32>,
    item_type: PhantomData<T>,
    serialization_type: PhantomData<Ser>,
}

impl<T: Serialize + DeserializeOwned, Ser: Serde> InternalItem<T, Ser> {
    fn new(index_pos: Option<u32>, item: &T) -> StdResult<Self> {
        Ok(Self {
            item_vec: Ser::serialize(item)?,
            index_pos,
            item_type: PhantomData,
            serialization_type: PhantomData,
        })
    }

    fn get_item(&self) -> StdResult<T> {
        Ser::deserialize(&self.item_vec)
    }
}

pub struct MapBuilder<'a, K, T, Ser = Bincode2, I = WithIter> {
    /// namespace of the newly constructed Storage
    namespace: &'a [u8],
    page_size: u32,
    key_type: PhantomData<K>,
    item_type: PhantomData<T>,
    serialization_type: PhantomData<Ser>,
    iter_option: PhantomData<I>,
}

impl<'a, K, T, Ser> MapBuilder<'a, K, T, Ser, WithIter>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// Creates a MapBuilder with default features
    pub const fn new(namespace: &'a [u8]) -> Self {
        Self {
            namespace,
            page_size: DEFAULT_PAGE_SIZE,
            key_type: PhantomData,
            item_type: PhantomData,
            serialization_type: PhantomData,
            iter_option: PhantomData,
        }
    }
    /// Modifies the number of keys stored in one page of indexing, for the iterator
    pub const fn with_page_size(&self, indexes_size: u32) -> Self {
        if indexes_size == 0 {
            panic!("zero index page size used in map")
        }
        Self {
            namespace: self.namespace,
            page_size: indexes_size,
            key_type: self.key_type,
            item_type: self.item_type,
            serialization_type: self.serialization_type,
            iter_option: self.iter_option,
        }
    }
    /// Disables the iterator of the map, saving at least 4000 gas in each insertion.
    pub const fn without_iter(&self) -> MapBuilder <'a, K, T, Ser, WithoutIter> {
        MapBuilder {
            namespace: self.namespace,
            page_size: self.page_size,
            key_type: PhantomData,
            item_type: PhantomData,
            serialization_type: PhantomData,
            iter_option: PhantomData,
        }
    }
    /// Returns a map with the given configuration
    pub const fn build(&self) -> Map<'a, K, T, Ser, WithIter> {
        Map {
            namespace: self.namespace,
            prefix: None,
            page_size: self.page_size,
            length: Mutex::new(None),
            key_type: self.key_type,
            item_type: self.item_type,
            iter_option: self.iter_option,
            serialization_type: self.serialization_type,
        }
    }
}

// This enables writing `.iter().skip(n).rev()`
impl<'a, K, T, Ser> MapBuilder<'a, K, T, Ser, WithoutIter>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    pub const fn build(&self) -> Map<'a, K, T, Ser, WithoutIter> {
        Map {
            namespace: self.namespace,
            prefix: None,
            page_size: self.page_size,
            length: Mutex::new(None),
            key_type: self.key_type,
            item_type: self.item_type,
            iter_option: self.iter_option,
            serialization_type: self.serialization_type,
        }
    }
}

pub struct Map<'a, K, T, Ser = Bincode2, I = WithIter>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
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
    item_type: PhantomData<T>,
    iter_option: PhantomData<I>,
    serialization_type: PhantomData<Ser>,
}

impl<'a, K: Serialize + DeserializeOwned, T: Serialize + DeserializeOwned, Ser: Serde>
    Map<'a, K, T, Ser>
{
    /// constructor
    pub const fn new(namespace: &'a [u8]) -> Self {
        Self {
            namespace,
            prefix: None,
            page_size: DEFAULT_PAGE_SIZE,
            length: Mutex::new(None),
            key_type: PhantomData,
            item_type: PhantomData,
            serialization_type: PhantomData,
            iter_option: PhantomData,
        }
    }

    /// This is used to produce a new Map. This can be used when you want to associate an Map to each user
    /// and you still get to define the Map as a static constant
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
            item_type: self.item_type,
            serialization_type: self.serialization_type,
            iter_option: self.iter_option,
        }
    }
}

impl<'a, K: Serialize + DeserializeOwned, T: Serialize + DeserializeOwned, Ser: Serde>
    Map<'a, K, T, Ser, WithoutIter>
{
    /// Serialize key
    fn serialize_key(&self, key: &K) -> StdResult<Vec<u8>> {
        Ser::serialize(key)
    }

    /// user facing get function
    pub fn get(&self, storage: &dyn Storage, key: &K) -> Option<T> {
        self.get_from_key(storage, key).ok()
    }

    /// internal item get function
    fn get_from_key(&self, storage: &dyn Storage, key: &K) -> StdResult<T> {
        let key_vec = self.serialize_key(key)?;
        self.load_impl(storage, &key_vec)
    }

    /// user facing remove function
    pub fn remove(&self, storage: &mut dyn Storage, key: &K) -> StdResult<()> {
        let key_vec = self.serialize_key(key)?;
        self.remove_impl(storage, &key_vec);

        Ok(())
    }

    /// user facing insert function
    pub fn insert(&self, storage: &mut dyn Storage, key: &K, item: &T) -> StdResult<()> {
        let key_vec = self.serialize_key(key)?;
        self.save_impl(storage, &key_vec, item)
    }

    /// user facing method that checks if any item is stored with this key.
    pub fn contains(&self, storage: &dyn Storage, key: &K) -> bool {
        match self.serialize_key(key) {
            Ok(key_vec) => self.contains_impl(storage, &key_vec),
            Err(_) => false,
        }
    }
}

impl<'a, K: Serialize + DeserializeOwned, T: Serialize + DeserializeOwned, Ser: Serde>
    Map<'a, K, T, Ser, WithIter>
{
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

    /// user facing get function
    pub fn get(&self, storage: &dyn Storage, key: &K) -> Option<T> {
        if let Ok(internal_item) = self.get_from_key(storage, key) {
            internal_item.get_item().ok()
        } else {
            None
        }
    }

    /// internal item get function
    fn get_from_key(&self, storage: &dyn Storage, key: &K) -> StdResult<InternalItem<T, Ser>> {
        let key_vec = self.serialize_key(key)?;
        self.load_impl(storage, &key_vec)
    }

    /// user facing remove function
    pub fn remove(&self, storage: &mut dyn Storage, key: &K) -> StdResult<()> {
        let key_vec = self.serialize_key(key)?;

        let removed_pos = self.get_from_key(storage, key)?.index_pos.unwrap();

        let page = self.page_from_position(removed_pos);

        let mut len = self.get_len(storage)?;
        len -= 1;
        self.set_len(storage, len)?;

        let mut indexes = self.get_indexes(storage, page)?;

        let pos_in_indexes = (removed_pos % self.page_size) as usize;

        if indexes[pos_in_indexes] != key_vec {
            return Err(StdError::generic_err(
                "tried to remove from map, but key not found in indexes - should never happen",
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
            let last_key = indexes.pop().ok_or_else(|| {
                StdError::generic_err("last item's key not found - should never happen")
            })?;
            // modify last item
            let mut last_internal_item = self.load_impl(storage, &last_key)?;
            last_internal_item.index_pos = Some(removed_pos);
            self.save_impl(storage, &last_key, &last_internal_item)?;
            // save to indexes
            indexes[pos_in_indexes] = last_key;
            self.set_indexes_page(storage, page, &indexes)?;
        } else {
            let mut last_page_indexes = self.get_indexes(storage, max_page)?;
            let last_key = last_page_indexes.pop().ok_or_else(|| {
                StdError::generic_err("last item's key not found - should never happen")
            })?;
            // modify last item
            let mut last_internal_item = self.load_impl(storage, &last_key)?;
            last_internal_item.index_pos = Some(removed_pos);
            self.save_impl(storage, &last_key, &last_internal_item)?;
            // save indexes
            indexes[pos_in_indexes] = last_key;
            self.set_indexes_page(storage, page, &indexes)?;
            self.set_indexes_page(storage, max_page, &last_page_indexes)?;
        }

        self.remove_impl(storage, &key_vec);

        Ok(())
    }

    /// user facing insert function
    pub fn insert(&self, storage: &mut dyn Storage, key: &K, item: &T) -> StdResult<()> {
        let key_vec = self.serialize_key(key)?;

        match self.may_load_impl(storage, &key_vec)? {
            Some(existing_internal_item) => {
                // if item already exists
                let new_internal_item = InternalItem::new(existing_internal_item.index_pos, item)?;
                self.save_impl(storage, &key_vec, &new_internal_item)
            }
            None => {
                // not already saved
                let pos = self.get_len(storage)?;
                self.set_len(storage, pos + 1)?;
                let page = self.page_from_position(pos);
                // save the item
                let internal_item = InternalItem::new(Some(pos), item)?;
                self.save_impl(storage, &key_vec, &internal_item)?;
                // add index
                let mut indexes = self.get_indexes(storage, page)?;
                indexes.push(key_vec);
                self.set_indexes_page(storage, page, &indexes)
            }
        }
    }

    /// user facing method that checks if any item is stored with this key.
    pub fn contains(&self, storage: &dyn Storage, key: &K) -> bool {
        match self.serialize_key(key) {
            Ok(key_vec) => self.contains_impl(storage, &key_vec),
            Err(_) => false,
        }
    }

    /// paginates (key, item) pairs.
    pub fn paging(
        &self,
        storage: &dyn Storage,
        start_page: u32,
        size: u32,
    ) -> StdResult<Vec<(K, T)>> {
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
        self.get_pairs_at_positions(storage, start_pos, end_pos)
    }

    /// paginates only the keys. More efficient than paginating both items and keys
    pub fn paging_keys(
        &self,
        storage: &dyn Storage,
        start_page: u32,
        size: u32,
    ) -> StdResult<Vec<K>> {
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

    /// tries to list (key, item) pairs without checking start/end bounds
    fn get_pairs_at_positions(
        &self,
        storage: &dyn Storage,
        start: u32,
        end: u32,
    ) -> StdResult<Vec<(K, T)>> {
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
                let item = self.load_impl(storage, key_vec)?.get_item()?;
                res.push((key, item));
            }
        }
        Ok(res)
    }

    /// Returns a readonly iterator only for keys. More efficient than iter().
    pub fn iter_keys(&self, storage: &'a dyn Storage) -> StdResult<KeyIter<K, T, Ser>> {
        let len = self.get_len(storage)?;
        let iter = KeyIter::new(self, storage, 0, len);
        Ok(iter)
    }

    /// Returns a readonly iterator for (key-item) pairs
    pub fn iter(&self, storage: &'a dyn Storage) -> StdResult<KeyItemIter<K, T, Ser>> {
        let len = self.get_len(storage)?;
        let iter = KeyItemIter::new(self, storage, 0, len);
        Ok(iter)
    }
}

impl<'a, K: Serialize + DeserializeOwned, T: Serialize + DeserializeOwned, Ser: Serde>
    PrefixedTypedStorage<InternalItem<T, Ser>, Bincode2> for Map<'a, K, T, Ser, WithIter>
{
    fn as_slice(&self) -> &[u8] {
        if let Some(prefix) = &self.prefix {
            prefix
        } else {
            self.namespace
        }
    }
}

impl<'a, K: Serialize + DeserializeOwned, T: Serialize + DeserializeOwned, Ser: Serde>
    PrefixedTypedStorage<T, Ser> for Map<'a, K, T, Ser, WithoutIter>
{
    fn as_slice(&self) -> &[u8] {
        if let Some(prefix) = &self.prefix {
            prefix
        } else {
            self.namespace
        }
    }
}

/// An iterator over the keys of the Map.
pub struct KeyIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    map: &'a Map<'a, K, T, Ser>,
    storage: &'a dyn Storage,
    start: u32,
    end: u32,
    cache: HashMap<u32, Vec<Vec<u8>>>,
}

impl<'a, K, T, Ser> KeyIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// constructor
    pub fn new(
        map: &'a Map<'a, K, T, Ser>,
        storage: &'a dyn Storage,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            map,
            storage,
            start,
            end,
            cache: HashMap::new(),
        }
    }
}

impl<'a, K, T, Ser> Iterator for KeyIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    type Item = StdResult<K>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }

        let key;
        let page = self.map.page_from_position(self.start);
        let indexes_pos = (self.start % self.map.page_size) as usize;

        match self.cache.get(&page) {
            Some(indexes) => {
                let key_data = &indexes[indexes_pos];
                key = self.map.deserialize_key(key_data);
            }
            None => match self.map.get_indexes(self.storage, page) {
                Ok(indexes) => {
                    let key_data = &indexes[indexes_pos];
                    key = self.map.deserialize_key(key_data);
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
    // `.iter().skip(start).take(length).collect()`
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.start = self.start.saturating_add(n as u32);
        self.next()
    }
}

impl<'a, K, T, Ser> DoubleEndedIterator for KeyIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        self.end -= 1;

        let key;
        let page = self.map.page_from_position(self.end);
        let indexes_pos = (self.end % self.map.page_size) as usize;

        match self.cache.get(&page) {
            Some(indexes) => {
                let key_data = &indexes[indexes_pos];
                key = self.map.deserialize_key(key_data);
            }
            None => match self.map.get_indexes(self.storage, page) {
                Ok(indexes) => {
                    let key_data = &indexes[indexes_pos];
                    key = self.map.deserialize_key(key_data);
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
    // `.iter().skip(start).take(length).collect()`
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.end = self.end.saturating_sub(n as u32);
        self.next_back()
    }
}

// This enables writing `.iter().skip(n).rev()`
impl<'a, K, T, Ser> ExactSizeIterator for KeyIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
}

// ===============================================================================================

/// An iterator over the (key, item) pairs of the Map. Less efficient than just iterating over keys.
pub struct KeyItemIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    map: &'a Map<'a, K, T, Ser>,
    storage: &'a dyn Storage,
    start: u32,
    end: u32,
    cache: HashMap<u32, Vec<Vec<u8>>>,
}

impl<'a, K, T, Ser> KeyItemIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// constructor
    pub fn new(
        map: &'a Map<'a, K, T, Ser>,
        storage: &'a dyn Storage,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            map,
            storage,
            start,
            end,
            cache: HashMap::new(),
        }
    }
}

impl<'a, K, T, Ser> Iterator for KeyItemIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    type Item = StdResult<(K, T)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }

        let key;
        let page = self.map.page_from_position(self.start);
        let indexes_pos = (self.start % self.map.page_size) as usize;

        match self.cache.get(&page) {
            Some(indexes) => {
                let key_data = &indexes[indexes_pos];
                key = self.map.deserialize_key(key_data);
            }
            None => match self.map.get_indexes(self.storage, page) {
                Ok(indexes) => {
                    let key_data = &indexes[indexes_pos];
                    key = self.map.deserialize_key(key_data);
                    self.cache.insert(page, indexes);
                }
                Err(e) => key = Err(e),
            },
        }
        self.start += 1;
        // turn key into pair
        let pair = match key {
            Ok(k) => match self.map.get_from_key(self.storage, &k) {
                Ok(internal_item) => match internal_item.get_item() {
                    Ok(item) => Ok((k, item)),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        };
        Some(pair)
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
    // `.iter().skip(start).take(length).collect()`
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.start = self.start.saturating_add(n as u32);
        self.next()
    }
}

impl<'a, K, T, Ser> DoubleEndedIterator for KeyItemIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        self.end -= 1;

        let key;
        let page = self.map.page_from_position(self.end);
        let indexes_pos = (self.end % self.map.page_size) as usize;

        match self.cache.get(&page) {
            Some(indexes) => {
                let key_data = &indexes[indexes_pos];
                key = self.map.deserialize_key(key_data);
            }
            None => match self.map.get_indexes(self.storage, page) {
                Ok(indexes) => {
                    let key_data = &indexes[indexes_pos];
                    key = self.map.deserialize_key(key_data);
                    self.cache.insert(page, indexes);
                }
                Err(e) => key = Err(e),
            },
        }
        // turn key into pair
        let pair = match key {
            Ok(k) => match self.map.get_from_key(self.storage, &k) {
                Ok(internal_item) => match internal_item.get_item() {
                    Ok(item) => Ok((k, item)),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        };
        Some(pair)
    }

    // I implement `nth_back` manually because it is used in the standard library whenever
    // it wants to skip over elements, but the default implementation repeatedly calls next_back.
    // because that is very expensive in this case, and the items are just discarded, we wan
    // do better here.
    // In practice, this enables cheap paging over the storage by calling:
    // `.iter().skip(start).take(length).collect()`
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.end = self.end.saturating_sub(n as u32);
        self.next_back()
    }
}

// This enables writing `.iter().skip(n).rev()`
impl<'a, K, T, Ser> ExactSizeIterator for KeyItemIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
}

trait PrefixedTypedStorage<T: Serialize + DeserializeOwned, Ser: Serde> {
    fn as_slice(&self) -> &[u8];

    /// Returns bool from retrieving the item with the specified key.
    ///
    /// # Arguments
    ///
    /// * `storage` - a reference to the storage this item is in
    /// * `key` - a byte slice representing the key to access the stored item
    fn contains_impl(&self, storage: &dyn Storage, key: &[u8]) -> bool {
        let prefixed_key = [self.as_slice(), key].concat();
        storage.get(&prefixed_key).is_some()
    }

    /// Returns StdResult<T> from retrieving the item with the specified key.  Returns a
    /// StdError::NotFound if there is no item with that key
    ///
    /// # Arguments
    ///
    /// * `storage` - a reference to the storage this item is in
    /// * `key` - a byte slice representing the key to access the stored item
    fn load_impl(&self, storage: &dyn Storage, key: &[u8]) -> StdResult<T> {
        let prefixed_key = [self.as_slice(), key].concat();
        Ser::deserialize(
            &storage
                .get(&prefixed_key)
                .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
        )
    }

    /// Returns StdResult<Option<T>> from retrieving the item with the specified key.  Returns a
    /// None if there is no item with that key
    ///
    /// # Arguments
    ///
    /// * `storage` - a reference to the storage this item is in
    /// * `key` - a byte slice representing the key to access the stored item
    fn may_load_impl(&self, storage: &dyn Storage, key: &[u8]) -> StdResult<Option<T>> {
        let prefixed_key = [self.as_slice(), key].concat();
        match storage.get(&prefixed_key) {
            Some(value) => Ser::deserialize(&value).map(Some),
            None => Ok(None),
        }
    }

    /// Returns StdResult<()> resulting from saving an item to storage
    ///
    /// # Arguments
    ///
    /// * `storage` - a mutable reference to the storage this item should go to
    /// * `key` - a byte slice representing the key to access the stored item
    /// * `value` - a reference to the item to store
    fn save_impl(&self, storage: &mut dyn Storage, key: &[u8], value: &T) -> StdResult<()> {
        let prefixed_key = [self.as_slice(), key].concat();
        storage.set(&prefixed_key, &Ser::serialize(value)?);
        Ok(())
    }

    /// Removes an item from storage
    ///
    /// # Arguments
    ///
    /// * `storage` - a mutable reference to the storage this item is in
    /// * `key` - a byte slice representing the key to access the stored item
    fn remove_impl(&self, storage: &mut dyn Storage, key: &[u8]) {
        let prefixed_key = [self.as_slice(), key].concat();
        storage.remove(&prefixed_key);
    }
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
    fn map_perf_insert() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let total_items = 1000;

        let map: Map<Vec<u8>, i32> = Map::new(b"test");

        for i in 0..total_items {
            let key: Vec<u8> = (i as i32).to_be_bytes().to_vec();
            map.insert(&mut storage, &key, &i)?;
        }

        assert_eq!(map.get_len(&storage)?, 1000);

        Ok(())
    }

    #[test]
    fn test_map_perf_insert_remove() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let total_items = 100;

        let map: Map<i32, i32> = Map::new(b"test");

        for i in 0..total_items {
            map.insert(&mut storage, &i, &i)?;
        }

        for i in 0..total_items {
            map.remove(&mut storage, &i)?;
        }

        assert_eq!(map.get_len(&storage)?, 0);

        Ok(())
    }

    #[test]
    fn test_map_paging() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let page_size: u32 = 5;
        let total_items: u32 = 50;
        let map: Map<Vec<u8>, u32> = Map::new(b"test");

        for i in 0..total_items {
            let key: Vec<u8> = (i as i32).to_be_bytes().to_vec();
            map.insert(&mut storage, &key, &i)?;
        }

        for i in 0..((total_items / page_size) - 1) {
            let start_page = i;

            let values = map.paging(&storage, start_page, page_size)?;

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
    fn test_map_paging_overflow() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let page_size = 50;
        let total_items = 10;
        let map: Map<i32, u32> = Map::new(b"test");

        for i in 0..total_items {
            map.insert(&mut storage, &(i as i32), &i)?;
        }

        let values = map.paging_keys(&storage, 0, page_size)?;

        assert_eq!(values.len(), total_items as usize);

        for (index, value) in values.iter().enumerate() {
            assert_eq!(value, &(index as i32))
        }

        Ok(())
    }

    #[test]
    fn test_map_insert_multiple() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let map: Map<Vec<u8>, Foo> = Map::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 1111,
        };

        map.insert(&mut storage, &b"key1".to_vec(), &foo1)?;
        map.insert(&mut storage, &b"key2".to_vec(), &foo2)?;

        let read_foo1 = map.get(&storage, &b"key1".to_vec()).unwrap();
        let read_foo2 = map.get(&storage, &b"key2".to_vec()).unwrap();

        assert_eq!(foo1, read_foo1);
        assert_eq!(foo2, read_foo2);
        Ok(())
    }

    #[test]
    fn test_map_contains() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let map: Map<Vec<u8>, Foo> = Map::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };

        map.insert(&mut storage, &b"key1".to_vec(), &foo1)?;
        let contains_k1 = map.contains(&storage, &b"key1".to_vec());

        assert!(contains_k1);

        Ok(())
    }

    #[test]
    fn test_map_iter() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let map: Map<Vec<u8>, Foo> = Map::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 1111,
        };

        map.insert(&mut storage, &b"key1".to_vec(), &foo1)?;
        map.insert(&mut storage, &b"key2".to_vec(), &foo2)?;

        let mut x = map.iter(&storage)?;
        let (len, _) = x.size_hint();
        assert_eq!(len, 2);

        assert_eq!(x.next().unwrap()?, (b"key1".to_vec(), foo1));

        assert_eq!(x.next().unwrap()?, (b"key2".to_vec(), foo2));

        Ok(())
    }

    #[test]
    fn test_map_iter_keys() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let map: Map<String, Foo> = Map::new(b"test");
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

        map.insert(&mut storage, &key1, &foo1)?;
        map.insert(&mut storage, &key2, &foo2)?;

        let mut x = map.iter_keys(&storage)?;
        let (len, _) = x.size_hint();
        assert_eq!(len, 2);

        assert_eq!(x.next().unwrap()?, key1);

        assert_eq!(x.next().unwrap()?, key2);

        Ok(())
    }

    #[test]
    fn test_map_overwrite() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let map: Map<Vec<u8>, Foo> = Map::new(b"test");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string two".to_string(),
            number: 2222,
        };

        map.insert(&mut storage, &b"key1".to_vec(), &foo1)?;
        map.insert(&mut storage, &b"key1".to_vec(), &foo2)?;

        let foo3 = map.get(&storage, &b"key1".to_vec()).unwrap();

        assert_eq!(foo3, foo2);

        Ok(())
    }

    #[test]
    fn test_map_suffixed_basics() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let original_map: Map<String, Foo> = Map::new(b"test");
        let map = original_map.add_suffix(b"test_suffix");
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        map.insert(&mut storage, &"key1".to_string(), &foo1)?;
        map.insert(&mut storage, &"key2".to_string(), &foo2)?;

        let read_foo1 = map.get(&storage, &"key1".to_string()).unwrap();
        let read_foo2 = map.get(&storage, &"key2".to_string()).unwrap();

        assert_eq!(original_map.get_len(&storage)?, 0);
        assert_eq!(foo1, read_foo1);
        assert_eq!(foo2, read_foo2);

        let alternative_map: Map<String, Foo> = Map::new(b"alternative");
        let alt_same_suffix = alternative_map.add_suffix(b"test_suffix");

        assert!(alt_same_suffix.is_empty(&storage)?);

        // show that it loads foo1 before removal
        let before_remove_foo1 = map.get(&storage, &"key1".to_string());
        assert!(before_remove_foo1.is_some());
        assert_eq!(foo1, before_remove_foo1.unwrap());
        // and returns None after removal
        map.remove(&mut storage, &"key1".to_string())?;
        let removed_foo1 = map.get(&storage, &"key1".to_string());
        assert!(removed_foo1.is_none());

        // show what happens when reading from keys that have not been set yet.
        assert!(map.get(&storage, &"key3".to_string()).is_none());

        Ok(())
    }

    #[test]
    fn test_map_length() -> StdResult<()> {
        test_map_length_with_page_size(1)?;
        test_map_length_with_page_size(5)?;
        test_map_length_with_page_size(13)?;
        Ok(())
    }

    fn test_map_length_with_page_size(page_size: u32) -> StdResult<()> {
        let mut storage = MockStorage::new();

        let map: Map<String, Foo> = MapBuilder::new(b"test")
            .with_page_size(page_size)
            .build();
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };

        assert!(map.length.lock().unwrap().eq(&None));
        assert_eq!(map.get_len(&storage)?, 0);
        assert!(map.length.lock().unwrap().eq(&Some(0)));

        let key1 = "k1".to_string();
        let key2 = "k2".to_string();

        map.insert(&mut storage, &key1, &foo1)?;
        assert_eq!(map.get_len(&storage)?, 1);
        assert!(map.length.lock().unwrap().eq(&Some(1)));

        // add another item
        map.insert(&mut storage, &key2, &foo2)?;
        assert_eq!(map.get_len(&storage)?, 2);
        assert!(map.length.lock().unwrap().eq(&Some(2)));

        // remove item and check length
        map.remove(&mut storage, &key1)?;
        assert_eq!(map.get_len(&storage)?, 1);
        assert!(map.length.lock().unwrap().eq(&Some(1)));

        // override item (should not change length)
        map.insert(&mut storage, &key2, &foo1)?;
        assert_eq!(map.get_len(&storage)?, 1);
        assert!(map.length.lock().unwrap().eq(&Some(1)));

        // remove item and check length
        map.remove(&mut storage, &key2)?;
        assert_eq!(map.get_len(&storage)?, 0);
        assert!(map.length.lock().unwrap().eq(&Some(0)));

        Ok(())
    }

    #[test]
    fn test_map_without_iter() -> StdResult<()> {
        test_map_without_iter_custom_page(1)?;
        test_map_without_iter_custom_page(2)?;
        test_map_without_iter_custom_page(3)?;
        Ok(())
    }

    fn test_map_without_iter_custom_page(page_size: u32) -> StdResult<()> {
        let mut storage = MockStorage::new();

        let map: Map<String, Foo, Json, _> = MapBuilder::new(b"test")
            .with_page_size(page_size)
            .without_iter()
            .build();

        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        map.insert(&mut storage, &"key1".to_string(), &foo1)?;
        map.insert(&mut storage, &"key2".to_string(), &foo2)?;

        let read_foo1 = map.get(&storage, &"key1".to_string()).unwrap();
        let read_foo2 = map.get(&storage, &"key2".to_string()).unwrap();

        assert_eq!(foo1, read_foo1);
        assert_eq!(foo2, read_foo2);
        assert!(map.contains(&storage, &"key1".to_string()));

        map.remove(&mut storage, &"key1".to_string())?;

        let read_foo1 = map.get(&storage, &"key1".to_string());
        let read_foo2 = map.get(&storage, &"key2".to_string()).unwrap();

        assert!(read_foo1.is_none());
        assert_eq!(foo2, read_foo2);

        Ok(())
    }

    #[test]
    fn test_map_custom_paging() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let page_size: u32 = 5;
        let total_items: u32 = 50;
        let map: Map<Vec<u8>, u32> = MapBuilder::new(b"test").with_page_size(13).build();

        for i in 0..total_items {
            let key: Vec<u8> = (i as i32).to_be_bytes().to_vec();
            map.insert(&mut storage, &key, &i)?;
        }

        for i in 0..((total_items / page_size) - 1) {
            let start_page = i;

            let values = map.paging(&storage, start_page, page_size)?;

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
    fn test_map_custom_paging_overflow() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let page_size = 50;
        let total_items = 10;
        let map: Map<i32, u32, Json> = MapBuilder::new(b"test").with_page_size(3).build();

        for i in 0..total_items {
            map.insert(&mut storage, &(i as i32), &i)?;
        }

        let values = map.paging_keys(&storage, 0, page_size)?;

        assert_eq!(values.len(), total_items as usize);

        for (index, value) in values.iter().enumerate() {
            assert_eq!(value, &(index as i32))
        }

        Ok(())
    }

    #[test]
    fn test_map_custom_page_iter() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let map: Map<Vec<u8>, Foo> = MapBuilder::new(b"test").with_page_size(2).build();
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

        map.insert(&mut storage, &b"key1".to_vec(), &foo1)?;
        map.insert(&mut storage, &b"key2".to_vec(), &foo2)?;
        map.insert(&mut storage, &b"key3".to_vec(), &foo3)?;

        let mut x = map.iter(&storage)?;
        let (len, _) = x.size_hint();
        assert_eq!(len, 3);

        assert_eq!(x.next().unwrap()?, (b"key1".to_vec(), foo1));

        assert_eq!(x.next().unwrap()?, (b"key2".to_vec(), foo2));

        assert_eq!(x.next().unwrap()?, (b"key3".to_vec(), foo3));

        assert_eq!(x.next(), None);

        Ok(())
    }

    #[test]
    fn test_map_reverse_iter() -> StdResult<()> {
        test_map_custom_page_reverse_iterator(1)?;
        test_map_custom_page_reverse_iterator(2)?;
        test_map_custom_page_reverse_iterator(5)?;
        test_map_custom_page_reverse_iterator(25)?;
        Ok(())
    }

    fn test_map_custom_page_reverse_iterator(page_size: u32) -> StdResult<()> {
        let mut storage = MockStorage::new();
        let map: Map<i32, i32> = MapBuilder::new(b"test")
            .with_page_size(page_size)
            .build();
        map.insert(&mut storage, &1234, &1234)?;
        map.insert(&mut storage, &2143, &2143)?;
        map.insert(&mut storage, &3412, &3412)?;
        map.insert(&mut storage, &4321, &4321)?;

        let mut iter = map.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok((4321, 4321))));
        assert_eq!(iter.next(), Some(Ok((3412, 3412))));
        assert_eq!(iter.next(), Some(Ok((2143, 2143))));
        assert_eq!(iter.next(), Some(Ok((1234, 1234))));
        assert_eq!(iter.next(), None);

        // iterate twice to make sure nothing changed
        let mut iter = map.iter(&storage)?.rev();
        assert_eq!(iter.next(), Some(Ok((4321, 4321))));
        assert_eq!(iter.next(), Some(Ok((3412, 3412))));
        assert_eq!(iter.next(), Some(Ok((2143, 2143))));
        assert_eq!(iter.next(), Some(Ok((1234, 1234))));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `nth_back` doesn't break anything
        let mut iter = map.iter(&storage)?.rev().skip(2);
        assert_eq!(iter.next(), Some(Ok((2143, 2143))));
        assert_eq!(iter.next(), Some(Ok((1234, 1234))));
        assert_eq!(iter.next(), None);

        // make sure our implementation of `ExactSizeIterator` works well
        let mut iter = map.iter(&storage)?.skip(2).rev();
        assert_eq!(iter.next(), Some(Ok((4321, 4321))));
        assert_eq!(iter.next(), Some(Ok((3412, 3412))));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_serializations() -> StdResult<()> {
        test_serializations_with_page_size(1)?;
        test_serializations_with_page_size(3)?;
        test_serializations_with_page_size(19)?;
        Ok(())
    }

    fn test_serializations_with_page_size(page_size: u32) -> StdResult<()> {
        // Check the default behavior is Bincode2
        let mut storage = MockStorage::new();

        let map: Map<i32, i32> = MapBuilder::new(b"test")
            .with_page_size(page_size)
            .build();
        map.insert(&mut storage, &1234, &1234)?;

        let page_key = [map.as_slice(), INDEXES, &0_u32.to_be_bytes()].concat();
        if map.page_size == 1 {
            let item_data = storage.get(&page_key);
            let expected_data = Bincode2::serialize(&1234)?;
            assert_eq!(item_data, Some(expected_data));
        } else {
            let page_bytes = storage.get(&page_key);
            let expected_bincode2 = Bincode2::serialize(&vec![Bincode2::serialize(&1234)?])?;
            assert_eq!(page_bytes, Some(expected_bincode2));
        }

        // Check that overriding the serializer with Json works
        let mut storage = MockStorage::new();
        let json_map: Map<i32, i32, Json> = MapBuilder::new(b"test2")
            .with_page_size(page_size)
            .build();
        json_map.insert(&mut storage, &1234, &1234)?;

        let key = [json_map.as_slice(), INDEXES, &0_u32.to_be_bytes()].concat();
        if json_map.page_size == 1 {
            let item_data = storage.get(&key);
            let expected = b"1234".to_vec();
            assert_eq!(item_data, Some(expected));
        } else {
            let bytes = storage.get(&key);
            let expected = Bincode2::serialize(&vec![b"1234".to_vec()])?;
            assert_eq!(bytes, Some(expected));
        }

        Ok(())
    }
}
