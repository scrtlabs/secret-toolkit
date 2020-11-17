use core::fmt;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Coin, HumanAddr, Querier, QueryRequest, StdError, StdResult, Uint128, WasmQuery,
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
#[derive(Serialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    TokenInfo {},
    ExchangeRate {},
    Allowance {
        owner: HumanAddr,
        spender: HumanAddr,
        key: String,
    },
    Balance {
        address: HumanAddr,
        key: String,
    },
    TransferHistory {
        address: HumanAddr,
        key: String,
        page: Option<u32>,
        page_size: u32,
    },
    Minters {},
}

impl fmt::Display for QueryMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            QueryMsg::TokenInfo { .. } => write!(f, "TokenInfo"),
            QueryMsg::ExchangeRate { .. } => write!(f, "ExchangeRate"),
            QueryMsg::Allowance { .. } => write!(f, "Allowance"),
            QueryMsg::Balance { .. } => write!(f, "Balance"),
            QueryMsg::TransferHistory { .. } => write!(f, "TransferHistory"),
            QueryMsg::Minters { .. } => write!(f, "Minters"),
        }
    }
}

impl QueryMsg {
    /// Returns a StdResult<T>, where T is the "Response" type that wraps the query answer
    ///
    /// # Arguments
    ///
    /// * `querier` - a reference to the Querier dependency of the querying contract
    /// * `block_size` - pad the message to blocks of this size
    /// * `callback_code_hash` - String holding the code hash of the contract being queried
    /// * `contract_addr` - address of the contract being queried
    pub fn query<Q: Querier, T: DeserializeOwned>(
        &self,
        querier: &Q,
        mut block_size: usize,
        callback_code_hash: String,
        contract_addr: HumanAddr,
    ) -> StdResult<T> {
        // can not have block size of 0
        if block_size == 0 {
            block_size = 1;
        }
        let mut msg = to_binary(self)?;
        space_pad(&mut msg.0, block_size);
        querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                callback_code_hash,
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
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn token_info_query<Q: Querier>(
    querier: &Q,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<TokenInfo> {
    let answer: TokenInfoResponse =
        QueryMsg::TokenInfo {}.query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.token_info)
}

/// Returns a StdResult<ExchangeRate> from performing ExchangeRate query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn exchange_rate_query<Q: Querier>(
    querier: &Q,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<ExchangeRate> {
    let answer: ExchangeRateResponse =
        QueryMsg::ExchangeRate {}.query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.exchange_rate)
}

/// Returns a StdResult<Allowance> from performing Allowance query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `owner` - the address that owns the tokens
/// * `spender` - the address allowed to send/burn tokens
/// * `key` - String holding the authentication key needed to view the allowance
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
#[allow(clippy::too_many_arguments)]
pub fn allowance_query<Q: Querier>(
    querier: &Q,
    owner: HumanAddr,
    spender: HumanAddr,
    key: String,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<Allowance> {
    let answer: AllowanceResponse = QueryMsg::Allowance {
        owner,
        spender,
        key,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.allowance)
}

/// Returns a StdResult<Balance> from performing Balance query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `address` - the address whose balance should be displayed
/// * `key` - String holding the authentication key needed to view the balance
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn balance_query<Q: Querier>(
    querier: &Q,
    address: HumanAddr,
    key: String,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<Balance> {
    let answer: BalanceResponse = QueryMsg::Balance { address, key }.query(
        querier,
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
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `address` - the address whose transaction history should be displayed
/// * `key` - String holding the authentication key needed to view transactions
/// * `page` - Optional u32 representing the page number of transactions to display
/// * `page_size` - u32 number of transactions to return
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
#[allow(clippy::too_many_arguments)]
pub fn transfer_history_query<Q: Querier>(
    querier: &Q,
    address: HumanAddr,
    key: String,
    page: Option<u32>,
    page_size: u32,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<TransferHistory> {
    let answer: TransferHistoryResponse = QueryMsg::TransferHistory {
        address,
        key,
        page,
        page_size,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.transfer_history)
}

/// Returns a StdResult<Minters> from performing Minters query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn minters_query<Q: Querier>(
    querier: &Q,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<Minters> {
    let answer: MintersResponse =
        QueryMsg::Minters {}.query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.minters)
}
