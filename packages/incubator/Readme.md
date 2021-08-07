# Secret Contract Development Toolkit - Incubator

This package contains tools that are not yet final and may change or contain unknown bugs, and are pending more testing or reviews.

## Cashmap

A hashmap-like structure, that allows iteration with paging over keys without guaranteed ordering.
An example use-case for such a structure is if you want to contain a large amount of votes, deposits, or bets and iterate over them at some time in the future.
Since iterating over large amounts of data at once may be prohibitive, this structure allows you to specify the amount of data that will
be returned in each page.

This structure may also be used as a hashmap structure without the fancy bells and whistles, though gas-costs will be more expensive than simple storage.

### Usage

#### Initialization

You can open/initialize the cashmap directly using 

```rust
let mut storage = MockStorage::new();
let mut cmap = CashMap::init(b"cashmap-name", &mut storage);
```

#### Access pattern

Todo: improve this section

```rust
let foo1 = Foo {
            string: "string one".to_string(),
            number: 1111,
        };

cmap.insert(b"key1", foo1.clone())?;
let read_foo1 = cmap.get(b"key1").unwrap();
cmap.remove(b"key1")?;
```

#### Todo

Generalize keys to allow any hashable type, not just &[u8]