use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Binary, Coin, CosmosMsg, HumanAddr, StdResult, Uint128, WasmMsg};

use crate::expiration::Expiration;
use crate::metadata::Metadata;

use secret_toolkit_utils::space_pad;

//
// Structures Used for Input Parameters
//

/// permission access level
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    /// approve permission only for the specified token
    ApproveToken,
    /// grant permission for all tokens
    All,
    /// revoke permission only for the specified token
    RevokeToken,
    /// remove all permissions for this address
    None,
}

//
// structs used for optional batch processing as implemented in the reference
// contract
//

/// token mint info used when doing a [`BatchMintNft`](HandleMsg::BatchMintNft)
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct Mint {
    /// optional token id. if omitted, use current token index
    pub token_id: Option<String>,
    /// optional owner address. if omitted, owned by the message sender
    pub owner: Option<HumanAddr>,
    /// optional public metadata that can be seen by everyone
    pub public_metadata: Option<Metadata>,
    /// optional private metadata that can only be seen by the owner and whitelist
    pub private_metadata: Option<Metadata>,
    /// optional memo for the tx
    pub memo: Option<String>,
}

/// token burn info used when doing a [`BatchBurnNft`](HandleMsg::BatchBurnNft)
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct Burn {
    /// tokens being burnt
    pub token_ids: Vec<String>,
    /// optional memo for the tx
    pub memo: Option<String>,
}

/// token transfer info used when doing a [`BatchTransferNft`](HandleMsg::BatchTransferNft)
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct Transfer {
    /// recipient of the transferred tokens
    pub recipient: HumanAddr,
    /// tokens being transferred
    pub token_ids: Vec<String>,
    /// optional memo for the tx
    pub memo: Option<String>,
}

/// send token info used when doing a [`BatchSendNft`](HandleMsg::BatchSendNft)
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct Send {
    /// recipient of the sent tokens
    pub contract: HumanAddr,
    /// tokens being sent
    pub token_ids: Vec<String>,
    /// optional message to send with the (Batch)RecieveNft callback
    pub msg: Option<Binary>,
    /// optional memo for the tx
    pub memo: Option<String>,
}

/// SNIP-721 contract handle messages
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    //
    // Base SNIP-721 Messages
    //
    /// transfer a token
    TransferNft {
        /// recipient of the transfer
        recipient: HumanAddr,
        /// id of the token to transfer
        token_id: String,
        /// optional memo for the tx
        memo: Option<String>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// send a token and call receiving contract's (Batch)ReceiveNft
    SendNft {
        /// address to send the token to
        contract: HumanAddr,
        /// id of the token to send
        token_id: String,
        /// optional message to send with the (Batch)RecieveNft callback
        msg: Option<Binary>,
        /// optional memo for the tx
        memo: Option<String>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// gives the spender permission to transfer the specified token.  If you are the owner
    /// of the token, you can use [`SetWhitelistedApproval`](HandleMsg::SetWhitelistedApproval) to accomplish the same thing.  If
    /// you are an operator, you can only use Approve
    Approve {
        /// address being granted the permission
        spender: HumanAddr,
        /// id of the token that the spender can transfer
        token_id: String,
        /// optional expiration for this approval
        expires: Option<Expiration>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// revokes the spender's permission to transfer the specified token.  If you are the owner
    /// of the token, you can use [`SetWhitelistedApproval`](HandleMsg::SetWhitelistedApproval) to accomplish the same thing.  If you
    /// are an operator, you can only use Revoke, but you can not revoke the transfer approval
    /// of another operator
    Revoke {
        /// address whose permission is revoked
        spender: HumanAddr,
        /// id of the token that the spender can no longer transfer
        token_id: String,
        /// optional message length padding
        padding: Option<String>,
    },
    /// provided for cw721 compliance, but can be done with [`SetWhitelistedApproval`](HandleMsg::SetWhitelistedApproval)...
    /// gives the operator permission to transfer all of the message sender's tokens
    ApproveAll {
        /// address being granted permission to transfer
        operator: HumanAddr,
        /// optional expiration for this approval
        expires: Option<Expiration>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// provided for cw721 compliance, but can be done with [`SetWhitelistedApproval`](HandleMsg::SetWhitelistedApproval)...
    /// revokes the operator's permission to transfer any of the message sender's tokens
    RevokeAll {
        /// address whose permissions are revoked
        operator: HumanAddr,
        /// optional message length padding
        padding: Option<String>,
    },
    /// add/remove approval(s) for a specific address on the token(s) you own.  Any permissions
    /// that are omitted will keep the current permission setting for that whitelist address
    SetWhitelistedApproval {
        /// address being granted/revoked permission
        address: HumanAddr,
        /// optional token id to apply approval/revocation to
        token_id: Option<String>,
        /// optional permission level for viewing the owner
        view_owner: Option<AccessLevel>,
        /// optional permission level for viewing private metadata
        view_private_metadata: Option<AccessLevel>,
        /// optional permission level for transferring
        transfer: Option<AccessLevel>,
        /// optional expiration
        expires: Option<Expiration>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// register that the message sending contract implements ReceiveNft and possibly
    /// BatchReceiveNft
    RegisterReceiveNft {
        /// receving contract's code hash
        code_hash: String,
        /// optionally true if the contract also implements BatchReceiveNft.  Defaults
        /// to false if not specified
        also_implements_batch_receive_nft: Option<bool>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// set viewing key
    SetViewingKey {
        /// desired viewing key
        key: String,
        /// optional message length padding
        padding: Option<String>,
    },

    //
    // Optional Messages
    //

    // Minting and Modifying Tokens
    //
    /// mint new token
    MintNft {
        /// optional token id. if omitted, uses current token index
        token_id: Option<String>,
        /// optional owner address. if omitted, owned by the message sender
        owner: Option<HumanAddr>,
        /// optional public metadata that can be seen by everyone
        public_metadata: Option<Metadata>,
        /// optional private metadata that can only be seen by the owner and whitelist
        private_metadata: Option<Metadata>,
        /// optional memo for the tx
        memo: Option<String>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// add addresses with minting authority
    AddMinters {
        /// list of addresses that can now mint
        minters: Vec<HumanAddr>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// revoke minting authority from addresses
    RemoveMinters {
        /// list of addresses no longer allowed to mint
        minters: Vec<HumanAddr>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// define list of addresses with minting authority
    SetMinters {
        /// list of addresses with minting authority
        minters: Vec<HumanAddr>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// set the public and/or private metadata.
    SetMetadata {
        /// id of the token whose metadata should be updated
        token_id: String,
        /// the optional new public metadata
        public_metadata: Option<Metadata>,
        /// the optional new private metadata
        private_metadata: Option<Metadata>,
        /// optional message length padding
        padding: Option<String>,
    },

    //
    // Batch Processing
    //
    /// Mint multiple tokens
    BatchMintNft {
        /// list of mint operations to perform
        mints: Vec<Mint>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// transfer many tokens
    BatchTransferNft {
        /// list of transfers to perform
        transfers: Vec<Transfer>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// send many tokens and call receiving contracts' (Batch)ReceiveNft
    BatchSendNft {
        /// list of sends to perform
        sends: Vec<Send>,
        /// optional message length padding
        padding: Option<String>,
    },

    //
    // Burning Tokens
    //
    /// burn a token
    BurnNft {
        /// token to burn
        token_id: String,
        /// optional memo for the tx
        memo: Option<String>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// burn many tokens
    BatchBurnNft {
        /// list of burns to perform
        burns: Vec<Burn>,
        /// optional message length padding
        padding: Option<String>,
    },

    //
    // Making the Owner and/or Private Metadata Public
    //
    /// add/remove approval(s) that whitelist everyone (makes public)
    SetGlobalApproval {
        /// optional token id to apply approval/revocation to
        token_id: Option<String>,
        /// optional permission level for viewing the owner
        view_owner: Option<AccessLevel>,
        /// optional permission level for viewing private metadata
        view_private_metadata: Option<AccessLevel>,
        /// optional expiration
        expires: Option<Expiration>,
        /// optional message length padding
        padding: Option<String>,
    },

    //
    // Lootboxes and Wrapped Cards
    //
    /// Reveal the private metadata of a sealed token and mark the token as having been unwrapped
    Reveal {
        /// id of the token to unwrap
        token_id: String,
        /// optional message length padding
        padding: Option<String>,
    },
}

impl HandleMsg {
    /// Returns a StdResult<CosmosMsg> used to execute a SNIP721 contract function
    ///
    /// # Arguments
    ///
    /// * `block_size` - pad the message to blocks of this size
    /// * `callback_code_hash` - String holding the code hash of the contract being called
    /// * `contract_addr` - address of the contract being called
    /// * `send_amount` - Optional Uint128 amount of native coin to send with the callback message
    ///                 NOTE: No SNIP721 messages send native coin, but the parameter is
    ///                       included in case that ever changes
    pub fn to_cosmos_msg(
        &self,
        mut block_size: usize,
        callback_code_hash: String,
        contract_addr: HumanAddr,
        send_amount: Option<Uint128>,
    ) -> StdResult<CosmosMsg> {
        // can not have block size of 0
        if block_size == 0 {
            block_size = 1;
        }
        let mut msg = to_binary(self)?;
        space_pad(&mut msg.0, block_size);
        let mut send = Vec::new();
        if let Some(amount) = send_amount {
            send.push(Coin {
                amount,
                denom: String::from("uscrt"),
            });
        }
        let execute = WasmMsg::Execute {
            msg,
            contract_addr,
            callback_code_hash,
            send,
        };
        Ok(execute.into())
    }
}

//
// Base SNIP-721 messages
//

/// Returns a StdResult<CosmosMsg> used to execute [`TransferNft`](HandleMsg::TransferNft)
///
/// # Arguments
///
/// * `recipient` - the address the token is to be transferred to
/// * `token_id` - ID String of the token to transfer
/// * `memo` - Optional String memo for the tx
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn transfer_nft_msg(
    recipient: HumanAddr,
    token_id: String,
    memo: Option<String>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::TransferNft {
        recipient,
        token_id,
        memo,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute [`SendNft`](HandleMsg::SendNft)
///
/// # Arguments
///
/// * `contract` - the address the token is to be sent to.  It does not have to be a
///                contract address, but the field is named this for CW721 compliance
/// * `token_id` - ID String of the token to send
/// * `msg` - Optional base64 encoded message to pass to the recipient contract's
///           (Batch)ReceiveNft function
/// * `memo` - Optional String memo for the tx
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
#[allow(clippy::too_many_arguments)]
pub fn send_nft_msg(
    contract: HumanAddr,
    token_id: String,
    msg: Option<Binary>,
    memo: Option<String>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::SendNft {
        contract,
        token_id,
        msg,
        memo,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute [`Approve`](HandleMsg::Approve)
///
/// # Arguments
///
/// * `spender` - the address being granted permission to transfer the token
/// * `token_id` - ID String of the token that can be transferred
/// * `expires` - Optional Expiration of this approval
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn approve_msg(
    spender: HumanAddr,
    token_id: String,
    expires: Option<Expiration>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::Approve {
        spender,
        token_id,
        expires,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute [`Revoke`](HandleMsg::Revoke)
///
/// # Arguments
///
/// * `spender` - the address whose permission to transfer the token is being revoked
/// * `token_id` - ID String of the token that can no longer be transferred
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn revoke_msg(
    spender: HumanAddr,
    token_id: String,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::Revoke {
        spender,
        token_id,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute [`ApproveAll`](HandleMsg::ApproveAll)
///
/// # Arguments
///
/// * `operator` - the address being granted permission to transfer all the message sender's tokens
/// * `expires` - Optional Expiration of this approval
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn approve_all_msg(
    operator: HumanAddr,
    expires: Option<Expiration>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::ApproveAll {
        operator,
        expires,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute [`RevokeAll`](HandleMsg::RevokeAll)
///
/// # Arguments
///
/// * `operator` - the address whose permission to transfer tokens is being revoked
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn revoke_all_msg(
    operator: HumanAddr,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::RevokeAll { operator, padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        None,
    )
}

/// Returns a StdResult<CosmosMsg> used to execute [`SetWhitelistedApproval`](HandleMsg::SetWhitelistedApproval)
///
/// # Arguments
///
/// * `address` - the address being granted/revoked permission
/// * `token_id` - Optional ID String of the token whose permissions are being set
/// * `view_owner` - Optional AccessLevel for permission to view the owner
/// * `view_private_metadata` - Optional AccessLevel for permission to view private metadata
/// * `transfer` - Optional AccessLevel for permission to transfer token(s)
/// * `expires` - Optional Expiration of any approvals in this message
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
#[allow(clippy::too_many_arguments)]
pub fn set_whitelisted_approval_msg(
    address: HumanAddr,
    token_id: Option<String>,
    view_owner: Option<AccessLevel>,
    view_private_metadata: Option<AccessLevel>,
    transfer: Option<AccessLevel>,
    expires: Option<Expiration>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::SetWhitelistedApproval {
        address,
        token_id,
        view_owner,
        view_private_metadata,
        transfer,
        expires,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute [`RegisterReceiveNft`](HandleMsg::RegisterReceiveNft)
///
/// # Arguments
///
/// * `your_contracts_code_hash` - String holding the code hash of your contract
/// * `also_implements_batch_receive_nft` - Optional bool that is true if your contract also
///               implements BatchReceiveNft.  Defaults to false if omitted
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn register_receive_nft_msg(
    your_contracts_code_hash: String,
    also_implements_batch_receive_nft: Option<bool>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::RegisterReceiveNft {
        code_hash: your_contracts_code_hash,
        also_implements_batch_receive_nft,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute [`SetViewingKey`](HandleMsg::SetViewingKey)
///
/// # Arguments
///
/// * `key` - String holding the authentication key used for later queries
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn set_viewing_key_msg(
    key: String,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::SetViewingKey { key, padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        None,
    )
}

//
// Optional Messages
//

// Minting and Modifying Tokens
//

/// Returns a StdResult<CosmosMsg> used to execute [`MintNft`](HandleMsg::MintNft)
///
/// # Arguments
///
/// * `token_id` - Optional ID String of the token to mint
/// * `owner` - Optional address that will own the newly minted token
/// * `public_metadata` - Optional Metadata that everyone can view
/// * `private_metadata` - Optional Metadata that only the owner and whitelist can view
/// * `memo` - Optional String memo for the tx
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
#[allow(clippy::too_many_arguments)]
pub fn mint_nft_msg(
    token_id: Option<String>,
    owner: Option<HumanAddr>,
    public_metadata: Option<Metadata>,
    private_metadata: Option<Metadata>,
    memo: Option<String>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::MintNft {
        token_id,
        owner,
        public_metadata,
        private_metadata,
        memo,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute [`AddMinters`](HandleMsg::AddMinters)
///
/// # Arguments
///
/// * `minters` - list of new addresses that will be allowed to mint
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn add_minters_msg(
    minters: Vec<HumanAddr>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::AddMinters { minters, padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        None,
    )
}

/// Returns a StdResult<CosmosMsg> used to execute [`RemoveMinters`](HandleMsg::RemoveMinters)
///
/// # Arguments
///
/// * `minters` - list of addresses that are no longer allowed to mint
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn remove_minters_msg(
    minters: Vec<HumanAddr>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::RemoveMinters { minters, padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        None,
    )
}

/// Returns a StdResult<CosmosMsg> used to execute [`SetMinters`](HandleMsg::SetMinters)
///
/// # Arguments
///
/// * `minters` - list of the only addresses that are allowed to mint
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn set_minters_msg(
    minters: Vec<HumanAddr>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::SetMinters { minters, padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        None,
    )
}

/// Returns a StdResult<CosmosMsg> used to execute [`SetMetadata`](HandleMsg::SetMetadata)
///
/// # Arguments
///
/// * `token_id` - ID String of the token whose public metadata should be altered
/// * `public_metadata` - optional new Metadata that everyone can view
/// * `private_metadata` - optional new Metadata that only the owner and whitelist can view
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn set_metadata_msg(
    token_id: String,
    public_metadata: Option<Metadata>,
    private_metadata: Option<Metadata>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::SetMetadata {
        token_id,
        public_metadata,
        private_metadata,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

//
// Batch Processing
//

/// Returns a StdResult<CosmosMsg> used to execute [`BatchMintNft`](HandleMsg::BatchMintNft)
///
/// # Arguments
///
/// * `mints` - list of mint operations to perform
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn batch_mint_nft_msg(
    mints: Vec<Mint>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::BatchMintNft { mints, padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        None,
    )
}

/// Returns a StdResult<CosmosMsg> used to execute [`BatchTransferNft`](HandleMsg::BatchTransferNft)
///
/// # Arguments
///
/// * `transfers` - list of Transfers to perform
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn batch_transfer_nft_msg(
    transfers: Vec<Transfer>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::BatchTransferNft { transfers, padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        None,
    )
}

/// Returns a StdResult<CosmosMsg> used to execute [`BatchSendNft`](HandleMsg::BatchSendNft)
///
/// # Arguments
///
/// * `sends` - list of Sends to perform
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn batch_send_nft_msg(
    sends: Vec<Send>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::BatchSendNft { sends, padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        None,
    )
}

//
// Burning Tokens
//

/// Returns a StdResult<CosmosMsg> used to execute [`BurnNft`](HandleMsg::BurnNft)
///
/// # Arguments
///
/// * `token_id` - ID String of the token to burn
/// * `memo` - Optional String memo for the tx
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn burn_nft_msg(
    token_id: String,
    memo: Option<String>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::BurnNft {
        token_id,
        memo,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute [`BatchBurnNft`](HandleMsg::BatchBurnNft)
///
/// # Arguments
///
/// * `burns` - list of Burns to perform
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn batch_burn_nft_msg(
    burns: Vec<Burn>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::BatchBurnNft { burns, padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        None,
    )
}

//
// Making the Owner and/or Private Metadata Public
//

/// Returns a StdResult<CosmosMsg> used to execute [`SetGlobalApproval`](HandleMsg::SetGlobalApproval)
///
/// # Arguments
///
/// * `token_id` - Optional ID String of the token whose permissions are being set
/// * `view_owner` - Optional AccessLevel for permission to view the owner
/// * `view_private_metadata` - Optional AccessLevel for permission to view private metadata
/// * `expires` - Optional Expiration of any approvals in this message
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
#[allow(clippy::too_many_arguments)]
pub fn set_global_approval_msg(
    token_id: Option<String>,
    view_owner: Option<AccessLevel>,
    view_private_metadata: Option<AccessLevel>,
    expires: Option<Expiration>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::SetGlobalApproval {
        token_id,
        view_owner,
        view_private_metadata,
        expires,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

//
// Lootboxes and Wrapped Cards
//

/// Returns a StdResult<CosmosMsg> used to execute [`Reveal`](HandleMsg::Reveal)
///
/// # Arguments
///
/// * `token_id` - ID String of the token to unwrap
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn reveal_msg(
    token_id: String,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::Reveal { token_id, padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_nft_msg() -> StdResult<()> {
        let recipient = HumanAddr("alice".to_string());
        let token_id = "NFT1".to_string();
        let memo = Some("memo".to_string());
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = transfer_nft_msg(
            recipient.clone(),
            token_id.clone(),
            memo.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::TransferNft {
            recipient,
            token_id,
            memo,
            padding,
        })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_send_nft_msg() -> StdResult<()> {
        let contract = HumanAddr("alice".to_string());
        let recipient = HumanAddr("bob".to_string());
        let token_id = "NFT1".to_string();
        let memo = Some("memo".to_string());
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());
        // just using an arbitrary msg
        let send_msg = Some(to_binary(&HandleMsg::TransferNft {
            recipient,
            token_id: token_id.clone(),
            memo: memo.clone(),
            padding: padding.clone(),
        })?);
        let test_msg = send_nft_msg(
            contract.clone(),
            token_id.clone(),
            send_msg.clone(),
            memo.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::SendNft {
            contract,
            token_id,
            msg: send_msg,
            memo,
            padding,
        })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_approve_msg() -> StdResult<()> {
        let spender = HumanAddr("alice".to_string());
        let token_id = "NFT1".to_string();
        let expires = Some(Expiration::AtHeight(1000000));
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = approve_msg(
            spender.clone(),
            token_id.clone(),
            expires.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::Approve {
            spender,
            token_id,
            expires,
            padding,
        })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_revoke_msg() -> StdResult<()> {
        let spender = HumanAddr("alice".to_string());
        let token_id = "NFT1".to_string();
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = revoke_msg(
            spender.clone(),
            token_id.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::Revoke {
            spender,
            token_id,
            padding,
        })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_approve_all_msg() -> StdResult<()> {
        let operator = HumanAddr("alice".to_string());
        let expires = Some(Expiration::AtHeight(1000000));
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = approve_all_msg(
            operator.clone(),
            expires.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::ApproveAll {
            operator,
            expires,
            padding,
        })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_revoke_all_msg() -> StdResult<()> {
        let operator = HumanAddr("alice".to_string());
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = revoke_all_msg(
            operator.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::RevokeAll { operator, padding })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_set_whitelisted_approval_msg() -> StdResult<()> {
        let address = HumanAddr("alice".to_string());
        let token_id = Some("NFT1".to_string());
        let view_owner = Some(AccessLevel::All);
        let view_private_metadata = None;
        let transfer = Some(AccessLevel::RevokeToken);
        let expires = Some(Expiration::AtTime(1000000000));
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = set_whitelisted_approval_msg(
            address.clone(),
            token_id.clone(),
            view_owner.clone(),
            view_private_metadata.clone(),
            transfer.clone(),
            expires.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::SetWhitelistedApproval {
            address,
            token_id,
            view_owner,
            view_private_metadata,
            transfer,
            expires,
            padding,
        })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_register_receive_nft_msg() -> StdResult<()> {
        let code_hash = "receiver code hash".to_string();
        let also_implements_batch_receive_nft = Some(true);
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = register_receive_nft_msg(
            code_hash.clone(),
            also_implements_batch_receive_nft.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::RegisterReceiveNft {
            code_hash,
            also_implements_batch_receive_nft,
            padding,
        })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_set_viewing_key_msg() -> StdResult<()> {
        let key = "key".to_string();
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = set_viewing_key_msg(
            key.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::SetViewingKey { key, padding })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_mint_nft_msg() -> StdResult<()> {
        let owner = Some(HumanAddr("alice".to_string()));
        let token_id = Some("NFT1".to_string());
        let public_metadata = Some(Metadata {
            name: Some("public name".to_string()),
            description: None,
            image: Some("public image".to_string()),
        });
        let private_metadata = Some(Metadata {
            name: None,
            description: Some("private description".to_string()),
            image: Some("private image".to_string()),
        });
        let memo = Some("memo".to_string());
        let padding = None;
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = mint_nft_msg(
            token_id.clone(),
            owner.clone(),
            public_metadata.clone(),
            private_metadata.clone(),
            memo.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::MintNft {
            token_id,
            owner,
            public_metadata,
            private_metadata,
            memo,
            padding,
        })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_add_minters_msg() -> StdResult<()> {
        let minters = vec![HumanAddr("alice".to_string()), HumanAddr("bob".to_string())];
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = add_minters_msg(
            minters.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::AddMinters { minters, padding })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_remove_minters_msg() -> StdResult<()> {
        let minters = vec![
            HumanAddr("alice".to_string()),
            HumanAddr("bob".to_string()),
            HumanAddr("charlie".to_string()),
        ];
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = remove_minters_msg(
            minters.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::RemoveMinters { minters, padding })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_set_minters_msg() -> StdResult<()> {
        let minters = vec![
            HumanAddr("alice".to_string()),
            HumanAddr("bob".to_string()),
            HumanAddr("charlie".to_string()),
        ];
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = set_minters_msg(
            minters.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::SetMinters { minters, padding })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_set_metadata_msg() -> StdResult<()> {
        let token_id = "NFT1".to_string();
        let public_metadata = Some(Metadata {
            name: Some("public name".to_string()),
            description: Some("public description".to_string()),
            image: None,
        });
        let private_metadata = Some(Metadata {
            name: Some("private name".to_string()),
            description: Some("private description".to_string()),
            image: Some("private image".to_string()),
        });
        let padding = None;
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = set_metadata_msg(
            token_id.clone(),
            public_metadata.clone(),
            private_metadata.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::SetMetadata {
            token_id,
            public_metadata,
            private_metadata,
            padding,
        })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_batch_mint_nft_msg() -> StdResult<()> {
        let mints = vec![
            Mint {
                token_id: None,
                owner: Some(HumanAddr("alice".to_string())),
                public_metadata: Some(Metadata {
                    name: Some("public name 1".to_string()),
                    description: None,
                    image: Some("public image 1".to_string()),
                }),
                private_metadata: None,
                memo: Some("memo 1".to_string()),
            },
            Mint {
                token_id: Some("NFT2".to_string()),
                owner: None,
                public_metadata: Some(Metadata {
                    name: None,
                    description: Some("public description 2".to_string()),
                    image: Some("public image 2".to_string()),
                }),
                private_metadata: Some(Metadata {
                    name: Some("private name 2".to_string()),
                    description: Some("private description 2".to_string()),
                    image: None,
                }),
                memo: None,
            },
            Mint {
                token_id: Some("NFT3".to_string()),
                owner: Some(HumanAddr("bob".to_string())),
                public_metadata: None,
                private_metadata: Some(Metadata {
                    name: Some("private name 3".to_string()),
                    description: Some("private description 3".to_string()),
                    image: Some("private image 3".to_string()),
                }),
                memo: Some("memo 3".to_string()),
            },
        ];
        let padding = None;
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = batch_mint_nft_msg(
            mints.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::BatchMintNft { mints, padding })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_batch_transfer_nft_msg() -> StdResult<()> {
        let transfers = vec![
            Transfer {
                recipient: HumanAddr("alice".to_string()),
                token_ids: vec!["NFT1".to_string()],
                memo: Some("memo 1".to_string()),
            },
            Transfer {
                recipient: HumanAddr("bob".to_string()),
                token_ids: vec!["NFT2".to_string(), "NFT3".to_string(), "NFT4".to_string()],
                memo: None,
            },
        ];
        let padding = None;
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = batch_transfer_nft_msg(
            transfers.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::BatchTransferNft { transfers, padding })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_batch_send_nft_msg() -> StdResult<()> {
        let sends = vec![
            Send {
                contract: HumanAddr("alice".to_string()),
                token_ids: vec!["NFT1".to_string()],
                msg: Some(to_binary(&HandleMsg::TransferNft {
                    recipient: HumanAddr("bob".to_string()),
                    token_id: "NFT1".to_string(),
                    memo: Some("send msg memo".to_string()),
                    padding: None,
                })?),
                memo: Some("memo 1".to_string()),
            },
            Send {
                contract: HumanAddr("bob".to_string()),
                token_ids: vec!["NFT2".to_string(), "NFT3".to_string(), "NFT4".to_string()],
                msg: None,
                memo: None,
            },
        ];
        let padding = None;
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = batch_send_nft_msg(
            sends.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::BatchSendNft { sends, padding })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_burn_nft_msg() -> StdResult<()> {
        let token_id = "NFT1".to_string();
        let memo = Some("memo".to_string());
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = burn_nft_msg(
            token_id.clone(),
            memo.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::BurnNft {
            token_id,
            memo,
            padding,
        })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_batch_burn_nft_msg() -> StdResult<()> {
        let burns = vec![
            Burn {
                token_ids: vec!["NFT1".to_string()],
                memo: Some("memo 1".to_string()),
            },
            Burn {
                token_ids: vec!["NFT2".to_string(), "NFT3".to_string(), "NFT4".to_string()],
                memo: None,
            },
        ];
        let padding = None;
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = batch_burn_nft_msg(
            burns.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::BatchBurnNft { burns, padding })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_set_global_approval_msg() -> StdResult<()> {
        let token_id = Some("NFT1".to_string());
        let view_owner = Some(AccessLevel::All);
        let view_private_metadata = None;
        let expires = Some(Expiration::AtTime(1000000000));
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = set_global_approval_msg(
            token_id.clone(),
            view_owner.clone(),
            view_private_metadata.clone(),
            expires.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::SetGlobalApproval {
            token_id,
            view_owner,
            view_private_metadata,
            expires,
            padding,
        })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }

    #[test]
    fn test_reveal_msg() -> StdResult<()> {
        let token_id = "NFT1".to_string();
        let padding = Some("padding".to_string());
        let callback_code_hash = "code hash".to_string();
        let contract_addr = HumanAddr("contract".to_string());

        let test_msg = reveal_msg(
            token_id.clone(),
            padding.clone(),
            256usize,
            callback_code_hash.clone(),
            contract_addr.clone(),
        )?;
        let mut msg = to_binary(&HandleMsg::Reveal { token_id, padding })?;
        let msg = space_pad(&mut msg.0, 256usize);
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            msg: Binary(msg.to_vec()),
            contract_addr,
            callback_code_hash,
            send: vec![],
        });
        assert_eq!(test_msg, expected_msg);
        Ok(())
    }
}
