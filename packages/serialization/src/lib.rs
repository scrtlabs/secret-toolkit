use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::StdResult;

#[cfg(feature = "base64")]
mod base64;
#[cfg(feature = "bincode2")]
mod bincode2;
#[cfg(feature = "json")]
mod json;

#[cfg(all(feature = "bincode2", feature = "base64"))]
pub use crate::base64::Base64Bincode2Of;
#[cfg(all(feature = "json", feature = "base64"))]
pub use crate::base64::Base64JsonOf;
#[cfg(feature = "base64")]
pub use crate::base64::{Base64, Base64Of};

#[cfg(feature = "bincode2")]
pub use crate::bincode2::Bincode2;
#[cfg(feature = "json")]
pub use crate::json::Json;

/// This trait represents the ability to both serialize and deserialize using a specific format.
///
/// This is useful for types that want to have a default mode of serialization, but want
/// to allow users to override it if they want to.
///
/// It is intentionally simple at the moment to keep the implementation easy.
pub trait Serde {
    fn serialize<T: Serialize>(obj: &T) -> StdResult<Vec<u8>>;
    fn deserialize<T: DeserializeOwned>(data: &[u8]) -> StdResult<T>;
}
