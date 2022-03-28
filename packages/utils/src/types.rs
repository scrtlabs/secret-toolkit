use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone, JsonSchema)]
pub struct Contract {
    pub address: HumanAddr,
    pub hash: String,
}
