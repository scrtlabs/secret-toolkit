use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::StdResult;

use crate::Serde;

/// Use json for serialization
#[derive(Copy, Clone, Debug)]
pub struct Json;

impl Serde for Json {
    fn serialize<T: Serialize>(obj: &T) -> StdResult<Vec<u8>> {
        cosmwasm_std::to_vec(obj)
    }

    fn deserialize<T: DeserializeOwned>(data: &[u8]) -> StdResult<T> {
        cosmwasm_std::from_slice(data)
    }
}
