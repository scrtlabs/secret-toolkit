# Secret Contract Development Toolkit - SNIP-721 Interface

⚠️ This package is a sub-package of the `secret-toolkit` package. Please see its crate page for more context.

These functions are meant to help you easily interact with SNIP-721 compliant NFT contracts.  

## Handle Messages

You can create a HandleMsg variant and call the `to_cosmos_msg` function to generate the CosmosMsg that should be pushed onto the InitResponse or HandleResponse `messages` Vec.

Or you can call the individual function for each Handle message to generate the appropriate callback CosmosMsg.

Example:

```rust
# use cosmwasm_std::{Uint128, StdError, StdResult, CosmosMsg, Response};
# use secret_toolkit_snip721::transfer_nft_msg;
# fn main() -> StdResult<()> {
let recipient = "ADDRESS_TO_TRANSFER_TO".to_string();
let token_id = "TOKEN_ID".to_string();
let memo = Some("TRANSFER_MEMO".to_string());
let padding = None;
let block_size = 256;
let callback_code_hash = "TOKEN_CONTRACT_CODE_HASH".to_string();
let contract_addr = "TOKEN_CONTRACT_ADDRESS".to_string();

let cosmos_msg = transfer_nft_msg(
    recipient,
    token_id,
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

All you have to do to call a SNIP-721 Handle function is call the appropriate toolkit function, and place the resulting `CosmosMsg` in the `messages` Vec of the InitResponse or HandleResponse.  In this example, we are transferring an NFT named "TOKEN_ID" to the recipient address.  We are not using the `padding` field of the Transfer message, but instead, we are padding the entire message to blocks of 256 bytes.

You probably have also noticed that CreateViewingKey is not supported.  This is because a contract can not see the viewing key that is returned because it has already finished executing by the time CreateViewingKey would be called.  If a contract needs to have a viewing key, it must create its own sufficiently complex viewing key, and pass it as a parameter to SetViewingKey. You can see an example of creating a complex viewing key in the [Snip20 Reference Implementation](http://github.com/enigmampc/snip20-reference-impl).  It is also highly recommended that you use the block_size padding option to mask the length of the viewing key your contract has generated.

## Queries

These are the types that the SNIP-721 toolkit queries can return

```rust
# use secret_toolkit_snip721::{Expiration};
pub struct ContractInfo {
    pub name: String,
    pub symbol: String,
}

pub struct NumTokens {
    pub count: u32,
}

pub struct TokenList {
    pub tokens: Vec<String>,
}

pub struct Cw721Approval {
    pub spender: String,
    pub expires: Expiration,
}

pub struct OwnerOf {
    pub owner: Option<String>,
    pub approvals: Vec<Cw721Approval>,
}

pub struct Metadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
}

pub struct AllNftInfo {
    pub access: OwnerOf,
    pub info: Option<Metadata>,
}

pub struct Snip721Approval {
    pub address: String,
    pub view_owner_expiration: Option<Expiration>,
    pub view_private_metadata_expiration: Option<Expiration>,
    pub transfer_expiration: Option<Expiration>,
}

pub struct NftDossier {
    pub owner: Option<String>,
    pub public_metadata: Option<Metadata>,
    pub private_metadata: Option<Metadata>,
    pub display_private_metadata_error: Option<String>,
    pub owner_is_public: bool,
    pub public_ownership_expiration: Option<Expiration>,
    pub private_metadata_is_public: bool,
    pub private_metadata_is_public_expiration: Option<Expiration>,
    pub token_approvals: Option<Vec<Snip721Approval>>,
    pub inventory_approvals: Option<Vec<Snip721Approval>>,
}

pub struct TokenApprovals {
    pub owner_is_public: bool,
    pub public_ownership_expiration: Option<Expiration>,
    pub private_metadata_is_public: bool,
    pub private_metadata_is_public_expiration: Option<Expiration>,
    pub token_approvals: Vec<Snip721Approval>,
}

pub struct ApprovedForAll {
    pub operators: Vec<Cw721Approval>,
}

pub struct InventoryApprovals {
    pub owner_is_public: bool,
    pub public_ownership_expiration: Option<Expiration>,
    pub private_metadata_is_public: bool,
    pub private_metadata_is_public_expiration: Option<Expiration>,
    pub inventory_approvals: Vec<Snip721Approval>,
}

pub enum TxAction {
    Transfer {
        from: String,
        sender: Option<String>,
        recipient: String,
    },
    Mint {
        minter: String,
        recipient: String,
    },
    Burn {
        owner: String,
        burner: Option<String>,
    },
}

pub struct Tx {
    pub tx_id: u64,
    pub block_height: u64,
    pub block_time: u64,
    pub token_id: String,
    pub action: TxAction,
    pub memo: Option<String>,
}

pub struct TransactionHistory {
    pub total: u64,
    pub txs: Vec<Tx>,
}

pub struct Minters {
    pub minters: Vec<String>,
}

pub struct IsUnwrapped {
    pub token_is_unwrapped: bool,
}

pub struct VerifyTransferApproval {
    pub approved_for_all: bool,
    pub first_unapproved_token: Option<String>,
}
```

You can create a QueryMsg variant and call the `query` function to query a SNIP-721 token contract.

Or you can call the individual function for each query.

Example:

```rust
# use cosmwasm_std::{StdError, testing::mock_dependencies};
# use secret_toolkit_snip721::{nft_dossier_query, ViewerInfo};
# let mut deps = mock_dependencies();
#
let token_id = "TOKEN_ID".to_string();
let viewer = Some(ViewerInfo {
  address: "VIEWER'S_ADDRESS".to_string(),
  viewing_key: "VIEWER'S_KEY".to_string(),
});
let include_expired = None;
let block_size = 256;
let callback_code_hash = "TOKEN_CONTRACT_CODE_HASH".to_string();
let contract_addr = "TOKEN_CONTRACT_ADDRESS".to_string();

let nft_dossier = nft_dossier_query(
  deps.as_ref().querier,
  token_id,
  viewer,
  include_expired,
  block_size,
  callback_code_hash,
  contract_addr
);
#
# assert_eq!(
#     nft_dossier.unwrap_err().to_string(),
#     "Generic error: Error performing NftDossier query: Generic error: Querier system error: No such contract: TOKEN_CONTRACT_ADDRESS"
# );
```

In this example, we are doing an NftDossier query on the token named "TOKEN_ID", supplying the address and viewing key of the querier, and storing the response in the nft_dossier variable, which is of the NftDossier type defined above.  Because no `include_expired` was specified, the response defaults to only displaying approvals that have not expired, but approvals will only be displayed if the viewer is the owner of the token.  The query message is padded to blocks of 256 bytes.
