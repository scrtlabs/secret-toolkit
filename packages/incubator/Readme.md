# Secret Contract Development Toolkit - Incubator

⚠️ This package is a sub-package of the `secret-toolkit` package. Please see its crate page for more context.

This package contains tools that are not yet final and may change or contain unknown bugs, and are pending more testing or reviews.

## Max heap storage

A "max heap store" is a storage wrapper that implements a binary tree maxheap data structure.
<https://en.wikipedia.org/wiki/Min-max_heap>
Implementation based on <https://algorithmtutor.com/Data-Structures/Tree/Binary-Heaps/>

* Insertion O(log n)
* Remove max O(log n)

### Usage

The usage of `MaxHeapStoreMut` and `MaxHeapStore` are modeled on `AppendStoreMut` and `AppendStore`, respectively. To add an item to the heap use `insert` and to take the top value off use `remove`, which also returns the item that was removed. To peek at the max value without removing, use the `get_max` function. Duplicate items can be added to the heap.

```rust
# use cosmwasm_std::{StdError, testing::MockStorage};
# use secret_toolkit_incubator::maxheap::MaxHeapStoreMut;
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
# Ok::<(), StdError>(())
```

In order to use a custom struct with `MaxHeapStore` you will need to implement the appropriate Ordering traits. The following is an example with a custom struct `Tx` that uses the `amount` field to determine order in the heap:

```rust
# use cosmwasm_std::{StdError, testing::MockStorage, Addr};
# use secret_toolkit_incubator::maxheap::MaxHeapStoreMut;
# use serde::{Serialize, Deserialize};
# use std::cmp::Ordering;
#[derive(Serialize, Deserialize, Clone, Debug, Eq)]
pub struct Tx {
    address: Addr,
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

heap_store.insert(&Tx{
    address: Addr::unchecked("address1"),
    amount: 100,
})?;
heap_store.insert(&Tx{
    address: Addr::unchecked("address2"),
    amount: 200,
})?;
heap_store.insert(&Tx{
    address: Addr::unchecked("address5"),
    amount: 50,
})?;

assert_eq!(heap_store.remove(), Ok(Tx{
    address: Addr::unchecked("address3"),
    amount: 200,
}));
assert_eq!(heap_store.remove(), Ok(Tx{
    address: Addr::unchecked("address4"),
    amount: 100,
}));
assert_eq!(heap_store.remove(), Ok(Tx{
    address: Addr::unchecked("address1"),
    amount: 50,
}));
# Ok::<(), StdError>(())
```

`MaxHeapStore` is modeled on an `AppendStore` and stores the array representation of the heap in the same way, e.g. using `len` key to store the length. Therefore, you can attach an `AppendStore` to a max heap instead of `MaxHeapStore` if you want to iterate over all the values for some reason.

## Generational index storage

Also known as a slot map, a generational index storage is an iterable data structure where each element in the list is identified by a unique key that is a pair (index, generation). Each time an item is removed from the list the generation of the storage increments by one. If a new item is placed at the same index as a previous item which had been removed previously, the old references will not point to the new element. This is because although the index matches, the generation does not. This ensures that each reference to an element in the list is stable and safe.

Starting with an empty set, if we insert A we will have key: (index: 0, generation: 0). Inserting B will have the key: (index: 1, generation: 0). When we remove A the generation will increment by 1 and index 0 will be freed up. When we insert C it will go to the head of our list of free slots and be given the key (index: 0, generation: 1). If you attempt to get A the result will be None, even though A and C have both been at "0" position in the list.

Unlike AppendStore, iteration over a generational index storage is not in order of insertion.

### Use cases

The main use for this type of storage is when we want sets of elements that might be referenced by other structs or lists in a contract, and we want to ensure if an element is removed that our other references do not break. For example, imagine we have a contract where we want a collection of User structs that are independent of secret addresses (perhaps we want people to be able to move their accounts from one address to another). We also want people to be able to remove User accounts, so we use a generational index storage. We can reference the User account by its generational index key (index, generation). We can also reference relationships between users by adding a field in the User struct that points to another key in the generation index storage. If we remove a User and a new User is put in the same index but at a different generation, then there is no risk that the links will point to the wrong user. One can easily imagine this being expanded to a more heterogeneous group of inter-related elements, not just users.

In effect, this example is a graph structure where the nodes are elements and the references to unique (index, generation) pairs are the edges. Any graph like that could be implemented using the generational index storage.

### Usage

See tests in `generational_store.rs` for more examples, including iteration.

```rust
# use cosmwasm_std::{StdError, testing::MockStorage};
# use secret_toolkit_incubator::generational_store::{GenerationalStoreMut, Index};
let mut storage = MockStorage::new();
let mut gen_store = GenerationalStoreMut::attach_or_create(&mut storage)?;
let alpha = gen_store.insert(String::from("Alpha"));
let beta = gen_store.insert(String::from("Beta"));
let gamma = gen_store.insert(String::from("Gamma"));

assert_eq!(gen_store.len(), 3_u32);
assert_eq!(gen_store.remove(beta.clone()), Ok(Some(String::from("Beta"))));
assert_eq!(gen_store.len(), 2_u32);
assert_eq!(gen_store.get(alpha), Some(String::from("Alpha")));
assert_eq!(gen_store.get(beta.clone()), None);
assert_eq!(gen_store.get(gamma), Some(String::from("Gamma")));

let delta = gen_store.insert(String::from("Delta"));
assert_eq!(gen_store.get(delta.clone()), Some(String::from("Delta")));
// check that the generation has updated
assert_ne!(delta.clone(), Index::from_raw_parts(1, 0));
// delta has filled the slot where beta was but generation is now 1
assert_eq!(delta, Index::from_raw_parts(1, 1));

// cannot remove twice
assert!(gen_store.remove(beta).is_err());
# Ok::<(), StdError>(())
```

### Todo

Rename as SlotMap? (see: [https://docs.rs/slotmap/1.0.5/slotmap/](https://docs.rs/slotmap/1.0.5/slotmap/)) Simpler name though maybe not as evocative of what it actually does.
