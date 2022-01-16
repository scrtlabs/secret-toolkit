# Secret Contract Development Toolkit - SNIP20 Interface

These functions are meant to help you easily interact with SNIP20 compliant tokens.  

## Handle Messages

You can create a HandleMsg variant and call the `to_cosmos_msg` function to generate the CosmosMsg that should be pushed onto the InitResponse or HandleResponse `messages` Vec.

Or you can call the individual function for each Handle message to generate the appropriate callback CosmosMsg.

Example:
```rust
    let recipient = HumanAddr("ADDRESS_TO_TRANSFER_TO".to_string());
    let amount = Uint128(10000);
    let padding = None;
    let block_size = 256;
    let callback_code_hash = "TOKEN_CONTRACT_CODE_HASH".to_string();
    let contract_addr = HumanAddr("TOKEN_CONTRACT_ADDRESS".to_string());

    let cosmos_msg = transfer_msg(
        recipient,
        amount,
        padding,
        block_size,
        callback_code_hash,
        contract_addr,
    )?;

    Ok(HandleResponse {
        messages: vec![cosmos_msg],
        log: vec![],
        data: None,
    })
```
All you have to do to call a SNIP-20 Handle function is call the appropriate toolkit function, and place the resulting `CosmosMsg` in the `messages` Vec of the InitResponse or HandleResponse.  In this example, we are transferring 10000 (in the lowest denomination of the token) to the recipient address.  We are not using the `padding` field of the Transfer message, but instead, we are padding the entire message to blocks of 256 bytes.

You probably have also noticed that CreateViewingKey is not supported.  This is because a contract can not see the viewing key that is returned because it has already finished executing by the time CreateViewingKey would be called.  If a contract needs to have a viewing key, it must create its own sufficiently complex viewing key, and pass it as a parameter to SetViewingKey. You can see an example of creating a complex viewing key in the [Snip20 Reference Implementation](http://github.com/enigmampc/snip20-reference-impl).  It is also highly recommended that you use the block_size padding option to mask the length of the viewing key your contract has generated.

## Queries

These are the types that SNIP20 tokens can return from queries
```rust
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

pub struct Allowance {
    pub spender: HumanAddr,
    pub owner: HumanAddr,
    pub allowance: Uint128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<u64>,
}

pub struct Balance {
    pub amount: Uint128,
}

pub struct Tx {
    pub id: u64,
    pub from: HumanAddr,
    pub sender: HumanAddr,
    pub receiver: HumanAddr,
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

#[serde(rename_all = "snake_case")]
pub enum TxAction {
    Transfer {
        from: HumanAddr,
        sender: HumanAddr,
        recipient: HumanAddr,
    },
    Mint {
        minter: HumanAddr,
        recipient: HumanAddr,
    },
    Burn {
        burner: HumanAddr,
        owner: HumanAddr,
    },
    Deposit {},
    Redeem {},
}

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
    pub minters: Vec<HumanAddr>,
}
```
You can create a QueryMsg variant and call the `query` function to query a SNIP20 token contract.

Or you can call the individual function for each query.

Example:
```rust
    let address = HumanAddr("ADDRESS_WHOSE_BALANCE_IS_BEING_REQUESTED".to_string());
    let key = "THE_VIEWING_KEY_PREVIOUSLY_SET_BY_THE_ADDRESS".to_string();
    let block_size = 256;
    let callback_code_hash = "TOKEN_CONTRACT_CODE_HASH".to_string();
    let contract_addr = HumanAddr("TOKEN_CONTRACT_ADDRESS".to_string());

    let balance =
        balance_query(&deps.querier, address, key, block_size, callback_code_hash, contract_addr)?;
```
In this example, we are doing a Balance query for the specified address/key pair and storing the response in the balance variable, which is of the Balance type defined above.  The query message is padded to blocks of 256 bytes.