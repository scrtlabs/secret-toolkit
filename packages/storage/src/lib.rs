#![doc = include_str!("../Readme.md")]

pub mod append_store;
pub mod deque_store;
pub mod item;
pub mod map;
pub mod keyset;

pub use append_store::AppendStore;
pub use deque_store::Deque;
pub use item::Item;
pub use iter_options::WithoutIter;
use iter_options::{IterOption, WithIter};
pub use map::{Map, MapBuilder};
pub use keyset::{Keyset, KeysetBuilder};

pub mod iter_options {
    pub struct WithIter;
    pub struct WithoutIter;
    pub trait IterOption {}

    impl IterOption for WithIter {}
    impl IterOption for WithoutIter {}
}
