use std::any::type_name;
use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{ReadonlyStorage, StdError, StdResult, Storage};

use secret_toolkit_serialization::{Bincode2, Serde};

pub struct TypedStoreMut<'a, T, S, Ser = Bincode2>
where
    T: Serialize + DeserializeOwned,
    S: Storage,
    Ser: Serde,
{
    storage: &'a mut S,
    item_type: PhantomData<*const T>,
    serialization_type: PhantomData<*const Ser>,
}

impl<'a, T, S> TypedStoreMut<'a, T, S, Bincode2>
where
    T: Serialize + DeserializeOwned,
    S: Storage,
{
    pub fn attach(storage: &'a mut S) -> Self {
        Self::attach_with_serialization(storage, Bincode2)
    }
}

impl<'a, T, S, Ser> TypedStoreMut<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned,
    S: Storage,
    Ser: Serde,
{
    pub fn attach_with_serialization(storage: &'a mut S, _serialization: Ser) -> Self {
        Self {
            storage,
            serialization_type: PhantomData,
            item_type: PhantomData,
        }
    }

    pub fn store(&mut self, key: &[u8], item: &T) -> StdResult<()> {
        self.storage.set(key, &Ser::serialize(item)?);
        Ok(())
    }

    pub fn remove(&mut self, key: &[u8]) {
        self.storage.remove(key);
    }

    fn as_readonly(&self) -> TypedStore<T, S, Ser> {
        TypedStore {
            storage: self.storage,
            item_type: self.item_type,
            serialization_type: self.serialization_type,
        }
    }

    pub fn load(&self, key: &[u8]) -> StdResult<T> {
        self.as_readonly().load(key)
    }

    pub fn may_load(&self, key: &[u8]) -> StdResult<Option<T>> {
        self.as_readonly().may_load(key)
    }
}

pub struct TypedStore<'a, T, S, Ser = Bincode2>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{
    storage: &'a S,
    item_type: PhantomData<*const T>,
    serialization_type: PhantomData<*const Ser>,
}

impl<'a, T, S> TypedStore<'a, T, S, Bincode2>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
{
    pub fn attach(storage: &'a S) -> Self {
        Self::attach_with_serialization(storage, Bincode2)
    }
}

impl<'a, T, S, Ser> TypedStore<'a, T, S, Ser>
where
    T: Serialize + DeserializeOwned,
    S: ReadonlyStorage,
    Ser: Serde,
{
    pub fn attach_with_serialization(storage: &'a S, _serialization: Ser) -> Self {
        Self {
            storage,
            serialization_type: PhantomData,
            item_type: PhantomData,
        }
    }

    pub fn load(&self, key: &[u8]) -> StdResult<T> {
        let maybe_serialized = self.storage.get(key);
        let serialized = maybe_serialized.ok_or_else(|| StdError::not_found(type_name::<T>()))?;
        Ser::deserialize(&serialized)
    }

    pub fn may_load(&self, key: &[u8]) -> StdResult<Option<T>> {
        match self.storage.get(key) {
            Some(serialized) => Ser::deserialize(&serialized).map(Some),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use cosmwasm_std::testing::MockStorage;

    use secret_toolkit_serialization::Json;

    use super::*;

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        string: String,
        number: i32,
    }

    #[test]
    fn test_typed_store() -> StdResult<()> {
        let mut storage = MockStorage::new();

        let mut typed_store_mut = TypedStoreMut::attach(&mut storage);
        let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        let foo2 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };
        typed_store_mut.store(b"key1", &foo1)?;
        typed_store_mut.store(b"key2", &foo2)?;

        let read_foo1 = typed_store_mut.load(b"key1")?;
        let read_foo2 = typed_store_mut.load(b"key2")?;

        assert_eq!(foo1, read_foo1);
        assert_eq!(foo2, read_foo2);

        // show that it loads foo1 before removal
        let before_remove_foo1 = typed_store_mut.may_load(b"key1")?;
        assert!(before_remove_foo1.is_some());
        assert_eq!(foo1, before_remove_foo1.unwrap());
        // and returns None after removal
        typed_store_mut.remove(b"key1");
        let removed_foo1 = typed_store_mut.may_load(b"key1")?;
        assert!(removed_foo1.is_none());

        // show what happens when reading from keys that have not been set yet.
        assert!(typed_store_mut.load(b"key3").is_err());
        assert!(typed_store_mut.may_load(b"key3")?.is_none());

        // Try to load it with the wrong format
        let typed_store = TypedStore::<i32, _, _>::attach_with_serialization(&storage, Json);
        match typed_store.load(b"key2") {
            Err(StdError::ParseErr { target, msg, .. })
                if target == "i32" && msg == "Invalid type" => {}
            other => panic!("unexpected value: {:?}", other),
        }

        Ok(())
    }
}
