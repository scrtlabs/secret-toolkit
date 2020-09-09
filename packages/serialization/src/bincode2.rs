use std::any::type_name;

use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{StdError, StdResult};

use crate::Serde;

/// Use bincode2 for serialization.
#[derive(Copy, Clone, Debug)]
pub struct Bincode2;

impl Serde for Bincode2 {
    fn serialize<T: Serialize>(obj: &T) -> StdResult<Vec<u8>> {
        bincode2::serialize(obj).map_err(|err| StdError::serialize_err(type_name::<T>(), err))
    }

    fn deserialize<T: DeserializeOwned>(data: &[u8]) -> StdResult<T> {
        bincode2::deserialize(data).map_err(|err| StdError::parse_err(type_name::<T>(), err))
    }
}
