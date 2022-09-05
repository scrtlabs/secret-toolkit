use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Uint128};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct TransferAction {
    pub recipient: String,
    pub amount: Uint128,
    pub memo: Option<String>,
}

impl TransferAction {
    pub fn new(recipient: String, amount: Uint128, memo: Option<String>) -> Self {
        Self {
            recipient,
            amount,
            memo,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct SendAction {
    pub recipient: String,
    pub recipient_code_hash: Option<String>,
    pub amount: Uint128,
    pub msg: Option<Binary>,
    pub memo: Option<String>,
}

impl SendAction {
    pub fn new(
        recipient: String,
        amount: Uint128,
        msg: Option<Binary>,
        memo: Option<String>,
    ) -> Self {
        Self {
            recipient,
            recipient_code_hash: None,
            amount,
            msg,
            memo,
        }
    }

    pub fn new_with_code_hash(
        recipient: String,
        recipient_code_hash: Option<String>,
        amount: Uint128,
        msg: Option<Binary>,
        memo: Option<String>,
    ) -> Self {
        Self {
            recipient,
            recipient_code_hash,
            amount,
            msg,
            memo,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct TransferFromAction {
    pub owner: String,
    pub recipient: String,
    pub amount: Uint128,
    pub memo: Option<String>,
}

impl TransferFromAction {
    pub fn new(owner: String, recipient: String, amount: Uint128, memo: Option<String>) -> Self {
        Self {
            owner,
            recipient,
            amount,
            memo,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct SendFromAction {
    pub owner: String,
    pub recipient: String,
    pub recipient_code_hash: Option<String>,
    pub amount: Uint128,
    pub msg: Option<Binary>,
    pub memo: Option<String>,
}

impl SendFromAction {
    pub fn new(
        owner: String,
        recipient: String,
        amount: Uint128,
        msg: Option<Binary>,
        memo: Option<String>,
    ) -> Self {
        Self {
            owner,
            recipient,
            recipient_code_hash: None,
            amount,
            msg,
            memo,
        }
    }

    pub fn new_with_code_hash(
        owner: String,
        recipient: String,
        recipient_code_hash: Option<String>,
        amount: Uint128,
        msg: Option<Binary>,
        memo: Option<String>,
    ) -> Self {
        Self {
            owner,
            recipient,
            recipient_code_hash,
            amount,
            msg,
            memo,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct MintAction {
    pub recipient: String,
    pub amount: Uint128,
    pub memo: Option<String>,
}

impl MintAction {
    pub fn new(recipient: String, amount: Uint128, memo: Option<String>) -> Self {
        Self {
            recipient,
            amount,
            memo,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct BurnFromAction {
    pub owner: String,
    pub amount: Uint128,
    pub memo: Option<String>,
}

impl BurnFromAction {
    pub fn new(owner: String, amount: Uint128, memo: Option<String>) -> Self {
        Self {
            owner,
            amount,
            memo,
        }
    }
}
