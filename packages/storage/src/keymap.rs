use std::any::type_name;
use std::convert::TryInto;
use std::marker::PhantomData;
use std::sync::Mutex;

use serde::Deserialize;
use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{StdError, StdResult, Storage};
use cosmwasm_storage::to_length_prefixed;

use secret_toolkit_serialization::{Bincode2, Serde};

const INDEXES: &[u8] = b"indexes";
const MAP_LENGTH: &[u8] = b"length";

const PAGE_SIZE: u32 = 5;

fn _page_from_position(position: u32) -> u32 {
    position / PAGE_SIZE
}

#[derive(Serialize, Deserialize)]
struct InternalItem<T, Ser>
where
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    item_vec: Vec<u8>,
    index_pos: u32,
    item_type: PhantomData<T>,
    serialization_type: PhantomData<Ser>,
}

impl<T: Serialize + DeserializeOwned, Ser: Serde> InternalItem<T, Ser> {
    fn new(index_pos: u32, item: &T) -> StdResult<Self> {
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

pub struct Keymap<'a, K, T, Ser = Bincode2>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// prefix of the newly constructed Storage
    namespace: &'a [u8],
    /// needed if any suffixes were added to the original namespace.
    prefix: Option<Vec<u8>>,
    length: Mutex<Option<u32>>,
    key_type: PhantomData<K>,
    item_type: PhantomData<T>,
    serialization_type: PhantomData<Ser>,
}

impl<'a, K: Serialize + DeserializeOwned, T: Serialize + DeserializeOwned, Ser: Serde>
    Keymap<'a, K, T, Ser>
{
    /// constructor
    pub const fn new(prefix: &'a [u8]) -> Self {
        Self {
            namespace: prefix,
            prefix: None,
            length: Mutex::new(None),
            key_type: PhantomData,
            item_type: PhantomData,
            serialization_type: PhantomData,
        }
    }

    /// This is used to produce a new Keymap. This can be used when you want to associate an Keymap to each user
    /// and you still get to define the Keymap as a static constant
    pub fn add_suffix(&self, suffix: &[u8]) -> Self {
        let suffix = to_length_prefixed(suffix);
        let prefix = self.prefix.as_deref().unwrap_or(self.namespace);
        let prefix = [prefix, suffix.as_slice()].concat();
        Self {
            namespace: self.namespace,
            prefix: Some(prefix),
            length: Mutex::new(None),
            key_type: self.key_type,
            item_type: self.item_type,
            serialization_type: self.serialization_type,
        }
    }
}

impl<'a, K: Serialize + DeserializeOwned, T: Serialize + DeserializeOwned, Ser: Serde>
    Keymap<'a, K, T, Ser>
{
    /// Serialize key
    fn serialize_key(&self, key: &K) -> StdResult<Vec<u8>> {
        Ser::serialize(key)
    }

    /// Deserialize key
    fn deserialize_key(&self, key_data: &[u8]) -> StdResult<K> {
        Ser::deserialize(key_data)
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
    fn _get_indexes(&self, storage: &dyn Storage, page: u32) -> StdResult<Vec<Vec<u8>>> {
        let indexes_key = [self.as_slice(), INDEXES, page.to_be_bytes().as_slice()].concat();
        let maybe_serialized = storage.get(&indexes_key);
        match maybe_serialized {
            Some(serialized) => Bincode2::deserialize(&serialized),
            None => Ok(vec![]),
        }
    }

    /// Set an indexes page
    fn _set_indexes_page(
        &self,
        storage: &mut dyn Storage,
        page: u32,
        indexes: &Vec<Vec<u8>>,
    ) -> StdResult<()> {
        let indexes_key = [self.as_slice(), INDEXES, page.to_be_bytes().as_slice()].concat();
        storage.set(&indexes_key, &Bincode2::serialize(indexes)?);
        Ok(())
    }

    /// user facing get function
    pub fn get(&self, storage: &dyn Storage, key: &K) -> Option<T> {
        if let Ok(internal_item) = self._get_from_key(storage, key) {
            internal_item.get_item().ok()
        } else {
            None
        }
    }

    /// internal item get function
 cosmwasm-v1.0
    fn _get_from_key(&self, storage: &dyn Storage, key: &K) -> StdResult<InternalItem<T, Ser>> {
        let key_vec = self.serialize_key(key)?;
        self.load_impl(storage, &key_vec)
    }

    /// user facing remove function
    pub fn remove(&self, storage: &mut dyn Storage, key: &K) -> StdResult<()> {
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
            return Err(StdError::generic_err(
                "Tried to remove, but hash not found - should never happen",
            ));
        }

        // if our object is the last item, then just remove it
        if len == 0 || len == removed_pos {
            indexes.pop();
            self._set_indexes_page(storage, page, &indexes)?;
            return Ok(());
        }

        // max page should use previous_len - 1 which is exactly the current len
        let max_page = _page_from_position(len);
        if max_page == page {
            // last page indexes is the same as indexes
            let last_key = indexes.pop().ok_or_else(|| {
                StdError::generic_err("Last item's key not found - should never happen")
            })?;
            // modify last item
            let mut last_internal_item = self.load_impl(storage, &last_key)?;
            last_internal_item.index_pos = removed_pos;
            self.save_impl(storage, &last_key, &last_internal_item)?;
            // save to indexes
            indexes[pos_in_indexes] = last_key;
            self._set_indexes_page(storage, page, &indexes)?;
        } else {
            let mut last_page_indexes = self._get_indexes(storage, max_page)?;
            let last_key = last_page_indexes.pop().ok_or_else(|| {
                StdError::generic_err("Last item's key not found - should never happen")
            })?;
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
                let page = _page_from_position(pos);
                // save the item
                let internal_item = InternalItem::new(pos, item)?;
                self.save_impl(storage, &key_vec, &internal_item)?;
                // add index
                let mut indexes = self._get_indexes(storage, page)?;
                indexes.push(key_vec);
                self._set_indexes_page(storage, page, &indexes)
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
                kind: "Out of bounds".to_string(),
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
                kind: "Out of bounds".to_string(),
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
    fn get_pairs_at_positions(
        &self,
        storage: &dyn Storage,
        start: u32,
        end: u32,
    ) -> StdResult<Vec<(K, T)>> {
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
                let item = self.load_impl(storage, key_vec)?.get_item()?;
                res.push((key, item));
            }
        }
        Ok(res)
    }

    /// gets a key from a specific position in indexes
    fn get_key_from_pos(&self, storage: &dyn Storage, pos: u32) -> StdResult<K> {
        let page = _page_from_position(pos);
        let indexes = self._get_indexes(storage, page)?;
        let index = pos % PAGE_SIZE;
        let key_vec = &indexes[index as usize];
        self.deserialize_key(key_vec)
    }

    /// gets a key from a specific position in indexes
    fn get_pair_from_pos(&self, storage: &dyn Storage, pos: u32) -> StdResult<(K, T)> {
        let page = _page_from_position(pos);
        let indexes = self._get_indexes(storage, page)?;
        let index = pos % PAGE_SIZE;
        let key_vec = &indexes[index as usize];
        let key = self.deserialize_key(key_vec)?;
        let item = self.load_impl(storage, key_vec)?.get_item()?;
        Ok((key, item))
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
    PrefixedTypedStorage<InternalItem<T, Ser>, Bincode2> for Keymap<'a, K, T, Ser>
{
    fn as_slice(&self) -> &[u8] {
        if let Some(prefix) = &self.prefix {
            prefix
        } else {
            self.namespace
        }
    }
}

/// An iterator over the keys of the Keymap.
pub struct KeyIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    keymap: &'a Keymap<'a, K, T, Ser>,
    storage: &'a dyn Storage,
    start: u32,
    end: u32,
    saved_indexes: Option<Vec<Vec<u8>>>,
    saved_index_page: Option<u32>,
    saved_back_indexes: Option<Vec<Vec<u8>>>,
    saved_back_index_page: Option<u32>,
}

impl<'a, K, T, Ser> KeyIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// constructor
    pub fn new(
        keymap: &'a Keymap<'a, K, T, Ser>,
        storage: &'a dyn Storage,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            keymap,
            storage,
            start,
            end,
            saved_indexes: None,
            saved_index_page: None,
            saved_back_indexes: None,
            saved_back_index_page: None,
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
        let res: Option<Self::Item>;
        if let (Some(page), Some(indexes)) = (&self.saved_index_page, &self.saved_indexes) {
            let current_page = _page_from_position(self.start);
            if *page == current_page {
                let current_idx = (self.start % PAGE_SIZE) as usize;
                if current_idx + 1 > indexes.len() {
                    res = None;
                } else {
                    let key_vec = &indexes[current_idx];
                    match self.keymap.deserialize_key(key_vec) {
                        Ok(key) => {
                            res = Some(Ok(key));
                        }
                        Err(e) => {
                            res = Some(Err(e));
                        }
                    }
                }
            } else {
                match self.keymap._get_indexes(self.storage, current_page) {
                    Ok(new_indexes) => {
                        let current_idx = (self.start % PAGE_SIZE) as usize;
                        if current_idx + 1 > new_indexes.len() {
                            res = None;
                        } else {
                            let key_vec = &new_indexes[current_idx];
                            match self.keymap.deserialize_key(key_vec) {
                                Ok(key) => {
                                    res = Some(Ok(key));
                                }
                                Err(e) => {
                                    res = Some(Err(e));
                                }
                            }
                        }
                        self.saved_index_page = Some(current_page);
                        self.saved_indexes = Some(new_indexes);
                    }
                    Err(_) => match self.keymap.get_key_from_pos(self.storage, self.start) {
                        Ok(key) => {
                            res = Some(Ok(key));
                        }
                        Err(_) => {
                            res = None;
                        }
                    },
                }
            }
        } else {
            let next_page = _page_from_position(self.start + 1);
            let current_page = _page_from_position(self.start);
            match self.keymap._get_indexes(self.storage, next_page) {
                Ok(next_index) => {
                    if current_page == next_page {
                        let current_idx = (self.start % PAGE_SIZE) as usize;
                        if current_idx + 1 > next_index.len() {
                            res = None;
                        } else {
                            let key_vec = &next_index[current_idx];
                            match self.keymap.deserialize_key(key_vec) {
                                Ok(key) => {
                                    res = Some(Ok(key));
                                }
                                Err(e) => {
                                    res = Some(Err(e));
                                }
                            }
                        }
                    } else {
                        match self.keymap.get_key_from_pos(self.storage, self.start) {
                            Ok(key) => {
                                res = Some(Ok(key));
                            }
                            Err(_) => {
                                res = None;
                            }
                        }
                    }
                    self.saved_index_page = Some(next_page);
                    self.saved_indexes = Some(next_index);
                }
                Err(_) => match self.keymap.get_key_from_pos(self.storage, self.start) {
                    Ok(key) => {
                        res = Some(Ok(key));
                    }
                    Err(_) => {
                        res = None;
                    }
                },
            }
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
        let res;
        if let (Some(page), Some(indexes)) = (&self.saved_back_index_page, &self.saved_back_indexes)
        {
            let current_page = _page_from_position(self.end);
            if *page == current_page {
                let current_idx = (self.end % PAGE_SIZE) as usize;
                if current_idx + 1 > indexes.len() {
                    res = None;
                } else {
                    let key_vec = &indexes[current_idx];
                    match self.keymap.deserialize_key(key_vec) {
                        Ok(key) => {
                            res = Some(Ok(key));
                        }
                        Err(e) => {
                            res = Some(Err(e));
                        }
                    }
                }
            } else {
                match self.keymap._get_indexes(self.storage, current_page) {
                    Ok(new_indexes) => {
                        let current_idx = (self.end % PAGE_SIZE) as usize;
                        if current_idx + 1 > new_indexes.len() {
                            res = None;
                        } else {
                            let key_vec = &new_indexes[current_idx];
                            match self.keymap.deserialize_key(key_vec) {
                                Ok(key) => {
                                    res = Some(Ok(key));
                                }
                                Err(e) => {
                                    res = Some(Err(e));
                                }
                            }
                        }
                        self.saved_back_index_page = Some(current_page);
                        self.saved_back_indexes = Some(new_indexes);
                    }
                    Err(_) => match self.keymap.get_key_from_pos(self.storage, self.end) {
                        Ok(key) => {
                            res = Some(Ok(key));
                        }
                        Err(_) => {
                            res = None;
                        }
                    },
                }
            }
        } else {
            let next_page = _page_from_position(self.end - 1);
            let current_page = _page_from_position(self.end);
            match self.keymap._get_indexes(self.storage, next_page) {
                Ok(next_index) => {
                    if current_page == next_page {
                        let current_idx = (self.end % PAGE_SIZE) as usize;
                        if current_idx + 1 > next_index.len() {
                            res = None;
                        } else {
                            let key_vec = &next_index[current_idx];
                            match self.keymap.deserialize_key(key_vec) {
                                Ok(key) => {
                                    res = Some(Ok(key));
                                }
                                Err(e) => {
                                    res = Some(Err(e));
                                }
                            }
                        }
                    } else {
                        match self.keymap.get_key_from_pos(self.storage, self.end) {
                            Ok(key) => {
                                res = Some(Ok(key));
                            }
                            Err(_) => {
                                res = None;
                            }
                        }
                    }
                    self.saved_back_index_page = Some(next_page);
                    self.saved_back_indexes = Some(next_index);
                }
                Err(_) => match self.keymap.get_key_from_pos(self.storage, self.end) {
                    Ok(key) => {
                        res = Some(Ok(key));
                    }
                    Err(_) => {
                        res = None;
                    }
                },
            }
        }
        res
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
impl<'a, K, T, Ser> ExactSizeIterator for KeyIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
}

// ===============================================================================================

/// An iterator over the (key, item) pairs of the Keymap. Less efficient than just iterating over keys.
pub struct KeyItemIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    keymap: &'a Keymap<'a, K, T, Ser>,
    storage: &'a dyn Storage,
    start: u32,
    end: u32,
    saved_indexes: Option<Vec<Vec<u8>>>,
    saved_index_page: Option<u32>,
    saved_back_indexes: Option<Vec<Vec<u8>>>,
    saved_back_index_page: Option<u32>,
}

impl<'a, K, T, Ser> KeyItemIter<'a, K, T, Ser>
where
    K: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
    Ser: Serde,
{
    /// constructor
    pub fn new(
        keymap: &'a Keymap<'a, K, T, Ser>,
        storage: &'a dyn Storage,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            keymap,
            storage,
            start,
            end,
            saved_indexes: None,
            saved_index_page: None,
            saved_back_indexes: None,
            saved_back_index_page: None,
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
        let res: Option<Self::Item>;
        if let (Some(page), Some(indexes)) = (&self.saved_index_page, &self.saved_indexes) {
            let current_page = _page_from_position(self.start);
            if *page == current_page {
                let current_idx = (self.start % PAGE_SIZE) as usize;
                if current_idx + 1 > indexes.len() {
                    res = None;
                } else {
                    let key_vec = &indexes[current_idx];
                    match self.keymap.deserialize_key(key_vec) {
                        Ok(key) => {
                            let item = self.keymap.get(self.storage, &key)?;
                            res = Some(Ok((key, item)));
                        }
                        Err(e) => {
                            res = Some(Err(e));
                        }
                    }
                }
            } else {
                match self.keymap._get_indexes(self.storage, current_page) {
                    Ok(new_indexes) => {
                        let current_idx = (self.start % PAGE_SIZE) as usize;
                        if current_idx + 1 > new_indexes.len() {
                            res = None;
                        } else {
                            let key_vec = &new_indexes[current_idx];
                            match self.keymap.deserialize_key(key_vec) {
                                Ok(key) => {
                                    let item = self.keymap.get(self.storage, &key)?;
                                    res = Some(Ok((key, item)));
                                }
                                Err(e) => {
                                    res = Some(Err(e));
                                }
                            }
                        }
                        self.saved_index_page = Some(current_page);
                        self.saved_indexes = Some(new_indexes);
                    }
                    Err(_) => match self.keymap.get_pair_from_pos(self.storage, self.start) {
                        Ok(pair) => {
                            res = Some(Ok(pair));
                        }
                        Err(_) => {
                            res = None;
                        }
                    },
                }
            }
        } else {
            let next_page = _page_from_position(self.start + 1);
            let current_page = _page_from_position(self.start);
            match self.keymap._get_indexes(self.storage, next_page) {
                Ok(next_index) => {
                    if current_page == next_page {
                        let current_idx = (self.start % PAGE_SIZE) as usize;
                        if current_idx + 1 > next_index.len() {
                            res = None;
                        } else {
                            let key_vec = &next_index[current_idx];
                            match self.keymap.deserialize_key(key_vec) {
                                Ok(key) => {
                                    let item = self.keymap.get(self.storage, &key)?;
                                    res = Some(Ok((key, item)));
                                }
                                Err(e) => {
                                    res = Some(Err(e));
                                }
                            }
                        }
                    } else {
                        match self.keymap.get_pair_from_pos(self.storage, self.start) {
                            Ok(pair) => {
                                res = Some(Ok(pair));
                            }
                            Err(_) => {
                                res = None;
                            }
                        }
                    }
                    self.saved_index_page = Some(next_page);
                    self.saved_indexes = Some(next_index);
                }
                Err(_) => match self.keymap.get_pair_from_pos(self.storage, self.start) {
                    Ok(pair) => {
                        res = Some(Ok(pair));
                    }
                    Err(_) => {
                        res = None;
                    }
                },
            }
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
        let res;
        if let (Some(page), Some(indexes)) = (&self.saved_back_index_page, &self.saved_back_indexes)
        {
            let current_page = _page_from_position(self.end);
            if *page == current_page {
                let current_idx = (self.end % PAGE_SIZE) as usize;
                if current_idx + 1 > indexes.len() {
                    res = None;
                } else {
                    let key_vec = &indexes[current_idx];
                    match self.keymap.deserialize_key(key_vec) {
                        Ok(key) => {
                            let item = self.keymap.get(self.storage, &key)?;
                            res = Some(Ok((key, item)));
                        }
                        Err(e) => {
                            res = Some(Err(e));
                        }
                    }
                }
            } else {
                match self.keymap._get_indexes(self.storage, current_page) {
                    Ok(new_indexes) => {
                        let current_idx = (self.end % PAGE_SIZE) as usize;
                        if current_idx + 1 > new_indexes.len() {
                            res = None;
                        } else {
                            let key_vec = &new_indexes[current_idx];
                            match self.keymap.deserialize_key(key_vec) {
                                Ok(key) => {
                                    let item = self.keymap.get(self.storage, &key)?;
                                    res = Some(Ok((key, item)));
                                }
                                Err(e) => {
                                    res = Some(Err(e));
                                }
                            }
                        }
                        self.saved_back_index_page = Some(current_page);
                        self.saved_back_indexes = Some(new_indexes);
                    }
                    Err(_) => match self.keymap.get_pair_from_pos(self.storage, self.end) {
                        Ok(pair) => {
                            res = Some(Ok(pair));
                        }
                        Err(_) => {
                            res = None;
                        }
                    },
                }
            }
        } else {
            let next_page = _page_from_position(self.end - 1);
            let current_page = _page_from_position(self.end);
            match self.keymap._get_indexes(self.storage, next_page) {
                Ok(next_index) => {
                    if current_page == next_page {
                        let current_idx = (self.end % PAGE_SIZE) as usize;
                        if current_idx + 1 > next_index.len() {
                            res = None;
                        } else {
                            let key_vec = &next_index[current_idx];
                            match self.keymap.deserialize_key(key_vec) {
                                Ok(key) => {
                                    let item = self.keymap.get(self.storage, &key)?;
                                    res = Some(Ok((key, item)));
                                }
                                Err(e) => {
                                    res = Some(Err(e));
                                }
                            }
                        }
                    } else {
                        match self.keymap.get_pair_from_pos(self.storage, self.end) {
                            Ok(pair) => {
                                res = Some(Ok(pair));
                            }
                            Err(_) => {
                                res = None;
                            }
                        }
                    }
                    self.saved_back_index_page = Some(next_page);
                    self.saved_back_indexes = Some(next_index);
                }
                Err(_) => match self.keymap.get_pair_from_pos(self.storage, self.end) {
                    Ok(pair) => {
                        res = Some(Ok(pair));
                    }
                    Err(_) => {
                        res = None;
                    }
                },
            }
        }
        res
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
            keymap.insert(&mut storage, &key, &i)?;
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
            keymap.insert(&mut storage, &i, &i)?;
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
            keymap.insert(&mut storage, &key, &i)?;
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
            keymap.insert(&mut storage, &(i as i32), &i)?;
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

        keymap.insert(&mut storage, &b"key1".to_vec(), &foo1)?;
        keymap.insert(&mut storage, &b"key2".to_vec(), &foo2)?;

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

        keymap.insert(&mut storage, &b"key1".to_vec(), &foo1)?;
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

        keymap.insert(&mut storage, &b"key1".to_vec(), &foo1)?;
        keymap.insert(&mut storage, &b"key2".to_vec(), &foo2)?;

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

        keymap.insert(&mut storage, &key1, &foo1)?;
        keymap.insert(&mut storage, &key2, &foo2)?;

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

        keymap.insert(&mut storage, &b"key1".to_vec(), &foo1)?;
        keymap.insert(&mut storage, &b"key1".to_vec(), &foo2)?;

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
        keymap.insert(&mut storage, &"key1".to_string(), &foo1)?;
        keymap.insert(&mut storage, &"key2".to_string(), &foo2)?;

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

        assert!(keymap.length.lock().unwrap().eq(&None));
        assert_eq!(keymap.get_len(&storage)?, 0);
        assert!(keymap.length.lock().unwrap().eq(&Some(0)));

        let key1 = "k1".to_string();
        let key2 = "k2".to_string();

        keymap.insert(&mut storage, &key1, &foo1)?;
        assert_eq!(keymap.get_len(&storage)?, 1);
        assert!(keymap.length.lock().unwrap().eq(&Some(1)));

        // add another item
        keymap.insert(&mut storage, &key2, &foo2)?;
        assert_eq!(keymap.get_len(&storage)?, 2);
        assert!(keymap.length.lock().unwrap().eq(&Some(2)));

        // remove item and check length
        keymap.remove(&mut storage, &key1)?;
        assert_eq!(keymap.get_len(&storage)?, 1);
        assert!(keymap.length.lock().unwrap().eq(&Some(1)));

        // override item (should not change length)
        keymap.insert(&mut storage, &key2, &foo1)?;
        assert_eq!(keymap.get_len(&storage)?, 1);
        assert!(keymap.length.lock().unwrap().eq(&Some(1)));

        // remove item and check length
        keymap.remove(&mut storage, &key2)?;
        assert_eq!(keymap.get_len(&storage)?, 0);
        assert!(keymap.length.lock().unwrap().eq(&Some(0)));

        Ok(())
    }
}
