# Secret Contract Development Toolkit - Storage Tools

⚠️ This is a sub-package of the `secret-toolkit` package. Please see its crate page for more context. You need Rust 1.63+ to compile this package.

This package contains many tools related to storage access patterns. This readme file assumes basic familiarity with basic cosmwasm storage, [click here to learn about this](https://docs.scrt.network/secret-network-documentation/development/secret-contracts/storage).

## **How to Import This Subpackage**

To import this package, add one of the following lines to your `Cargo.toml` file

```toml
secret-toolkit = { version = "0.5", default-features = false, features = ["utils", "storage", "serialization"] }
```

for the release versions (when it is updated to cosmwasm 1.0), or

```toml
secret-toolkit = { git = "https://github.com/scrtlabs/secret-toolkit", branch = "master", default-features = false, features = ["utils", "storage", "serialization"]}
```

for the github version. We also import the `serialization` feature in case we want to switch to using Json instead of Bincode2 to serialize/deserialize data.

## **Storage Objects**

### **Item**

This is the simplest storage object in this toolkit. It based on the similarly named Item from cosmwasm-storage-plus. Item allows the user to specify the type of the object being stored and the serialization/deserialization method used to store it (default being Bincode2). **One can think of the Item struct as a wrapper for the storage key.** Note that you want to use Json to serde an enum or any struct that stores an enum (except for the standard Option enum), because Bincode2 somehow uses floats during the deserialization of enums. This is why other cosmwasm chains don't use Bincode2 at all, however, you gain some performance when you can use it.

#### **Initialize**

This object is meant to be initialized as a static constant in `state.rs`. However, it would also work perfectly fine if it was initialized during run time with a variable key (in this case though, you'd have to remind it what type of object is stored and its serde). Import it using the following lines:

```ignore
use secret_toolkit::storage::{Item}
```

And initialize it using the following lines:

```ignore
pub static OWNER: Item<Addr> = Item::new(b"owner");
```

This uses Bincode2 to serde Addr by default. To specify the Serde algorithm as Json, first import it from `secret-toolkit::serialization`

```ignore
use secret_toolkit::serialization::{Bincode2, Json};
```

then

```ignore
pub static SOME_ENUM: Item<SomeEnum, Json> = Item::new(b"some_enum");
```

#### **Read/Write**

The way to read/write to/from strorage is to use its methods. These methods are `save`, `load`, `may_load`, `remove`, `update`. Here is an example usecase for each in execution inside `contract.rs`:

```ignore
// The compiler knows that owner_addr is Addr
let owner_addr = OWNER.load(&deps.storage)?;
```

```ignore
OWNER.save(&mut deps.storage, &env.message.sender)?;
```

```ignore
// The compiler knows that may_addr is Option<Addr>
let may_addr = OWNER.may_load(&deps.storage)?;
```

```ignore
// The compiler knows that may_addr is Option<Addr>
let may_addr = OWNER.remove(&mut deps.storage)?;
```

```ignore
// The compiler knows that may_addr is Option<Addr>
let may_addr = OWNER.update(&mut deps.storage, |_x| Ok(env.message.sender))?;
```

### **AppendStore**

AppendStore is meant replicate the functionality of an append list in a cosmwasm efficient manner. The length of the list is stored and used to pop/push items to the list. It also has a method to create a read only iterator.

This storage object also has the method `remove` to remove a stored object from an arbitrary position in the list, but this can be exteremely inefficient.

> ❗ Removing a storage object further from the tail gets increasingly inefficient. We recommend you use `pop` and `push` whenever possible.

The same conventions from `Item` also apply here, that is:

1. AppendStore has to be told the type of the stored objects. And the serde optionally.
2. Every methods needs it's own reference to `deps.storage`.

#### **Initialize**

To import and intialize this storage object as a static constant in `state.rs`, do the following:

```ignore
use secret_toolkit::storage::{AppendStore}
```

```ignore
pub static COUNT_STORE: AppendStore<i32> = AppendStore::new(b"count");
```

> ❗ Initializing the object as const instead of static will also work but be less efficient since the variable won't be able to cache length data.

Often times we need these storage objects to be associated to a user address or some other key that is variable. In this case, you need not initialize a completely new AppendStore inside `contract.rs`. Instead, you can create a new AppendStore by adding a suffix to an already existing AppendStore. This has the benefit of preventing you from having to rewrite the signature of the AppendStore. For example

```ignore
// The compiler knows that user_count_store is AppendStore<i32, Bincode2>
let user_count_store = COUNT_STORE.add_suffix(env.message.sender.to_string().as_bytes());
```

Sometimes when iterating these objects, we may want to load the next `n` objects at once. This may be prefered if the objects we are iterating over are cheap to store or if we know that multiple objects will need to be accessed back to back. In such cases we may want to change the internal indexing size (default of 1). We do this in `state.rs`:

```ignore
pub static COUNT_STORE: AppendStore<i32> = AppendStore::new_with_page_size(b"count", 5);
```

#### **Read/Write**

The main user facing methods to read/write to AppendStore are `pop`, `push`, `get_len`, `set_at` (which replaces data at a position within the length bound), `clear` (which deletes all data in the storage), `remove` (which removes an item in an arbitrary position, this is very inefficient). An extensive list of examples of these being used can be found inside the unit tests of AppendStore found in `append_store.rs`.

#### **Iterator**

AppendStore also implements a readonly iterator feature. This feature is also used to create a paging wrapper method called `paging`. The way you create the iterator is:

```ignore
let iter = user_count_store.iter(&deps.storage)?;
```

More examples can be found in the unit tests. And the paging wrapper is used in the following manner:

```ignore
let start_page: u32 = 0;
let page_size: u32 = 5;
// The compiler knows that values is Vec<i32>
let values = user_count_store.paging(&deps.storage, start_page, page_size)?;
```

### **DequeStore**

This is a storage wrapper based on AppendStore that replicates a double ended list. This storage object allows the user to efficiently pop/push items to either end of the list.

#### **Init**

To import and intialize this storage object as a static constant in `state.rs`, do the following:

```ignore
use secret_toolkit::storage::{DequeStore}
```

```ignore
pub static COUNT_STORE: DequeStore<i32> = DequeStore::new(b"count");
```

> ❗ Initializing the object as const instead of static will also work but be less efficient since the variable won't be able to cache length data.

> new_with_page_size works similarly to that of AppendStore's

#### **Read/Write**

The main user facing methods to read/write to DequeStore are `pop_back`, `pop_front`, `push_back`, `push_front`, `get_len`, `get_off`, `set_at` (which replaces data at a position within the length bound), `clear` (which deletes all data in the storage), `remove` (which removes an item in an arbitrary position, this is very inefficient). An extensive list of examples of these being used can be found inside the unit tests of DequeStore found in `deque_store.rs`.

#### **Iterator**

This is exactly same as that of AppendStore.

### **Keymap**

This hashmap-like storage structure allows the user to use generic typed keys to store objects. Allows iteration with paging over keys and/or items (without guaranteed ordering, although the order of insertion is preserved until you start removing objects).
An example use-case for such a structure is if you want to contain a large amount of votes, deposits, or bets and iterate over them at some time in the future.
Since iterating over large amounts of data at once may be prohibitive, this structure allows you to specify the amount of data that will
be returned in each page.

#### **Init**

To import and intialize this storage object as a static constant in `state.rs`, do the following:

```ignore
use secret_toolkit::storage::{Keymap, KeymapBuilder}
```

```ignore
pub static ADDR_VOTE: Keymap<Addr, Foo> = Keymap::new(b"vote");
pub static BET_STORE: Keymap<u32, BetInfo> = Keymap::new(b"bet");
```

> ❗ Initializing the object as const instead of static will also work but be less efficient since the variable won't be able to cache length data.

You can use Json serde algorithm by changing the signature to `Keymap<Addr, Uint128, Json>`, similar to all the other storage objects above. However, keep in mind that the Serde algorthm is used to serde both the stored object (`Uint128`) AND the key (`Addr`).

If you need to associate a keymap to a user address (or any other variable), then you can also do this using the `.add_suffix` method.

For example suppose that in your contract, a user can make multiple bets. Then, you'd want a Keymap to be associated to each user. You would achieve this my doing the following during execution in `contract.rs`.

```ignore
// The compiler knows that user_bet_store is AppendStore<u32, BetInfo>
let user_count_store = BET_STORE.add_suffix(env.message.sender.to_string().as_bytes());
```

#### **Advanced Init**

It is also possible to modify some of the configuration settings of the Keymap structure so that it suits better to a specific usecase. In this case, we use a struct called `KeymapBuilder` to build a keymap with specialized config. Currently, we can use KeymapBuilder to modify two attributes of keymaps.

One is to be able disable the iterator feature altogether using `.without_iter()`. This basically turns a keymap into a typed PrefixedStorage, but it also saves a ton of gas by not storing the keys and the length of the keymap.

The other feature is to modify the page size of the internal indexer (only if the iterator feature is enabled, i.e. this setting is irrelevant if `.without_iter()` is used). Keymap iterates by using internal index pages allowing it to load the next 5 objects at the same time. You can change the default 5 to any `u32` greater than zero by using `.with_page_size(num)`. This allows the user to optimize the gas usage of Keymap.

The following is used to produce a Keymap without an iterator in `state.rs`

```ignore
pub static JSON_ADDR_VOTE: Keymap<String, Foo, Json, WithoutIter> =
            KeymapBuilder::new(b"json_vote").without_iter().build();

pub static BINCODE_ADDR_VOTE: Keymap<String, Foo, Bincode2, WithoutIter> =
            KeymapBuilder::new(b"bincode_vote").without_iter().build();
```

The following is used to produce a Keymap with modified index page size:

```ignore
pub static ADDR_VOTE: Keymap<Addr, Foo> = KeymapBuilder::new(b"page_vote").with_page_size(13).build();

pub static JSON_VOTE: Keymap<Addr, Foo, Json> =
            KeymapBuilder::new(b"page_vote").with_page_size(3).build();
```

#### **Read/Write**

You can find more examples of using keymaps in the unit tests of Keymap in `keymap.rs`.

To insert, remove, read from the keymap, do the following:

```ignore
let user_addr: Addr = env.message.sender;

let foo = Foo {
    message: "string one".to_string(),
    votes: 1111,
};

ADDR_VOTE.insert(&mut deps.storage, &user_addr, &foo)?;
// Compiler knows that this is Foo
let read_foo = ADDR_VOTE.get(&deps.storage, &user_addr).unwrap();
assert_eq!(read_foo, foo1);
ADDR_VOTE.remove(&mut deps.storage, &user_addr)?;
assert_eq!(ADDR_VOTE.get_len(&deps.storage)?, 0);
```

#### **Iterator**

There are two methods that create an iterator in Keymap. These are `.iter` and `.iter_keys`. `iter_keys` only iterates over the keys whereas `iter` iterates over (key, item) pairs. Needless to say, `.iter_keys` is more efficient as it does not attempt to read the item.

Keymap also has two paging methods, these are `.paging` and `.paging_keys`. `paging_keys` only paginates keys whereas `iter` iterates over (key, item) pairs. Needless to say, `.iter_keys` is more efficient as it does not attempt to read the item.

Here are some select examples from the unit tests:

```ignore
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
```

```ignore
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

    assert_eq!(x.next().unwrap()?.1, foo1);

    assert_eq!(x.next().unwrap()?.1, foo2);

    Ok(())
}
```

### **Keyset**

This hashset-like storage structure allows the user to store typed objects. Allows iteration with paging over values (without guaranteed ordering, although the order of insertion is preserved until you start removing objects).
An example use-case for such a structure is if you have a set of whitelisted users (that you might want to iterate over).

#### **Init**

To import and intialize this storage object as a static constant in `state.rs`, do the following:

```ignore
use secret_toolkit::storage::{Keyset, KeysetBuilder}
```

```ignore
pub static WHITELIST: Keyset<Addr> = Keyset::new(b"whitelist");
```

> ❗ Initializing the object as const instead of static will also work but be less efficient since the variable won't be able to cache length data.

> add_suffix and KeysetBuilder methods function similarly to that of Keymap's.

#### **Storage Methods**

The following are the methods used to interact with the Keyset:

- `.remove(storage, value)` returns `StdResult<()>`
- `.insert(storage, value)` returns `StdResult<()>` if iter is disabled, but returns `StdResult<bool>` if iter is enabled depending on whether or not the value was already stored (false if already stored).
- (only if iterator is enabled) `.is_empty(storage)` returns `StdResult<bool>`
- (only if iterator is enabled) `.get_len(storage)` returns `StdResult<u32>`
- `.contains(storage, value)` returns `bool`
- (only if iterator is enabled) `.paging(storage, start_page, size)` returns `StdResult<Vec<K>>` where `K` is the stored object's type.
- (only if iterator is enabled) `.iter(storage)` returns `StdResult<ValueIter<K, Ser>>` where `ValueIter` is an iterator of the stored values.
