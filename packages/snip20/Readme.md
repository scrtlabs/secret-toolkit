# Secret Contract Development Toolkit - SNIP20 Interface

These functions are meant to help you easily interact with SNIP20 compliant tokens.  

## Handle Messages

You can create a HandleMsg variant and call the `to_cosmos_msg` function to generate the CosmosMsg that shoud be pushed onto the InitResponse or HandleResponse `messages` Vec.

Or you can call the individual function for each Handle message to generate the appropriate callback CosmosMsg.

You probably have also noticed that CreateViewingKey is not supported.  This is because a contract can not see the viewing key that is returned because it has already finished executing by the time CreateViewingKey would be called.  If a contract needs to have a viewing key, it must create its own sufficiently complex viewing key, and pass it as a parameter to SetViewingKey. You can see an example of creating a complex viewing key in the [Snip20 Reference Implementation](http://github.com/enigmampc/snip20-reference-impl).  It is also highly recommended that you use the block_size padding option to also mask the length of the viewing key your contract has generated.

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
}

pub struct TransferHistory {
    pub txs: Vec<Tx>,
}

pub struct Minters {
    pub minters: Vec<HumanAddr>,
}
```
You can create a QueryMsg variant and call the `query` function to query a SNIP20 token contract.

Or you can call the individual function for each query.