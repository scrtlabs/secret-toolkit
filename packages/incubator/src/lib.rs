#[cfg(feature = "cashmap")]
pub mod cashmap;
#[cfg(feature = "cashmap")]
pub use cashmap::{CashMap, ReadOnlyCashMap};

#[cfg(feature = "generational-store")]
pub mod generational_store;
#[cfg(feature = "generational-store")]
pub use generational_store::{GenerationalStore, GenerationalStoreMut};

#[cfg(feature = "maxheap")]
pub mod maxheap;
#[cfg(feature = "maxheap")]
pub use maxheap::{MaxHeapStore, MaxHeapStoreMut};
