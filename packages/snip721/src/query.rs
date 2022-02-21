use core::fmt;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmwasm_std::{to_binary, HumanAddr, Querier, QueryRequest, StdError, StdResult, WasmQuery};

use crate::expiration::Expiration;
use crate::metadata::Metadata;
use secret_toolkit_utils::space_pad;

//
// Structs Used for Input Parameters
//

/// the address and viewing key making an authenticated query request
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ViewerInfo {
    /// querying address
    pub address: HumanAddr,
    /// authentication key string
    pub viewing_key: String,
}

//
// Base SNIP-721 Query Responses
//

/// [`ContractInfo`](QueryMsg::ContractInfo) response
///
/// display the contract's name and symbol
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractInfo {
    pub name: String,
    pub symbol: String,
}

/// [`NumTokens`](QueryMsg::NumTokens) response
///
/// display the number of tokens controlled by the contract.  The token supply must
/// either be public, or the querier must be authorized to view
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NumTokens {
    pub count: u32,
}

/// response for [`AllTokens`](QueryMsg::AllTokens) and [`Tokens`](QueryMsg::Tokens)
///
/// * AllTokens:
/// display an optionally paginated list of all the tokens controlled by the contract.
/// The token supply must either be public, or the querier must be authorized to view
/// * Tokens:
/// displays a list of all the tokens belonging to the input owner in which the viewer
/// has view_owner permission
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenList {
    /// list of token IDs
    pub tokens: Vec<String>,
}

/// CW-721 Approval
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw721Approval {
    /// address that can transfer the token
    pub spender: HumanAddr,
    /// expiration of this approval
    pub expires: Expiration,
}

/// response of [`OwnerOf`](QueryMsg::OwnerOf)
///
/// display the owner of the specified token if authorized to view it.  If the requester
/// is also the token's owner, the response will also include a list of any addresses
/// that can transfer this token.  The transfer approval list is for CW721 compliance,
/// but the [`NftDossier`](QueryMsg::NftDossier) query will be more complete by showing viewing approvals as well
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OwnerOf {
    /// Owner of the token if permitted to view it
    pub owner: Option<HumanAddr>,
    /// list of addresses approved to transfer this token
    pub approvals: Vec<Cw721Approval>,
}

/// response of [`AllNftInfo`](QueryMsg::AllNftInfo)
///
/// displays all the information contained in the [`OwnerOf`](QueryMsg::OwnerOf) and [`NftInfo`](QueryMsg::NftInfo) queries
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AllNftInfo {
    /// OwnerOf response
    pub access: OwnerOf,
    /// the public metadata if it exists
    pub info: Option<Metadata>,
}

/// SNIP721 Approval
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Snip721Approval {
    /// whitelisted address
    pub address: HumanAddr,
    /// optional expiration if the address has view owner permission
    pub view_owner_expiration: Option<Expiration>,
    /// optional expiration if the address has view private metadata permission
    pub view_private_metadata_expiration: Option<Expiration>,
    /// optional expiration if the address has transfer permission
    pub transfer_expiration: Option<Expiration>,
}

/// response of [`NftDossier`](QueryMsg::NftDossier)
///
/// displays all the information about a token that the viewer has permission to
/// see.  This may include the owner, the public metadata, the private metadata, and
/// the token and inventory approvals
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NftDossier {
    /// owner of the token if permitted to view it
    pub owner: Option<HumanAddr>,
    /// the token's public metadata
    pub public_metadata: Option<Metadata>,
    /// the token's private metadata if permitted to view it
    pub private_metadata: Option<Metadata>,
    /// description of why private metadata is not displayed (if applicable)
    pub display_private_metadata_error: Option<String>,
    /// true if the owner is publicly viewable
    pub owner_is_public: bool,
    /// expiration of public display of ownership (if applicable)
    pub public_ownership_expiration: Option<Expiration>,
    /// true if private metadata is publicly viewable
    pub private_metadata_is_public: bool,
    /// expiration of public display of private metadata (if applicable)
    pub private_metadata_is_public_expiration: Option<Expiration>,
    /// approvals for this token (only viewable if queried by the owner)
    pub token_approvals: Option<Vec<Snip721Approval>>,
    /// approvals that apply to this token because they apply to all of
    /// the owner's tokens (only viewable if queried by the owner)
    pub inventory_approvals: Option<Vec<Snip721Approval>>,
}

/// response of [`TokenApprovals`](QueryMsg::TokenApprovals)
///
/// list all the [`Approvals`](Snip721Approval) in place for a specified token if given the owner's viewing
/// key
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenApprovals {
    /// true if the owner is publicly viewable
    pub owner_is_public: bool,
    /// expiration of public display of ownership (if applicable)
    pub public_ownership_expiration: Option<Expiration>,
    /// true if private metadata is publicly viewable
    pub private_metadata_is_public: bool,
    /// expiration of public display of private metadata (if applicable)
    pub private_metadata_is_public_expiration: Option<Expiration>,
    /// approvals for this token
    pub token_approvals: Vec<Snip721Approval>,
}

/// response of [`ApprovedForAll`](QueryMsg::ApprovedForAll)
///
/// displays a list of all the CW721-style operators (any address that was granted
/// approval to transfer all of the owner's tokens).  This query is provided to maintain
/// CW-721 compliance, however, approvals are private on secret network, so only the
/// owner's viewing key will authorize the ability to see the list of operators
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ApprovedForAll {
    pub operators: Vec<Cw721Approval>,
}

/// response of [`InventoryApprovals`](QueryMsg::InventoryApprovals)
///
/// list all the inventory-wide [`Approvals`](Snip721Approval) in place for the specified address if given the
/// the correct viewing key for the address
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InventoryApprovals {
    /// true if the owner is publicly viewable
    pub owner_is_public: bool,
    /// expiration of public display of ownership (if applicable)
    pub public_ownership_expiration: Option<Expiration>,
    /// true if private metadata is publicly viewable
    pub private_metadata_is_public: bool,
    /// expiration of public display of private metadata (if applicable)
    pub private_metadata_is_public_expiration: Option<Expiration>,
    /// approvals that apply to the owner's entire inventory of tokens
    pub inventory_approvals: Vec<Snip721Approval>,
}

/// tx type and specifics
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TxAction {
    /// transferred token ownership
    Transfer {
        /// previous owner
        from: HumanAddr,
        /// optional sender if not owner
        sender: Option<HumanAddr>,
        /// new owner
        recipient: HumanAddr,
    },
    /// minted new token
    Mint {
        /// minter's address
        minter: HumanAddr,
        /// token's first owner
        recipient: HumanAddr,
    },
    /// burned a token
    Burn {
        /// previous owner
        owner: HumanAddr,
        /// burner's address if not owner
        burner: Option<HumanAddr>,
    },
}

/// tx for display
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Tx {
    /// tx id
    pub tx_id: u64,
    /// the block containing this tx
    pub block_height: u64,
    /// the time (in seconds since 01/01/1970) of the block containing this tx
    pub block_time: u64,
    /// token id
    pub token_id: String,
    /// tx type and specifics
    pub action: TxAction,
    /// optional memo
    pub memo: Option<String>,
}

/// response of [`TransactionHistory`](QueryMsg::TransactionHistory)
///
/// display the transaction history for the specified address in reverse
/// chronological order
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TransactionHistory {
    /// total transaction count
    pub total: u64,
    /// list of transactions
    pub txs: Vec<Tx>,
}

//
// Optional Queries
//

/// response of [`Minters`](QueryMsg::Minters)
///
/// display the list of authorized minters
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Minters {
    pub minters: Vec<HumanAddr>,
}

/// response of [`IsUnwrapped`](QueryMsg::IsUnwrapped)
///
/// display if a token is unwrapped
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IsUnwrapped {
    pub token_is_unwrapped: bool,
}

/// response of [`VerifyTransferApproval`](QueryMsg::VerifyTransferApproval)
///
/// verify that the specified address has approval to transfer every listed token
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VerifyTransferApproval {
    /// true if `address` has transfer approval for all tokens in the list
    pub approved_for_all: bool,
    /// first token in the list that `address` does not have transfer approval
    pub first_unapproved_token: Option<String>,
}

/// SNIP-721 queries
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    //
    // Base SNIP-721 Queries
    //
    /// display the contract's name and symbol
    ContractInfo {},
    /// display the number of tokens controlled by the contract.  The token supply must
    /// either be public, or the querier must be an authenticated minter
    NumTokens {
        /// optional address and key requesting to view the number of tokens
        viewer: Option<ViewerInfo>,
    },
    /// display an optionally paginated list of all the tokens controlled by the contract.
    /// The token supply must either be public, or the querier must be authorized to view
    AllTokens {
        /// optional address and key requesting to view the list of tokens
        viewer: Option<ViewerInfo>,
        /// optionally display only token ids that come after the input String in
        /// lexicographical order
        start_after: Option<String>,
        /// optional number of token ids to display
        limit: Option<u32>,
    },
    /// display the owner of the specified token if authorized to view it.  If the requester
    /// is also the token's owner, the response will also include a list of any addresses
    /// that can transfer this token.  The transfer approval list is for CW721 compliance,
    /// but the [`NftDossier`](QueryMsg::NftDossier) query will be more complete by showing viewing approvals as well
    OwnerOf {
        token_id: String,
        /// optional address and key requesting to view the token owner
        viewer: Option<ViewerInfo>,
        /// optionally include expired [Approvals](Cw721Approval) in the response list.  If ommitted or
        /// false, expired [Approvals](Cw721Approval) will be filtered out of the response
        include_expired: Option<bool>,
    },
    /// displays the token's public metadata
    NftInfo { token_id: String },
    /// displays all the information contained in the [`OwnerOf`](QueryMsg::OwnerOf) and [`NftInfo`](QueryMsg::NftInfo) queries
    AllNftInfo {
        token_id: String,
        /// optional address and key requesting to view the token owner
        viewer: Option<ViewerInfo>,
        /// optionally include expired [Approvals](Cw721Approval) in the response list.  If ommitted or
        /// false, expired [Approvals](Cw721Approval) will be filtered out of the response
        include_expired: Option<bool>,
    },
    /// displays the token's private [`Metadata`](crate::metadata::Metadata)
    PrivateMetadata {
        token_id: String,
        /// optional address and key requesting to view the private metadata
        viewer: Option<ViewerInfo>,
    },
    /// displays all the information about a token that the viewer has permission to
    /// see.  This may include the owner, the public metadata, the private metadata, and
    /// the token and inventory approvals
    NftDossier {
        token_id: String,
        /// optional address and key requesting to view the token information
        viewer: Option<ViewerInfo>,
        /// optionally include expired [`Approvals`](Snip721Approval) in the response list.  If ommitted or
        /// false, expired [`Approvals`](Snip721Approval) will be filtered out of the response
        include_expired: Option<bool>,
    },
    /// list all the [`Approvals`](Snip721Approval) in place for a specified token if given the owner's viewing
    /// key
    TokenApprovals {
        token_id: String,
        /// the token owner's viewing key
        viewing_key: String,
        /// optionally include expired [`Approvals`](Snip721Approval) in the response list.  If ommitted or
        /// false, expired [`Approvals`](Snip721Approval) will be filtered out of the response
        include_expired: Option<bool>,
    },
    /// displays a list of all the CW721-style operators (any address that was granted
    /// approval to transfer all of the owner's tokens).  This query is provided to maintain
    /// CW-721 compliance, however, approvals are private on secret network, so only the
    /// owner's viewing key will authorize the ability to see the list of operators
    ApprovedForAll {
        owner: HumanAddr,
        /// optional viewing key to authenticate this query.  It is "optional" only in the
        /// sense that a CW721 query does not have this field.  However, not providing the
        /// key will always result in an empty list
        viewing_key: Option<String>,
        /// optionally include expired [`Approvals`](Cw721Approval) in the response list.  If ommitted or
        /// false, expired [`Approvals`](Cw721Approval) will be filtered out of the response
        include_expired: Option<bool>,
    },
    /// list all the inventory-wide [`Approvals`](Snip721Approval) in place for the specified address if given the
    /// the correct viewing key for the address
    InventoryApprovals {
        address: HumanAddr,
        /// the viewing key
        viewing_key: String,
        /// optionally include expired [`Approvals`](Snip721Approval) in the response list.  If ommitted or
        /// false, expired [`Approvals`](Snip721Approval) will be filtered out of the response
        include_expired: Option<bool>,
    },
    /// displays a list of all the tokens belonging to the input owner in which the viewer
    /// has view_owner permission
    Tokens {
        owner: HumanAddr,
        /// optional address of the querier if different from the owner
        viewer: Option<HumanAddr>,
        /// optional viewing key
        viewing_key: Option<String>,
        /// optionally display only token ids that come after the input String in
        /// lexicographical order
        start_after: Option<String>,
        /// optional number of token ids to display
        limit: Option<u32>,
    },
    /// display the transaction history for the specified address in reverse
    /// chronological order
    TransactionHistory {
        address: HumanAddr,
        /// viewing key
        viewing_key: String,
        /// optional page to display
        page: Option<u32>,
        /// optional number of transactions per page
        page_size: Option<u32>,
    },

    //
    // Optional Queries
    //
    /// display the list of authorized minters
    Minters {},
    /// display if a token is unwrapped
    IsUnwrapped { token_id: String },
    /// verify that the specified address has approval to transfer every listed token
    VerifyTransferApproval {
        /// list of tokens to verify approval for
        token_ids: Vec<String>,
        /// address that has approval
        address: HumanAddr,
        /// viewing key
        viewing_key: String,
    },
}

impl fmt::Display for QueryMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            QueryMsg::ContractInfo { .. } => write!(f, "ContractInfo"),
            QueryMsg::NumTokens { .. } => write!(f, "NumTokens"),
            QueryMsg::AllTokens { .. } => write!(f, "AllTokens"),
            QueryMsg::OwnerOf { .. } => write!(f, "OwnerOf"),
            QueryMsg::NftInfo { .. } => write!(f, "NftInfo"),
            QueryMsg::AllNftInfo { .. } => write!(f, "AllNftInfo"),
            QueryMsg::PrivateMetadata { .. } => write!(f, "PrivateMetadata"),
            QueryMsg::NftDossier { .. } => write!(f, "NftDossier"),
            QueryMsg::TokenApprovals { .. } => write!(f, "TokenApprovals"),
            QueryMsg::ApprovedForAll { .. } => write!(f, "ApprovedForAll"),
            QueryMsg::InventoryApprovals { .. } => write!(f, "InventoryApprovals"),
            QueryMsg::Tokens { .. } => write!(f, "Tokens"),
            QueryMsg::TransactionHistory { .. } => write!(f, "TransactionHistory"),
            QueryMsg::Minters { .. } => write!(f, "Minters"),
            QueryMsg::IsUnwrapped { .. } => write!(f, "IsUnwrapped"),
            QueryMsg::VerifyTransferApproval { .. } => write!(f, "VerifyTransferApproval"),
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

/// wrapper to deserialize [`ContractInfo`](ContractInfo) response
#[derive(Serialize, Deserialize)]
pub struct ContractInfoResponse {
    pub contract_info: ContractInfo,
}

/// wrapper to deserialize [`NumTokens`](NumTokens) response
#[derive(Serialize, Deserialize)]
pub struct NumTokensResponse {
    pub num_tokens: NumTokens,
}

/// wrapper to deserialize [`AllTokens`](TokenList) and [`Tokens`](TokenList) responses
#[derive(Serialize, Deserialize)]
pub struct TokenListResponse {
    pub token_list: TokenList,
}

/// wrapper to deserialize [`OwnerOf`](OwnerOf) responses
#[derive(Serialize, Deserialize)]
pub struct OwnerOfResponse {
    pub owner_of: OwnerOf,
}

/// wrapper to deserialize [`NftInfo`](crate::metadata::Metadata) responses
#[derive(Serialize, Deserialize)]
pub struct NftInfoResponse {
    pub nft_info: Metadata,
}

/// wrapper to deserialize [`AllNftInfo`](AllNftInfo) responses
#[derive(Serialize, Deserialize)]
pub struct AllNftInfoResponse {
    pub all_nft_info: AllNftInfo,
}

/// wrapper to deserialize [`PrivateMetadata`](crate::metadata::Metadata) responses
#[derive(Serialize, Deserialize)]
pub struct PrivateMetadataResponse {
    pub private_metadata: Metadata,
}

/// wrapper to deserialize [`NftDossier`](NftDossier) responses
#[derive(Serialize, Deserialize)]
pub struct NftDossierResponse {
    pub nft_dossier: NftDossier,
}

/// wrapper to deserialize [`TokenApprovals`](TokenApprovals) responses
#[derive(Serialize, Deserialize)]
pub struct TokenApprovalsResponse {
    pub token_approvals: TokenApprovals,
}

/// wrapper to deserialize [`ApprovedForAll`](ApprovedForAll) responses
#[derive(Serialize, Deserialize)]
pub struct ApprovedForAllResponse {
    pub approved_for_all: ApprovedForAll,
}

/// wrapper to deserialize [`InventoryApprovals`](InventoryApprovals) responses
#[derive(Serialize, Deserialize)]
pub struct InventoryApprovalsResponse {
    pub inventory_approvals: InventoryApprovals,
}

/// wrapper to deserialize [`TransactionHistory`](TransactionHistory) response
#[derive(Serialize, Deserialize)]
pub struct TransactionHistoryResponse {
    pub transaction_history: TransactionHistory,
}

/// wrapper to deserialize [`Minters`](Minters) response
#[derive(Serialize, Deserialize)]
pub struct MintersResponse {
    pub minters: Minters,
}

/// wrapper to deserialize [`IsUnwrapped`](IsUnwrapped) response
#[derive(Serialize, Deserialize)]
pub struct IsUnwrappedResponse {
    pub is_unwrapped: IsUnwrapped,
}

/// wrapper to deserialize [`VerifyTransferApproval`](VerifyTransferApproval) response
#[derive(Serialize, Deserialize)]
pub struct VerifyTransferApprovalResponse {
    pub verify_transfer_approval: VerifyTransferApproval,
}

/// Returns a StdResult<[`ContractInfo`](ContractInfo)> from performing [`ContractInfo`](QueryMsg::ContractInfo) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn contract_info_query<Q: Querier>(
    querier: &Q,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<ContractInfo> {
    let answer: ContractInfoResponse =
        QueryMsg::ContractInfo {}.query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.contract_info)
}

/// Returns a StdResult<[`NumTokens`](NumTokens)> from performing [`NumTokens`](QueryMsg::NumTokens) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `viewer` - Optional ViewerInfo holding the address and viewing key of the querier
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn num_tokens_query<Q: Querier>(
    querier: &Q,
    viewer: Option<ViewerInfo>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<NumTokens> {
    let answer: NumTokensResponse = QueryMsg::NumTokens { viewer }.query(
        querier,
        block_size,
        callback_code_hash,
        contract_addr,
    )?;
    Ok(answer.num_tokens)
}

/// Returns a StdResult<[`TokenList`](TokenList)> from performing [`AllTokens`](QueryMsg::AllTokens) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `viewer` - Optional ViewerInfo holding the address and viewing key of the querier
/// * `start_after` - Optionally display only token ids that come after this String in
///                   lexicographical order
/// * `limit` - Optional u32 number of token ids to display
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn all_tokens_query<Q: Querier>(
    querier: &Q,
    viewer: Option<ViewerInfo>,
    start_after: Option<String>,
    limit: Option<u32>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<TokenList> {
    let answer: TokenListResponse = QueryMsg::AllTokens {
        viewer,
        start_after,
        limit,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.token_list)
}

/// Returns a StdResult<[`OwnerOf`](OwnerOf)> from performing [`OwnerOf`](QueryMsg::OwnerOf) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `token_id` - ID of the token whose info is being requested
/// * `viewer` - Optional ViewerInfo holding the address and viewing key of the querier
/// * `include_expired` - Optionally include expired Approvals in the response list.  If
///                       ommitted or false, expired Approvals will be filtered out of
///                       the response
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn owner_of_query<Q: Querier>(
    querier: &Q,
    token_id: String,
    viewer: Option<ViewerInfo>,
    include_expired: Option<bool>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<OwnerOf> {
    let answer: OwnerOfResponse = QueryMsg::OwnerOf {
        token_id,
        viewer,
        include_expired,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.owner_of)
}

/// Returns a StdResult<[`Metadata`](crate::metadata::Metadata)> from performing [`NftInfo`](QueryMsg::NftInfo) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `token_id` - ID of the token whose info is being requested
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn nft_info_query<Q: Querier>(
    querier: &Q,
    token_id: String,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<Metadata> {
    let answer: NftInfoResponse = QueryMsg::NftInfo { token_id }.query(
        querier,
        block_size,
        callback_code_hash,
        contract_addr,
    )?;
    Ok(answer.nft_info)
}

/// Returns a StdResult<[`AllNftInfo`](AllNftInfo)> from performing [`AllNftInfo`](QueryMsg::AllNftInfo) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `token_id` - ID of the token whose info is being requested
/// * `viewer` - Optional ViewerInfo holding the address and viewing key of the querier
/// * `include_expired` - Optionally include expired Approvals in the response list.  If
///                       ommitted or false, expired Approvals will be filtered out of
///                       the response
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn all_nft_info_query<Q: Querier>(
    querier: &Q,
    token_id: String,
    viewer: Option<ViewerInfo>,
    include_expired: Option<bool>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<AllNftInfo> {
    let answer: AllNftInfoResponse = QueryMsg::AllNftInfo {
        token_id,
        viewer,
        include_expired,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.all_nft_info)
}

/// Returns a StdResult<[`Metadata`](crate::metadata::Metadata)> from performing [`PrivateMetadata`](QueryMsg::PrivateMetadata) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `token_id` - ID of the token whose info is being requested
/// * `viewer` - Optional ViewerInfo holding the address and viewing key of the querier
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn private_metadata_query<Q: Querier>(
    querier: &Q,
    token_id: String,
    viewer: Option<ViewerInfo>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<Metadata> {
    let answer: PrivateMetadataResponse = QueryMsg::PrivateMetadata { token_id, viewer }.query(
        querier,
        block_size,
        callback_code_hash,
        contract_addr,
    )?;
    Ok(answer.private_metadata)
}

/// Returns a StdResult<[`NftDossier`](NftDossier)> from performing [`NftDossier`](QueryMsg::NftDossier) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `token_id` - ID of the token whose info is being requested
/// * `viewer` - Optional ViewerInfo holding the address and viewing key of the querier
/// * `include_expired` - Optionally include expired Approvals in the response list.  If
///                       ommitted or false, expired Approvals will be filtered out of
///                       the response
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn nft_dossier_query<Q: Querier>(
    querier: &Q,
    token_id: String,
    viewer: Option<ViewerInfo>,
    include_expired: Option<bool>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<NftDossier> {
    let answer: NftDossierResponse = QueryMsg::NftDossier {
        token_id,
        viewer,
        include_expired,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.nft_dossier)
}

/// Returns a StdResult<[`TokenApprovals`](TokenApprovals)> from performing [`TokenApprovals`](QueryMsg::TokenApprovals) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `token_id` - ID of the token whose info is being requested
/// * `viewing_key` - String holding the viewing key of the token's owner
/// * `include_expired` - Optionally include expired Approvals in the response list.  If
///                       ommitted or false, expired Approvals will be filtered out of
///                       the response
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn token_approvals_query<Q: Querier>(
    querier: &Q,
    token_id: String,
    viewing_key: String,
    include_expired: Option<bool>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<TokenApprovals> {
    let answer: TokenApprovalsResponse = QueryMsg::TokenApprovals {
        token_id,
        viewing_key,
        include_expired,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.token_approvals)
}

/// Returns a StdResult<[`ApprovedForAll`](ApprovedForAll)> from performing [`ApprovedForAll`](QueryMsg::ApprovedForAll) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `owner` - the address whose approvals are being requested
/// * `viewing_key` - Optional String holding the viewing key of the owner
/// * `include_expired` - Optionally include expired Approvals in the response list.  If
///                       ommitted or false, expired Approvals will be filtered out of
///                       the response
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn approved_for_all_query<Q: Querier>(
    querier: &Q,
    owner: HumanAddr,
    viewing_key: Option<String>,
    include_expired: Option<bool>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<ApprovedForAll> {
    let answer: ApprovedForAllResponse = QueryMsg::ApprovedForAll {
        owner,
        viewing_key,
        include_expired,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.approved_for_all)
}

/// Returns a StdResult<[`InventoryApprovals`](InventoryApprovals)> from performing [`InventoryApprovals`](QueryMsg::InventoryApprovals) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `address` - the address whose approvals are being requested
/// * `viewing_key` - String holding the viewing key of the specified address
/// * `include_expired` - Optionally include expired Approvals in the response list.  If
///                       ommitted or false, expired Approvals will be filtered out of
///                       the response
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn inventory_approvals_query<Q: Querier>(
    querier: &Q,
    address: HumanAddr,
    viewing_key: String,
    include_expired: Option<bool>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<InventoryApprovals> {
    let answer: InventoryApprovalsResponse = QueryMsg::InventoryApprovals {
        address,
        viewing_key,
        include_expired,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.inventory_approvals)
}

/// Returns a StdResult<[`TokenList`](TokenList)> from performing [`Tokens`](QueryMsg::Tokens) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `owner` - the address whose token inventory is being requested
/// * `viewer` - Optional address of the querier if different from the owner
/// * `viewing_key` - Optional String holding the viewing key of the querier
/// * `start_after` - Optionally display only token ids that come after this String in
///                   lexicographical order
/// * `limit` - Optional u32 number of token ids to display
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
#[allow(clippy::too_many_arguments)]
pub fn tokens_query<Q: Querier>(
    querier: &Q,
    owner: HumanAddr,
    viewer: Option<HumanAddr>,
    viewing_key: Option<String>,
    start_after: Option<String>,
    limit: Option<u32>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<TokenList> {
    let answer: TokenListResponse = QueryMsg::Tokens {
        owner,
        viewer,
        viewing_key,
        start_after,
        limit,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.token_list)
}

/// Returns a StdResult<[`TransactionHistory`](TransactionHistory)> from performing [`TransactionHistory`](QueryMsg::TransactionHistory) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `address` - the address whose transaction history should be displayed
/// * `viewing_key` - String holding the authentication key needed to view transactions
/// * `page` - Optional u32 representing the page number of transactions to display
/// * `page_size` - Optional u32 number of transactions to return
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
#[allow(clippy::too_many_arguments)]
pub fn transaction_history_query<Q: Querier>(
    querier: &Q,
    address: HumanAddr,
    viewing_key: String,
    page: Option<u32>,
    page_size: Option<u32>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<TransactionHistory> {
    let answer: TransactionHistoryResponse = QueryMsg::TransactionHistory {
        address,
        viewing_key,
        page,
        page_size,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.transaction_history)
}

/// Returns a StdResult<[`Minters`](Minters)> from performing [`Minters`](QueryMsg::Minters) query
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

/// Returns a StdResult<[`IsUnwrapped`](IsUnwrapped)> from performing [`IsUnwrapped`](QueryMsg::IsUnwrapped) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `token_id` - ID of the token whose info is being requested
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn is_unwrapped_query<Q: Querier>(
    querier: &Q,
    token_id: String,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<IsUnwrapped> {
    let answer: IsUnwrappedResponse = QueryMsg::IsUnwrapped { token_id }.query(
        querier,
        block_size,
        callback_code_hash,
        contract_addr,
    )?;
    Ok(answer.is_unwrapped)
}

/// Returns a StdResult<[`VerifyTransferApproval`](VerifyTransferApproval)> from performing [`VerifyTransferApproval`](QueryMsg::VerifyTransferApproval) query
///
/// # Arguments
///
/// * `querier` - a reference to the Querier dependency of the querying contract
/// * `token_ids` - list of tokens to verify approval for
/// * `address` - address that has transfer approval
/// * `viewing_key` - String holding the address' viewing key
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being queried
/// * `contract_addr` - address of the contract being queried
pub fn verify_transfer_approval_query<Q: Querier>(
    querier: &Q,
    token_ids: Vec<String>,
    address: HumanAddr,
    viewing_key: String,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<VerifyTransferApproval> {
    let answer: VerifyTransferApprovalResponse = QueryMsg::VerifyTransferApproval {
        token_ids,
        address,
        viewing_key,
    }
    .query(querier, block_size, callback_code_hash, contract_addr)?;
    Ok(answer.verify_transfer_approval)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{to_vec, QuerierResult, SystemError};

    #[test]
    fn test_contract_info_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let mut expected_msg =
                    to_binary(&QueryMsg::ContractInfo {}).map_err(|_e| SystemError::Unknown {})?;
                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);
                let response = ContractInfoResponse {
                    contract_info: ContractInfo {
                        name: "NFTs".to_string(),
                        symbol: "NFTS".to_string(),
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let expected_response = ContractInfo {
            name: "NFTs".to_string(),
            symbol: "NFTS".to_string(),
        };

        let response = contract_info_query(&querier, 256usize, hash, address)?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_num_tokens_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let viewer = Some(ViewerInfo {
                    address: HumanAddr("alice".to_string()),
                    viewing_key: "key".to_string(),
                });
                let mut expected_msg = to_binary(&QueryMsg::NumTokens { viewer })
                    .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = NumTokensResponse {
                    num_tokens: NumTokens { count: 32 },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let viewer = Some(ViewerInfo {
            address: HumanAddr("alice".to_string()),
            viewing_key: "key".to_string(),
        });

        let expected_response = NumTokens { count: 32 };

        let response = num_tokens_query(&querier, viewer, 256usize, hash, address)?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_all_tokens_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let viewer = Some(ViewerInfo {
                    address: HumanAddr("alice".to_string()),
                    viewing_key: "key".to_string(),
                });
                let start_after = Some("NFT1".to_string());
                let limit = None;
                let mut expected_msg = to_binary(&QueryMsg::AllTokens {
                    viewer,
                    start_after,
                    limit,
                })
                .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = TokenListResponse {
                    token_list: TokenList {
                        tokens: vec!["NFT2".to_string(), "NFT3".to_string(), "NFT4".to_string()],
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let viewer = Some(ViewerInfo {
            address: HumanAddr("alice".to_string()),
            viewing_key: "key".to_string(),
        });
        let start_after = Some("NFT1".to_string());
        let limit = None;

        let expected_response = TokenList {
            tokens: vec!["NFT2".to_string(), "NFT3".to_string(), "NFT4".to_string()],
        };

        let response = all_tokens_query(
            &querier,
            viewer,
            start_after,
            limit,
            256usize,
            hash,
            address,
        )?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_owner_of_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let viewer = Some(ViewerInfo {
                    address: HumanAddr("alice".to_string()),
                    viewing_key: "key".to_string(),
                });
                let token_id = "NFT1".to_string();
                let include_expired = Some(true);
                let mut expected_msg = to_binary(&QueryMsg::OwnerOf {
                    token_id,
                    viewer,
                    include_expired,
                })
                .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = OwnerOfResponse {
                    owner_of: OwnerOf {
                        owner: Some(HumanAddr("alice".to_string())),
                        approvals: vec![
                            Cw721Approval {
                                spender: HumanAddr("bob".to_string()),
                                expires: Expiration::Never,
                            },
                            Cw721Approval {
                                spender: HumanAddr("charlie".to_string()),
                                expires: Expiration::AtHeight(1000000),
                            },
                        ],
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let viewer = Some(ViewerInfo {
            address: HumanAddr("alice".to_string()),
            viewing_key: "key".to_string(),
        });
        let token_id = "NFT1".to_string();
        let include_expired = Some(true);

        let expected_response = OwnerOf {
            owner: Some(HumanAddr("alice".to_string())),
            approvals: vec![
                Cw721Approval {
                    spender: HumanAddr("bob".to_string()),
                    expires: Expiration::Never,
                },
                Cw721Approval {
                    spender: HumanAddr("charlie".to_string()),
                    expires: Expiration::AtHeight(1000000),
                },
            ],
        };

        let response = owner_of_query(
            &querier,
            token_id,
            viewer,
            include_expired,
            256usize,
            hash,
            address,
        )?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_nft_info_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let token_id = "NFT1".to_string();
                let mut expected_msg = to_binary(&QueryMsg::NftInfo { token_id })
                    .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = NftInfoResponse {
                    nft_info: Metadata {
                        name: Some("NFT1".to_string()),
                        description: Some("description".to_string()),
                        image: Some("image".to_string()),
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let token_id = "NFT1".to_string();

        let expected_response = Metadata {
            name: Some("NFT1".to_string()),
            description: Some("description".to_string()),
            image: Some("image".to_string()),
        };

        let response = nft_info_query(&querier, token_id, 256usize, hash, address)?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_all_nft_info_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let viewer = Some(ViewerInfo {
                    address: HumanAddr("alice".to_string()),
                    viewing_key: "key".to_string(),
                });
                let token_id = "NFT1".to_string();
                let include_expired = Some(true);
                let mut expected_msg = to_binary(&QueryMsg::AllNftInfo {
                    token_id,
                    viewer,
                    include_expired,
                })
                .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = AllNftInfoResponse {
                    all_nft_info: AllNftInfo {
                        access: OwnerOf {
                            owner: Some(HumanAddr("alice".to_string())),
                            approvals: vec![
                                Cw721Approval {
                                    spender: HumanAddr("bob".to_string()),
                                    expires: Expiration::Never,
                                },
                                Cw721Approval {
                                    spender: HumanAddr("charlie".to_string()),
                                    expires: Expiration::AtHeight(1000000),
                                },
                            ],
                        },
                        info: Some(Metadata {
                            name: Some("NFT1".to_string()),
                            description: Some("description".to_string()),
                            image: Some("image".to_string()),
                        }),
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let viewer = Some(ViewerInfo {
            address: HumanAddr("alice".to_string()),
            viewing_key: "key".to_string(),
        });
        let token_id = "NFT1".to_string();
        let include_expired = Some(true);

        let expected_response = AllNftInfo {
            access: OwnerOf {
                owner: Some(HumanAddr("alice".to_string())),
                approvals: vec![
                    Cw721Approval {
                        spender: HumanAddr("bob".to_string()),
                        expires: Expiration::Never,
                    },
                    Cw721Approval {
                        spender: HumanAddr("charlie".to_string()),
                        expires: Expiration::AtHeight(1000000),
                    },
                ],
            },
            info: Some(Metadata {
                name: Some("NFT1".to_string()),
                description: Some("description".to_string()),
                image: Some("image".to_string()),
            }),
        };

        let response = all_nft_info_query(
            &querier,
            token_id,
            viewer,
            include_expired,
            256usize,
            hash,
            address,
        )?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_private_metadata_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let token_id = "NFT1".to_string();
                let viewer = Some(ViewerInfo {
                    address: HumanAddr("alice".to_string()),
                    viewing_key: "key".to_string(),
                });
                let mut expected_msg = to_binary(&QueryMsg::PrivateMetadata { token_id, viewer })
                    .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = PrivateMetadataResponse {
                    private_metadata: Metadata {
                        name: Some("NFT1".to_string()),
                        description: Some("description".to_string()),
                        image: Some("image".to_string()),
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let token_id = "NFT1".to_string();
        let viewer = Some(ViewerInfo {
            address: HumanAddr("alice".to_string()),
            viewing_key: "key".to_string(),
        });

        let expected_response = Metadata {
            name: Some("NFT1".to_string()),
            description: Some("description".to_string()),
            image: Some("image".to_string()),
        };

        let response = private_metadata_query(&querier, token_id, viewer, 256usize, hash, address)?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_nft_dossier_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let viewer = Some(ViewerInfo {
                    address: HumanAddr("alice".to_string()),
                    viewing_key: "key".to_string(),
                });
                let token_id = "NFT1".to_string();
                let include_expired = Some(true);
                let mut expected_msg = to_binary(&QueryMsg::NftDossier {
                    token_id,
                    viewer,
                    include_expired,
                })
                .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = NftDossierResponse {
                    nft_dossier: NftDossier {
                        owner: Some(HumanAddr("alice".to_string())),
                        public_metadata: Some(Metadata {
                            name: Some("NFT1".to_string()),
                            description: Some("description".to_string()),
                            image: Some("image".to_string()),
                        }),
                        private_metadata: None,
                        display_private_metadata_error: Some("pretend it is sealed".to_string()),
                        owner_is_public: true,
                        public_ownership_expiration: Some(Expiration::Never),
                        private_metadata_is_public: false,
                        private_metadata_is_public_expiration: None,
                        token_approvals: Some(vec![
                            Snip721Approval {
                                address: HumanAddr("bob".to_string()),
                                view_owner_expiration: None,
                                view_private_metadata_expiration: Some(Expiration::AtTime(1000000)),
                                transfer_expiration: Some(Expiration::AtHeight(10000)),
                            },
                            Snip721Approval {
                                address: HumanAddr("charlie".to_string()),
                                view_owner_expiration: Some(Expiration::Never),
                                view_private_metadata_expiration: None,
                                transfer_expiration: None,
                            },
                        ]),
                        inventory_approvals: None,
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let viewer = Some(ViewerInfo {
            address: HumanAddr("alice".to_string()),
            viewing_key: "key".to_string(),
        });
        let token_id = "NFT1".to_string();
        let include_expired = Some(true);

        let expected_response = NftDossier {
            owner: Some(HumanAddr("alice".to_string())),
            public_metadata: Some(Metadata {
                name: Some("NFT1".to_string()),
                description: Some("description".to_string()),
                image: Some("image".to_string()),
            }),
            private_metadata: None,
            display_private_metadata_error: Some("pretend it is sealed".to_string()),
            owner_is_public: true,
            public_ownership_expiration: Some(Expiration::Never),
            private_metadata_is_public: false,
            private_metadata_is_public_expiration: None,
            token_approvals: Some(vec![
                Snip721Approval {
                    address: HumanAddr("bob".to_string()),
                    view_owner_expiration: None,
                    view_private_metadata_expiration: Some(Expiration::AtTime(1000000)),
                    transfer_expiration: Some(Expiration::AtHeight(10000)),
                },
                Snip721Approval {
                    address: HumanAddr("charlie".to_string()),
                    view_owner_expiration: Some(Expiration::Never),
                    view_private_metadata_expiration: None,
                    transfer_expiration: None,
                },
            ]),
            inventory_approvals: None,
        };

        let response = nft_dossier_query(
            &querier,
            token_id,
            viewer,
            include_expired,
            256usize,
            hash,
            address,
        )?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_token_approvals_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let viewing_key = "key".to_string();
                let token_id = "NFT1".to_string();
                let include_expired = None;
                let mut expected_msg = to_binary(&QueryMsg::TokenApprovals {
                    token_id,
                    viewing_key,
                    include_expired,
                })
                .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = TokenApprovalsResponse {
                    token_approvals: TokenApprovals {
                        owner_is_public: true,
                        public_ownership_expiration: Some(Expiration::Never),
                        private_metadata_is_public: false,
                        private_metadata_is_public_expiration: None,
                        token_approvals: vec![
                            Snip721Approval {
                                address: HumanAddr("bob".to_string()),
                                view_owner_expiration: None,
                                view_private_metadata_expiration: Some(Expiration::AtTime(1000000)),
                                transfer_expiration: Some(Expiration::AtHeight(10000)),
                            },
                            Snip721Approval {
                                address: HumanAddr("charlie".to_string()),
                                view_owner_expiration: Some(Expiration::Never),
                                view_private_metadata_expiration: None,
                                transfer_expiration: None,
                            },
                        ],
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let viewing_key = "key".to_string();
        let token_id = "NFT1".to_string();
        let include_expired = None;

        let expected_response = TokenApprovals {
            owner_is_public: true,
            public_ownership_expiration: Some(Expiration::Never),
            private_metadata_is_public: false,
            private_metadata_is_public_expiration: None,
            token_approvals: vec![
                Snip721Approval {
                    address: HumanAddr("bob".to_string()),
                    view_owner_expiration: None,
                    view_private_metadata_expiration: Some(Expiration::AtTime(1000000)),
                    transfer_expiration: Some(Expiration::AtHeight(10000)),
                },
                Snip721Approval {
                    address: HumanAddr("charlie".to_string()),
                    view_owner_expiration: Some(Expiration::Never),
                    view_private_metadata_expiration: None,
                    transfer_expiration: None,
                },
            ],
        };

        let response = token_approvals_query(
            &querier,
            token_id,
            viewing_key,
            include_expired,
            256usize,
            hash,
            address,
        )?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_approved_for_all_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let viewing_key = Some("key".to_string());
                let owner = HumanAddr("alice".to_string());
                let include_expired = None;
                let mut expected_msg = to_binary(&QueryMsg::ApprovedForAll {
                    owner,
                    viewing_key,
                    include_expired,
                })
                .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = ApprovedForAllResponse {
                    approved_for_all: ApprovedForAll {
                        operators: vec![
                            Cw721Approval {
                                spender: HumanAddr("bob".to_string()),
                                expires: Expiration::Never,
                            },
                            Cw721Approval {
                                spender: HumanAddr("charlie".to_string()),
                                expires: Expiration::AtHeight(1000000),
                            },
                        ],
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let viewing_key = Some("key".to_string());
        let owner = HumanAddr("alice".to_string());
        let include_expired = None;

        let expected_response = ApprovedForAll {
            operators: vec![
                Cw721Approval {
                    spender: HumanAddr("bob".to_string()),
                    expires: Expiration::Never,
                },
                Cw721Approval {
                    spender: HumanAddr("charlie".to_string()),
                    expires: Expiration::AtHeight(1000000),
                },
            ],
        };

        let response = approved_for_all_query(
            &querier,
            owner,
            viewing_key,
            include_expired,
            256usize,
            hash,
            address,
        )?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_inventory_approvals_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let viewing_key = "key".to_string();
                let address = HumanAddr("alice".to_string());
                let include_expired = None;
                let mut expected_msg = to_binary(&QueryMsg::InventoryApprovals {
                    address,
                    viewing_key,
                    include_expired,
                })
                .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = InventoryApprovalsResponse {
                    inventory_approvals: InventoryApprovals {
                        owner_is_public: true,
                        public_ownership_expiration: Some(Expiration::Never),
                        private_metadata_is_public: false,
                        private_metadata_is_public_expiration: None,
                        inventory_approvals: vec![
                            Snip721Approval {
                                address: HumanAddr("bob".to_string()),
                                view_owner_expiration: None,
                                view_private_metadata_expiration: Some(Expiration::AtTime(1000000)),
                                transfer_expiration: Some(Expiration::AtHeight(10000)),
                            },
                            Snip721Approval {
                                address: HumanAddr("charlie".to_string()),
                                view_owner_expiration: Some(Expiration::Never),
                                view_private_metadata_expiration: None,
                                transfer_expiration: None,
                            },
                        ],
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let contract_address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let viewing_key = "key".to_string();
        let address = HumanAddr("alice".to_string());
        let include_expired = None;

        let expected_response = InventoryApprovals {
            owner_is_public: true,
            public_ownership_expiration: Some(Expiration::Never),
            private_metadata_is_public: false,
            private_metadata_is_public_expiration: None,
            inventory_approvals: vec![
                Snip721Approval {
                    address: HumanAddr("bob".to_string()),
                    view_owner_expiration: None,
                    view_private_metadata_expiration: Some(Expiration::AtTime(1000000)),
                    transfer_expiration: Some(Expiration::AtHeight(10000)),
                },
                Snip721Approval {
                    address: HumanAddr("charlie".to_string()),
                    view_owner_expiration: Some(Expiration::Never),
                    view_private_metadata_expiration: None,
                    transfer_expiration: None,
                },
            ],
        };

        let response = inventory_approvals_query(
            &querier,
            address,
            viewing_key,
            include_expired,
            256usize,
            hash,
            contract_address,
        )?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_tokens_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let owner = HumanAddr("alice".to_string());
                let viewer = Some(HumanAddr("bob".to_string()));
                let viewing_key = Some("key".to_string());
                let start_after = Some("NFT1".to_string());
                let limit = Some(33);
                let mut expected_msg = to_binary(&QueryMsg::Tokens {
                    owner,
                    viewer,
                    viewing_key,
                    start_after,
                    limit,
                })
                .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = TokenListResponse {
                    token_list: TokenList {
                        tokens: vec!["NFT2".to_string(), "NFT3".to_string(), "NFT4".to_string()],
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let owner = HumanAddr("alice".to_string());
        let viewer = Some(HumanAddr("bob".to_string()));
        let viewing_key = Some("key".to_string());
        let start_after = Some("NFT1".to_string());
        let limit = Some(33);

        let expected_response = TokenList {
            tokens: vec!["NFT2".to_string(), "NFT3".to_string(), "NFT4".to_string()],
        };

        let response = tokens_query(
            &querier,
            owner,
            viewer,
            viewing_key,
            start_after,
            limit,
            256usize,
            hash,
            address,
        )?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_transaction_history_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let address = HumanAddr("alice".to_string());
                let viewing_key = "key".to_string();
                let page = Some(2);
                let page_size = None;
                let mut expected_msg = to_binary(&QueryMsg::TransactionHistory {
                    address,
                    viewing_key,
                    page,
                    page_size,
                })
                .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = TransactionHistoryResponse {
                    transaction_history: TransactionHistory {
                        total: 3,
                        txs: vec![
                            Tx {
                                tx_id: 103,
                                block_height: 2000000,
                                block_time: 2000000000,
                                token_id: "NFT3".to_string(),
                                action: TxAction::Burn {
                                    owner: HumanAddr("alice".to_string()),
                                    burner: Some(HumanAddr("bob".to_string())),
                                },
                                memo: None,
                            },
                            Tx {
                                tx_id: 99,
                                block_height: 1900000,
                                block_time: 1900000000,
                                token_id: "NFT2".to_string(),
                                action: TxAction::Transfer {
                                    from: HumanAddr("alice".to_string()),
                                    sender: None,
                                    recipient: HumanAddr("bob".to_string()),
                                },
                                memo: Some("xfer memo".to_string()),
                            },
                            Tx {
                                tx_id: 93,
                                block_height: 1800000,
                                block_time: 1800000000,
                                token_id: "NFT1".to_string(),
                                action: TxAction::Mint {
                                    minter: HumanAddr("admin".to_string()),
                                    recipient: HumanAddr("alice".to_string()),
                                },
                                memo: None,
                            },
                        ],
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let contract_address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let address = HumanAddr("alice".to_string());
        let viewing_key = "key".to_string();
        let page = Some(2);
        let page_size = None;

        let expected_response = TransactionHistory {
            total: 3,
            txs: vec![
                Tx {
                    tx_id: 103,
                    block_height: 2000000,
                    block_time: 2000000000,
                    token_id: "NFT3".to_string(),
                    action: TxAction::Burn {
                        owner: HumanAddr("alice".to_string()),
                        burner: Some(HumanAddr("bob".to_string())),
                    },
                    memo: None,
                },
                Tx {
                    tx_id: 99,
                    block_height: 1900000,
                    block_time: 1900000000,
                    token_id: "NFT2".to_string(),
                    action: TxAction::Transfer {
                        from: HumanAddr("alice".to_string()),
                        sender: None,
                        recipient: HumanAddr("bob".to_string()),
                    },
                    memo: Some("xfer memo".to_string()),
                },
                Tx {
                    tx_id: 93,
                    block_height: 1800000,
                    block_time: 1800000000,
                    token_id: "NFT1".to_string(),
                    action: TxAction::Mint {
                        minter: HumanAddr("admin".to_string()),
                        recipient: HumanAddr("alice".to_string()),
                    },
                    memo: None,
                },
            ],
        };

        let response = transaction_history_query(
            &querier,
            address,
            viewing_key,
            page,
            page_size,
            256usize,
            hash,
            contract_address,
        )?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_minters_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let mut expected_msg =
                    to_binary(&QueryMsg::Minters {}).map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = MintersResponse {
                    minters: Minters {
                        minters: vec![
                            HumanAddr("alice".to_string()),
                            HumanAddr("bob".to_string()),
                            HumanAddr("charlie".to_string()),
                        ],
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let expected_response = Minters {
            minters: vec![
                HumanAddr("alice".to_string()),
                HumanAddr("bob".to_string()),
                HumanAddr("charlie".to_string()),
            ],
        };

        let response = minters_query(&querier, 256usize, hash, address)?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_is_unwrapped_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let token_id = "NFT1".to_string();
                let mut expected_msg = to_binary(&QueryMsg::IsUnwrapped { token_id })
                    .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = IsUnwrappedResponse {
                    is_unwrapped: IsUnwrapped {
                        token_is_unwrapped: false,
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let token_id = "NFT1".to_string();

        let expected_response = IsUnwrapped {
            token_is_unwrapped: false,
        };

        let response = is_unwrapped_query(&querier, token_id, 256usize, hash, address)?;
        assert_eq!(response, expected_response);

        Ok(())
    }

    #[test]
    fn test_verify_transfer_approval_query() -> StdResult<()> {
        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let token_ids = vec!["NFT1".to_string(), "NFT2".to_string(), "NFT3".to_string()];
                let address = HumanAddr("alice".to_string());
                let viewing_key = "key".to_string();

                let mut expected_msg = to_binary(&QueryMsg::VerifyTransferApproval {
                    token_ids,
                    address,
                    viewing_key,
                })
                .map_err(|_e| SystemError::Unknown {})?;

                space_pad(&mut expected_msg.0, 256);
                let expected_request: QueryRequest<QueryMsg> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: HumanAddr("contract".to_string()),
                        callback_code_hash: "code hash".to_string(),
                        msg: expected_msg,
                    });
                let test_req: &[u8] =
                    &to_vec(&expected_request).map_err(|_e| SystemError::Unknown {})?;
                assert_eq!(request, test_req);

                let response = VerifyTransferApprovalResponse {
                    verify_transfer_approval: VerifyTransferApproval {
                        approved_for_all: false,
                        first_unapproved_token: Some("NFT3".to_string()),
                    },
                };
                Ok(to_binary(&response))
            }
        }

        let querier = MyMockQuerier {};
        let contract_address = HumanAddr("contract".to_string());
        let hash = "code hash".to_string();

        let token_ids = vec!["NFT1".to_string(), "NFT2".to_string(), "NFT3".to_string()];
        let address = HumanAddr("alice".to_string());
        let viewing_key = "key".to_string();

        let expected_response = VerifyTransferApproval {
            approved_for_all: false,
            first_unapproved_token: Some("NFT3".to_string()),
        };

        let response = verify_transfer_approval_query(
            &querier,
            token_ids,
            address,
            viewing_key,
            256usize,
            hash,
            contract_address,
        )?;
        assert_eq!(response, expected_response);

        Ok(())
    }
}
