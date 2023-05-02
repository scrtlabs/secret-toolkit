#![doc = include_str!("../Readme.md")]

#[cfg(feature = "hash")]
mod hash;


#[cfg(feature = "rand")]
mod rng;

#[cfg(feature = "rand")]
pub use rand_core::RngCore as RngCore;

#[cfg(feature = "ecc-secp256k1")]
pub mod secp256k1;

#[cfg(feature = "hash")]
pub use hash::{sha_256, SHA256_HASH_SIZE};

#[cfg(feature = "rand")]
pub use rng::ContractPrng;
