# Secret Contract Development Toolkit - SNIP20 Interface

⚠️ This package is a sub-package of the `secret-toolkit` package. Please see its crate page for more context.

These functions are meant to help you easily interact with SNIP20 compliant tokens.  

## Handle Messages

You can create a HandleMsg variant and call the `to_cosmos_msg` function to generate the CosmosMsg that should be pushed onto the InitResponse or HandleResponse `messages` Vec.

Or you can call the individual function for each Handle message to generate the appropriate callback CosmosMsg.

Example:

```rust
# use cosmwasm_std::{Uint128, StdError, StdResult, CosmosMsg, Response};
# use secret_toolkit_snip20::{transfer_msg};
#
# fn main() -> StdResult<()> {
let recipient = "ADDRESS_TO_TRANSFER_TO".to_string();
let amount = Uint128::from(10000u128);
let memo = Some("memo".to_string());
let padding = None;
let block_size = 256;
let callback_code_hash = "TOKEN_CONTRACT_CODE_HASH".to_string();
let contract_addr = "TOKEN_CONTRACT_ADDRESS".to_string();

let cosmos_msg = transfer_msg(
    recipient,
    amount,
    memo,
    padding,
    block_size,
    callback_code_hash,
    contract_addr,
)?;

let response = Ok(Response::new().add_message(cosmos_msg));
# response.map(|_r| ())
# }
```

All you have to do to call a SNIP-20 Handle function is call the appropriate toolkit function, and place the resulting `CosmosMsg` in the `messages` Vec of the InitResponse or HandleResponse.  In this example, we are transferring 10000 (in the lowest denomination of the token) to the recipient address.  We are not using the `padding` field of the Transfer message, but instead, we are padding the entire message to blocks of 256 bytes.

You probably have also noticed that CreateViewingKey is not supported.  This is because a contract can not see the viewing key that is returned because it has already finished executing by the time CreateViewingKey would be called.  If a contract needs to have a viewing key, it must create its own sufficiently complex viewing key, and pass it as a parameter to SetViewingKey. You can see an example of creating a complex viewing key in the [Snip20 Reference Implementation](http://github.com/enigmampc/snip20-reference-impl).  It is also highly recommended that you use the block_size padding option to mask the length of the viewing key your contract has generated.

## Queries

These are the types that SNIP20 tokens can return from queries

```rust
# use cosmwasm_std::{Uint128, Coin};
# use serde::Serialize;
#
# #[derive(Serialize)]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_supply: Option<Uint128>,
}

pub struct ExchangeRate {
    pub rate: Uint128,
    pub denom: String,
}

# #[derive(Serialize)]
pub struct Allowance {
    pub spender: String,
    pub owner: String,
    pub allowance: Uint128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<u64>,
}

pub struct Balance {
    pub amount: Uint128,
}

# #[derive(Serialize)]
pub struct Tx {
    pub id: u64,
    pub from: String,
    pub sender: String,
    pub receiver: String,
    pub coins: Coin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    pub block_time: Option<u64>,
    pub block_height: Option<u64>,
}

pub struct TransferHistory {
    pub total: Option<u64>,
    pub txs: Vec<Tx>,
}

# #[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TxAction {
    Transfer {
        from: String,
        sender: String,
        recipient: String,
    },
    Mint {
        minter: String,
        recipient: String,
    },
    Burn {
        burner: String,
        owner: String,
    },
    Deposit {},
    Redeem {},
}

# #[derive(Serialize)]
pub struct RichTx {
    pub id: u64,
    pub action: TxAction,
    pub coins: Coin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    pub block_time: u64,
    pub block_height: u64,
}

pub struct TransactionHistory {
    pub total: Option<u64>,
    pub txs: Vec<RichTx>,
}

pub struct Minters {
    pub minters: Vec<String>,
}
```

You can create a QueryMsg variant and call the `query` function to query a SNIP20 token contract.

Or you can call the individual function for each query.

Example:

```rust
# use cosmwasm_std::{StdError, QuerierWrapper, testing::mock_dependencies};
# use secret_toolkit_snip20::balance_query;
# let mut deps = mock_dependencies();
#
let address = "ADDRESS_WHOSE_BALANCE_IS_BEING_REQUESTED".to_string();
let key = "THE_VIEWING_KEY_PREVIOUSLY_SET_BY_THE_ADDRESS".to_string();
let block_size = 256;
let callback_code_hash = "TOKEN_CONTRACT_CODE_HASH".to_string();
let contract_addr = "TOKEN_CONTRACT_ADDRESS".to_string();

let balance =
    balance_query(deps.as_ref().querier, address, key, block_size, callback_code_hash, contract_addr);
#
# assert_eq!(
#     balance.unwrap_err().to_string(), 
#     "Generic error: Error performing Balance query: Generic error: Querier system error: No such contract: TOKEN_CONTRACT_ADDRESS"
# );
```

In this example, we are doing a Balance query for the specified address/key pair and storing the response in the balance variable, which is of the Balance type defined above.  The query message is padded to blocks of 256 bytes.
