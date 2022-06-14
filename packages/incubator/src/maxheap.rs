//! A "max heap store" is a storage wrapper that implements a binary tree maxheap data structure.
//! https://en.wikipedia.org/wiki/Min-max_heap
//! Implementation based on https://algorithmtutor.com/Data-Structures/Tree/Binary-Heaps/
//!
//! Insertion O(log n)
//! Remove max O(log n)
//!
use std::convert::TryInto;
use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};
use std::cmp::PartialOrd;

use cosmwasm_std::{ReadonlyStorage, StdError, StdResult, Storage};

use secret_toolkit_serialization::{Bincode2, Serde};

const LEN_KEY: &[u8] = b"len";

// Mutable maxheap store

/// A type allowing both reads from and writes to the maxheap store at a given storage location.
#[derive(Debug)]
pub struct MaxHeapStoreMut<'a, T, S, Ser = Bincode2>
where
    T: Serialize + DeserializeOwned + PartialOrd,
    S: Storage,
    Ser: Serde,
{
    storage: &'a mut S,
    item_type: PhantomData<*const T>,
    serialization_type: PhantomData<*const Ser>,
    len: u32,
}

impl<'a, T, S> MaxHeapStoreMut<'a, T, S, Bincode2>
where
    T: Serialize + DeserializeOwned + PartialOrd,
    S: Storage,
{
    /// Try to use the provided storage as an MaxHeapStore. If it doesn't seem to be one, then
    /// initialize it as one.
    ///
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_or_create(storage: &'a mut S) -> StdResult<Self> {
        MaxHeapStoreMut::attach_or_create_with_serialization(storage, Bincode2)
    }

    /// Try to use the provided storage as an MaxHeapStore.
    ///
    /// Returns None if the provided storage doesn't seem like an MaxHeapStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach(storage: &'a mut S) -> Option<StdResult<Self>> {
        MaxHeapStoreMut::attach_with_serialization(storage, Bincode2)
    }
}

impl<'a, T, S, Ser> MaxHeapStoreMut<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned + PartialOrd,
    S: Storage,
    Ser: Serde,
{
    /// Try to use the provided storage as an MaxHeapStore. If it doesn't seem to be one, then
    /// initialize it as one. This method allows choosing the serialization format you want to use.
    ///
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_or_create_with_serialization(storage: &'a mut S, _ser: Ser) -> StdResult<Self> {
        if let Some(len_vec) = storage.get(LEN_KEY) {
            Self::new(storage, &len_vec)
        } else {
            let len_vec = 0_u32.to_be_bytes();
            storage.set(LEN_KEY, &len_vec);
            Self::new(storage, &len_vec)
        }
    }

    /// Try to use the provided storage as an MaxHeapStore.
    /// This method allows choosing the serialization format you want to use.
    ///
    /// Returns None if the provided storage doesn't seem like an MaxHeapStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_with_serialization(storage: &'a mut S, _ser: Ser) -> Option<StdResult<Self>> {
        let len_vec = storage.get(LEN_KEY)?;
        Some(Self::new(storage, &len_vec))
    }

    fn new(storage: &'a mut S, len_vec: &[u8]) -> StdResult<Self> {
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

    pub fn storage(&mut self) -> &mut S {
        self.storage
    }

    pub fn readonly_storage(&self) -> &S {
        self.storage
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
            return Err(StdError::generic_err("MaxHeapStore access out of bounds"));
        }
        self.set_at_unchecked(pos, item)
    }

    fn set_at_unchecked(&mut self, pos: u32, item: &T) -> StdResult<()> {
        let serialized = Ser::serialize(item)?;
        self.storage.set(&pos.to_be_bytes(), &serialized);
        Ok(())
    }

    /// return index of the parent node
    fn parent(&self, idx: u32) -> u32 {
        (idx - 1) / 2
    }

    /// return index of the left child
    fn left_child(&self, idx: u32) -> u32 {
        2 * idx + 1
    }

    /// return index of the right child
    fn right_child(&self, idx: u32) -> u32 {
        2 * idx + 2
    }

    /// inserts an item into the heap at the correct position O(log n)
    pub fn insert(&mut self, item: &T) -> StdResult<()> {
        self.set_at_unchecked(self.len, item)?;
        self.set_length(self.len + 1);

        let mut i = self.len - 1;
        while i != 0 {
            let parent_i = self.parent(i);
            let parent_val = self.get_at_unchecked(parent_i)?;
            let val = self.get_at_unchecked(i)?;
            if parent_val < val {
                // swap
                self.set_at_unchecked(parent_i, item)?;
                self.set_at_unchecked(i, &parent_val)?;
            }
            i = parent_i;
        }

        Ok(())
    }

    /// moves the item at position idx into its correct position
    fn max_heapify(&mut self, idx: u32) -> StdResult<()> {
        // find left child node
        let left = self.left_child(idx);

        // find the right child node
        let right = self.right_child(idx);

        // find the largest among 3 nodes
        let mut largest = idx;

        // check if the left node is larger than the current node
        if left <= self.len() && self.get_at_unchecked(left)? > self.get_at_unchecked(largest)? {
            largest = left;
        }

        // check if the right node is larger than the current node
        if right <= self.len() && self.get_at_unchecked(right)? > self.get_at_unchecked(largest)? {
            largest = right;
        }

        // swap the largest node with the current node
        // and repeat this process until the current node is larger than
        // the right and the left node
        if largest != idx {
            let temp: T = self.get_at_unchecked(idx)?;
            self.set_at_unchecked(idx, &self.get_at_unchecked(largest)?)?;
            self.set_at_unchecked(largest, &temp)?;
            self.max_heapify(largest)?;
        }

        Ok(())
    }

    /// remove the max item and returns it
    pub fn remove(&mut self) -> StdResult<T> {
        if let Some(len) = self.len.checked_sub(1) {
            let max_item = self.get_max()?;

            // replace the first item with the last item
            self.set_at_unchecked(0, &self.get_at_unchecked(len)?)?;
            self.set_length(len);

            // maintain the heap property by heapifying the first item
            self.max_heapify(0)?;

            Ok(max_item)
        } else {
            Err(StdError::generic_err("Can not pop from empty MaxHeap"))
        }
    }

    /// returns the maximum item in heap
    pub fn get_max(&self) -> StdResult<T> {
        self.as_readonly().get_max()
    }

    /// Set the length of the collection
    fn set_length(&mut self, len: u32) {
        self.storage.set(LEN_KEY, &len.to_be_bytes());
        self.len = len;
    }

    /// Gain access to the implementation of the immutable methods
    fn as_readonly(&self) -> MaxHeapStore<T, S, Ser> {
        MaxHeapStore {
            storage: self.storage,
            item_type: self.item_type,
            serialization_type: self.serialization_type,
            len: self.len,
        }
    }
}

// Readonly maxheap store

/// A type allowing only reads from an max heap store. useful in the context of queries.
#[derive(Debug)]
pub struct MaxHeapStore<'a, T, S, Ser = Bincode2>
where
    T: Serialize + DeserializeOwned + PartialOrd,
    S: ReadonlyStorage,
    Ser: Serde,
{
    storage: &'a S,
    item_type: PhantomData<*const T>,
    serialization_type: PhantomData<*const Ser>,
    len: u32,
}

impl<'a, T, S> MaxHeapStore<'a, T, S, Bincode2>
where
    T: Serialize + DeserializeOwned + PartialOrd,
    S: ReadonlyStorage,
{
    /// Try to use the provided storage as a MaxHeapStore.
    ///
    /// Returns None if the provided storage doesn't seem like a MaxHeapStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach(storage: &'a S) -> Option<StdResult<Self>> {
        MaxHeapStore::attach_with_serialization(storage, Bincode2)
    }
}

impl<'a, T, S, Ser> MaxHeapStore<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned + PartialOrd,
    S: ReadonlyStorage,
    Ser: Serde,
{
    /// Try to use the provided storage as an MaxHeapStore.
    /// This method allows choosing the serialization format you want to use.
    ///
    /// Returns None if the provided storage doesn't seem like an MaxHeapStore.
    /// Returns Err if the contents of the storage can not be parsed.
    pub fn attach_with_serialization(storage: &'a S, _ser: Ser) -> Option<StdResult<Self>> {
        let len_vec = storage.get(LEN_KEY)?;
        Some(MaxHeapStore::new(storage, len_vec))
    }

    fn new(storage: &'a S, len_vec: Vec<u8>) -> StdResult<Self> {
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

    pub fn readonly_storage(&self) -> &S {
        self.storage
    }

    /// Get the value stored at a given position.
    ///
    /// # Errors
    /// Will return an error if pos is out of bounds or if an item is not found.
    pub fn get_at(&self, pos: u32) -> StdResult<T> {
        if pos >= self.len {
            return Err(StdError::generic_err("MaxHeapStore access out of bounds"));
        }
        self.get_at_unchecked(pos)
    }

    fn get_at_unchecked(&self, pos: u32) -> StdResult<T> {
        let serialized = self.storage.get(&pos.to_be_bytes()).ok_or_else(|| {
            StdError::generic_err(format!("No item in MaxHeapStore at position {}", pos))
        })?;
        Ser::deserialize(&serialized)
    }

    /// returns the maximum item in heap
    pub fn get_max(&self) -> StdResult<T> {
        self.get_at(0)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::MockStorage;
    use serde::Deserialize;

    use cosmwasm_std::HumanAddr;
    use secret_toolkit_serialization::Json;
    use std::cmp::Ordering;

    use super::*;

    #[test]
    fn test_insert_remove() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let mut heap_store = MaxHeapStoreMut::attach_or_create(&mut storage)?;
        heap_store.insert(&1234)?;
        heap_store.insert(&2143)?;
        heap_store.insert(&4321)?;
        heap_store.insert(&3412)?;
        heap_store.insert(&2143)?;

        assert_eq!(heap_store.remove(), Ok(4321));
        assert_eq!(heap_store.remove(), Ok(3412));
        assert_eq!(heap_store.remove(), Ok(2143));
        assert_eq!(heap_store.remove(), Ok(2143));
        assert_eq!(heap_store.remove(), Ok(1234));
        assert!(heap_store.remove().is_err());

        heap_store.insert(&1234)?;
        assert_eq!(heap_store.remove(), Ok(1234));

        Ok(())
    }

    #[test]
    fn test_custom_ord() -> StdResult<()> {
        #[derive(Serialize, Deserialize, Clone, Debug, Eq)]
        pub struct Tx {
            address: HumanAddr,
            amount: u128,
        }

        impl PartialOrd for Tx {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for Tx {
            fn cmp(&self, other: &Self) -> Ordering {
                self.amount.cmp(&other.amount)
            }
        }

        impl PartialEq for Tx {
            fn eq(&self, other: &Self) -> bool {
                self.amount == other.amount
            }
        }

        let mut storage = MockStorage::new();
        let mut heap_store = MaxHeapStoreMut::attach_or_create(&mut storage)?;

        heap_store.insert(&Tx {
            address: HumanAddr("address1".to_string()),
            amount: 200,
        })?;
        heap_store.insert(&Tx {
            address: HumanAddr("address2".to_string()),
            amount: 100,
        })?;
        heap_store.insert(&Tx {
            address: HumanAddr("address3".to_string()),
            amount: 400,
        })?;
        heap_store.insert(&Tx {
            address: HumanAddr("address4".to_string()),
            amount: 300,
        })?;
        heap_store.insert(&Tx {
            address: HumanAddr("address5".to_string()),
            amount: 50,
        })?;

        assert_eq!(
            heap_store.remove(),
            Ok(Tx {
                address: HumanAddr("address3".to_string()),
                amount: 400,
            })
        );
        assert_eq!(
            heap_store.remove(),
            Ok(Tx {
                address: HumanAddr("address4".to_string()),
                amount: 300,
            })
        );
        assert_eq!(
            heap_store.remove(),
            Ok(Tx {
                address: HumanAddr("address1".to_string()),
                amount: 200,
            })
        );
        assert_eq!(
            heap_store.remove(),
            Ok(Tx {
                address: HumanAddr("address2".to_string()),
                amount: 100,
            })
        );
        assert_eq!(
            heap_store.remove(),
            Ok(Tx {
                address: HumanAddr("address5".to_string()),
                amount: 50,
            })
        );
        Ok(())
    }

    #[test]
    fn test_attach_to_wrong_location() {
        let mut storage = MockStorage::new();
        assert!(MaxHeapStore::<u8, _>::attach(&storage).is_none());
        assert!(MaxHeapStoreMut::<u8, _>::attach(&mut storage).is_none());
    }

    #[test]
    fn test_serializations() -> StdResult<()> {
        // Check the default behavior is Bincode2
        let mut storage = MockStorage::new();

        let mut heap_store = MaxHeapStoreMut::attach_or_create(&mut storage)?;
        heap_store.insert(&1234)?;

        let bytes = heap_store.readonly_storage().get(&0_u32.to_be_bytes());
        assert_eq!(bytes, Some(vec![210, 4, 0, 0]));

        // Check that overriding the serializer with Json works
        let mut storage = MockStorage::new();
        let mut heap_store =
            MaxHeapStoreMut::attach_or_create_with_serialization(&mut storage, Json)?;
        heap_store.insert(&1234)?;
        let bytes = heap_store.readonly_storage().get(&0_u32.to_be_bytes());
        assert_eq!(bytes, Some(b"1234".to_vec()));

        Ok(())
    }
}
