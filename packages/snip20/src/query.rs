use core::fmt;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Api, Coin, Extern, HumanAddr, Querier, QueryRequest, StdError, StdResult, Storage,
    Uint128, WasmQuery,
};

use secret_toolkit_utils::space_pad;

/// TokenInfo response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_supply: Option<Uint128>,
}
/// ExchangeRate response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExchangeRate {
    pub rate: Uint128,
    pub denom: String,
}
/// Allowance response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Allowance {
    pub spender: HumanAddr,
    pub owner: HumanAddr,
    pub allowance: Uint128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<u64>,
}
/// Balance response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Balance {
    pub amount: Uint128,
}
/// Transaction data
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Tx {
    pub id: u64,
    pub from: HumanAddr,
    pub sender: HumanAddr,
    pub receiver: HumanAddr,
    pub coins: Coin,
}
/// TransferHistory response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TransferHistory {
    pub txs: Vec<Tx>,
}
/// Minters response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Minters {
    pub minters: Vec<HumanAddr>,
}

/// SNIP20 queries
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Snip20QueryMsg<'a> {
    TokenInfo {},
    ExchangeRate {},
    Allowance {
        owner: &'a HumanAddr,
        spender: &'a HumanAddr,
        key: &'a str,
    },
    Balance {
        address: &'a HumanAddr,
        key: &'a str,
    },
    TransferHistory {
        address: &'a HumanAddr,
        key: &'a str,
        page: Option<u32>,
        page_size: u32,
    },
    Minters {},
}

impl<'a> fmt::Display for Snip20QueryMsg<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Snip20QueryMsg::TokenInfo { .. } => write!(f, "TokenInfo"),
            Snip20QueryMsg::ExchangeRate { .. } => write!(f, "ExchangeRate"),
            Snip20QueryMsg::Allowance { .. } => write!(f, "Allowance"),
            Snip20QueryMsg::Balance { .. } => write!(f, "Balance"),
            Snip20QueryMsg::TransferHistory { .. } => write!(f, "TransferHistory"),
            Snip20QueryMsg::Minters { .. } => write!(f, "Minters"),
        }
    }
}

impl<'a> Snip20QueryMsg<'a> {
    /// Returns a StdResult<T>, where T is the "Response" type that wraps the query answer
    ///
    /// # Arguments
    ///
    /// * `deps` - reference to Extern that holds all the external contract dependencies
    /// * `block_size` - pad message to blocks of this size
    /// * `callback_code_hash` - string slice holding the code hash of contract being queried
    /// * `contract_addr` - reference to address of contract being queries
    pub fn query<S: Storage, A: Api, Q: Querier, T: DeserializeOwned>(
        &'a self,
        deps: &'a Extern<S, A, Q>,
        block_size: usize,
        callback_code_hash: &'a str,
        contract_addr: &'a HumanAddr,
    ) -> StdResult<T> {
        let pad_block_size: usize;
        // can not have block size of 0
        if block_size == 0 {
            pad_block_size = 1;
        } else {
            pad_block_size = block_size;
        }
        let mut msg = to_binary(self)?;
        space_pad(&mut msg.0, pad_block_size);
        deps.querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: contract_addr.clone(),
                callback_code_hash: callback_code_hash.to_string(),
                msg,
            }))
            .map_err(|err| {
                StdError::generic_err(format!("Error performing {} query: {}", self, err))
            })
    }
}

/// wrapper to deserialize TokenInfo response
#[derive(Deserialize)]
pub struct TokenInfoResponse {
    pub token_info: TokenInfo,
}
/// wrapper to deserialize ExchangeRate response
#[derive(Deserialize)]
pub struct ExchangeRateResponse {
    pub exchange_rate: ExchangeRate,
}
/// wrapper to deserialize Allowance response
#[derive(Deserialize)]
pub struct AllowanceResponse {
    pub allowance: Allowance,
}
/// wrapper to deserialize Balance response
#[derive(Deserialize)]
pub struct BalanceResponse {
    pub balance: Balance,
}
/// wrapper to deserialize TransferHistory response
#[derive(Deserialize)]
pub struct TransferHistoryResponse {
    pub transfer_history: TransferHistory,
}
/// wrapper to deserialize Minters response
#[derive(Deserialize)]
pub struct MintersResponse {
    pub minters: Minters,
}

/// Returns a StdResult<TokenInfo> from performing TokenInfo query
///
/// # Arguments
///
/// * `deps` - reference to Extern that holds all the external contract dependencies
/// * `block_size` - pad message to blocks of this size
/// * `callback_code_hash` - string slice holding the code hash of contract being queried
/// * `contract_addr` - reference to address of contract being queries
pub fn token_info_query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    block_size: usize,
    callback_code_hash: &str,
    contract_addr: &HumanAddr,
) -> StdResult<TokenInfo> {
    let answer: TokenInfoResponse =
        Snip20QueryMsg::TokenInfo {}.query(deps, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.token_info)
}

/// Returns a StdResult<ExchangeRate> from performing ExchangeRate query
///
/// # Arguments
///
/// * `deps` - reference to Extern that holds all the external contract dependencies
/// * `block_size` - pad message to blocks of this size
/// * `callback_code_hash` - string slice holding the code hash of contract being queried
/// * `contract_addr` - reference to address of contract being queries
pub fn exchange_rate_query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    block_size: usize,
    callback_code_hash: &str,
    contract_addr: &HumanAddr,
) -> StdResult<ExchangeRate> {
    let answer: ExchangeRateResponse = Snip20QueryMsg::ExchangeRate {}.query(
        deps,
        block_size,
        callback_code_hash,
        contract_addr,
    )?;
    Ok(answer.exchange_rate)
}

/// Returns a StdResult<Allowance> from performing Allowance query
///
/// # Arguments
///
/// * `deps` - reference to Extern that holds all the external contract dependencies
/// * `owner` - reference to address that owns the tokens
/// * `spender` - reference to address allowed to send/burn tokens
/// * `key` - string slice holding the authentication key needed to view allowance
/// * `block_size` - pad message to blocks of this size
/// * `callback_code_hash` - string slice holding the code hash of contract being queried
/// * `contract_addr` - reference to address of contract being queries
#[allow(clippy::too_many_arguments)]
pub fn allowance_query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    owner: &HumanAddr,
    spender: &HumanAddr,
    key: &str,
    block_size: usize,
    callback_code_hash: &str,
    contract_addr: &HumanAddr,
) -> StdResult<Allowance> {
    let answer: AllowanceResponse = Snip20QueryMsg::Allowance {
        owner,
        spender,
        key,
    }
    .query(deps, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.allowance)
}

/// Returns a StdResult<Balance> from performing Balance query
///
/// # Arguments
///
/// * `deps` - reference to Extern that holds all the external contract dependencies
/// * `address` - reference to address whose balance should be displayed
/// * `key` - string slice holding the authentication key needed to view balance
/// * `block_size` - pad message to blocks of this size
/// * `callback_code_hash` - string slice holding the code hash of contract being queried
/// * `contract_addr` - reference to address of contract being queries
pub fn balance_query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
    key: &str,
    block_size: usize,
    callback_code_hash: &str,
    contract_addr: &HumanAddr,
) -> StdResult<Balance> {
    let answer: BalanceResponse = Snip20QueryMsg::Balance { address, key }.query(
        deps,
        block_size,
        callback_code_hash,
        contract_addr,
    )?;
    Ok(answer.balance)
}

/// Returns a StdResult<TransferHistory> from performing TransferHistory query
///
/// # Arguments
///
/// * `deps` - reference to Extern that holds all the external contract dependencies
/// * `address` - reference to address whose transaction history should be displayed
/// * `key` - string slice holding the authentication key needed to view transactions
/// * `page` - Optional u32 representing page number of transactions to display
/// * `page_size` - u32 number of transactions to return
/// * `block_size` - pad message to blocks of this size
/// * `callback_code_hash` - string slice holding the code hash of contract being queried
/// * `contract_addr` - reference to address of contract being queries
#[allow(clippy::too_many_arguments)]
pub fn transfer_history_query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
    key: &str,
    page: Option<u32>,
    page_size: u32,
    block_size: usize,
    callback_code_hash: &str,
    contract_addr: &HumanAddr,
) -> StdResult<TransferHistory> {
    let answer: TransferHistoryResponse = Snip20QueryMsg::TransferHistory {
        address,
        key,
        page,
        page_size,
    }
    .query(deps, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.transfer_history)
}

/// Returns a StdResult<Minters> from performing Minters query
///
/// # Arguments
///
/// * `deps` - reference to Extern that holds all the external contract dependencies
/// * `block_size` - pad message to blocks of this size
/// * `callback_code_hash` - string slice holding the code hash of contract being queried
/// * `contract_addr` - reference to address of contract being queries
pub fn minters_query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    block_size: usize,
    callback_code_hash: &str,
    contract_addr: &HumanAddr,
) -> StdResult<Minters> {
    let answer: MintersResponse =
        Snip20QueryMsg::Minters {}.query(deps, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.minters)
}
