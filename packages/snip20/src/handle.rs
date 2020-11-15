use serde::Serialize;

use cosmwasm_std::{to_binary, Binary, Coin, CosmosMsg, HumanAddr, StdResult, Uint128, WasmMsg};

use secret_toolkit_utils::space_pad;

/// SNIP20 token handle messages
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg<'a> {
    // Native coin interactions
    Redeem {
        amount: Uint128,
        // TO DO: remove skip_serializing once denom is added to sSCRT stored on mainnet
        #[serde(skip_serializing_if = "Option::is_none")]
        denom: Option<String>,
        padding: Option<String>,
    },
    Deposit {
        padding: Option<String>,
    },

    // Basic SNIP20 functions
    Transfer {
        recipient: &'a HumanAddr,
        amount: Uint128,
        padding: Option<String>,
    },
    Send {
        recipient: &'a HumanAddr,
        amount: Uint128,
        msg: Option<Binary>,
        padding: Option<String>,
    },
    Burn {
        amount: Uint128,
        padding: Option<String>,
    },
    SetViewingKey {
        key: &'a str,
        padding: Option<String>,
    },

    // Allowance functions
    IncreaseAllowance {
        spender: &'a HumanAddr,
        amount: Uint128,
        expiration: Option<u64>,
        padding: Option<String>,
    },
    DecreaseAllowance {
        spender: &'a HumanAddr,
        amount: Uint128,
        expiration: Option<u64>,
        padding: Option<String>,
    },
    TransferFrom {
        owner: &'a HumanAddr,
        recipient: &'a HumanAddr,
        amount: Uint128,
        padding: Option<String>,
    },
    SendFrom {
        owner: &'a HumanAddr,
        recipient: &'a HumanAddr,
        amount: Uint128,
        msg: Option<Binary>,
        padding: Option<String>,
    },
    BurnFrom {
        owner: &'a HumanAddr,
        amount: Uint128,
        padding: Option<String>,
    },

    // Mint
    Mint {
        recipient: &'a HumanAddr,
        amount: Uint128,
        padding: Option<String>,
    },
    AddMinters {
        minters: &'a [HumanAddr],
        padding: Option<String>,
    },
    RemoveMinters {
        minters: &'a [HumanAddr],
        padding: Option<String>,
    },
    SetMinters {
        minters: &'a [HumanAddr],
        padding: Option<String>,
    },

    // Set up Send/Receive functionality
    RegisterReceive {
        code_hash: &'a str,
        padding: Option<String>,
    },
}

impl<'a> HandleMsg<'a> {
    /// Returns a StdResult<CosmosMsg> used to execute a SNIP20 contract function
    ///
    /// # Arguments
    ///
    /// * `block_size` - pad the message to blocks of this size
    /// * `callback_code_hash` - String holding the code hash of the contract being called
    /// * `contract_addr` - address of the contract being called
    /// * `send_amount` - Optional Uint128 amount of native coin to send with the callback message
    ///                 NOTE: Only a Deposit message should have an amount sent with it
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

/// Returns a StdResult<CosmosMsg> used to execute Redeem
///
/// # Arguments
///
/// * `amount` - Uint128 amount of token to redeem for SCRT
/// * `denom` - Optional String to hold the denomination of tokens to redeem
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn redeem_msg(
    amount: Uint128,
    denom: Option<String>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::Redeem {
        amount,
        denom,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute Deposit
///
/// # Arguments
///
/// * `amount` - Uint128 amount of uSCRT to convert to the SNIP20 token
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn deposit_msg(
    amount: Uint128,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::Deposit { padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        Some(amount),
    )
}

/// Returns a StdResult<CosmosMsg> used to execute Transfer
///
/// # Arguments
///
/// * `recipient` - a reference to the address the tokens are to be sent to
/// * `amount` - Uint128 amount of tokens to send
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn transfer_msg(
    recipient: &HumanAddr,
    amount: Uint128,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::Transfer {
        recipient,
        amount,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute Send
///
/// # Arguments
///
/// * `recipient` - a reference to the address tokens are to be sent to
/// * `amount` - Uint128 amount of tokens to send
/// * `msg` - Optional base64 encoded string to pass to the recipient contract's
///           Receive function
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn send_msg(
    recipient: &HumanAddr,
    amount: Uint128,
    msg: Option<Binary>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::Send {
        recipient,
        amount,
        msg,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute Burn
///
/// # Arguments
///
/// * `amount` - Uint128 amount of tokens to burn
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn burn_msg(
    amount: Uint128,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::Burn { amount, padding }.to_cosmos_msg(
        block_size,
        callback_code_hash,
        contract_addr,
        None,
    )
}

/// Returns a StdResult<CosmosMsg> used to execute RegisterReceive
///
/// # Arguments
///
/// * `your_contracts_code_hash` - string slice holding the code hash of your contract
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn register_receive_msg(
    your_contracts_code_hash: &str,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::RegisterReceive {
        code_hash: your_contracts_code_hash,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute SetViewingKey
///
/// # Arguments
///
/// * `key` - string slice holding the authentication key used for later queries
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn set_viewing_key_msg(
    key: &str,
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

/// Returns a StdResult<CosmosMsg> used to execute IncreaseAllowance
///
/// # Arguments
///
/// * `spender` - a reference to the address of the allowed spender
/// * `amount` - Uint128 additional amount the spender is allowed to send/burn
/// * `expiration` - Optional u64 denoting the epoch time in seconds that the allowance will expire
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn increase_allowance_msg(
    spender: &HumanAddr,
    amount: Uint128,
    expiration: Option<u64>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::IncreaseAllowance {
        spender,
        amount,
        expiration,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute DecreaseAllowance
///
/// # Arguments
///
/// * `spender` - a reference to the address of the allowed spender
/// * `amount` - Uint128 amount the spender is no longer allowed to send/burn
/// * `expiration` - Optional u64 denoting the epoch time in seconds that the allowance will expire
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn decrease_allowance_msg(
    spender: &HumanAddr,
    amount: Uint128,
    expiration: Option<u64>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::DecreaseAllowance {
        spender,
        amount,
        expiration,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute TransferFrom
///
/// # Arguments
///
/// * `owner` - a reference to the address of the owner of the tokens to be sent
/// * `recipient` - a reference to the address the tokens are to be sent to
/// * `amount` - Uint128 amount of tokens to send
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn transfer_from_msg(
    owner: &HumanAddr,
    recipient: &HumanAddr,
    amount: Uint128,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::TransferFrom {
        owner,
        recipient,
        amount,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute SendFrom
///
/// # Arguments
///
/// * `owner` - a reference to the address of the owner of the tokens to be sent
/// * `recipient` - a reference to the address the tokens are to be sent to
/// * `amount` - Uint128 amount of tokens to send
/// * `msg` - Optional base64 encoded string to pass to the recipient contract's
///           Receive function
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
#[allow(clippy::too_many_arguments)]
pub fn send_from_msg(
    owner: &HumanAddr,
    recipient: &HumanAddr,
    amount: Uint128,
    msg: Option<Binary>,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::SendFrom {
        owner,
        recipient,
        amount,
        msg,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute BurnFrom
///
/// # Arguments
///
/// * `owner` - a reference to the address of the owner of the tokens to be burnt
/// * `amount` - Uint128 amount of tokens to burn
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn burn_from_msg(
    owner: &HumanAddr,
    amount: Uint128,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::BurnFrom {
        owner,
        amount,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute Mint
///
/// # Arguments
///
/// * `recipient` - a reference to the address that will receive the newly minted tokens
/// * `amount` - Uint128 amount of tokens to mint
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn mint_msg(
    recipient: &HumanAddr,
    amount: Uint128,
    padding: Option<String>,
    block_size: usize,
    callback_code_hash: String,
    contract_addr: HumanAddr,
) -> StdResult<CosmosMsg> {
    HandleMsg::Mint {
        recipient,
        amount,
        padding,
    }
    .to_cosmos_msg(block_size, callback_code_hash, contract_addr, None)
}

/// Returns a StdResult<CosmosMsg> used to execute AddMinters
///
/// # Arguments
///
/// * `minters` - slice of a list of new addresses that will be allowed to mint
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn add_minters_msg(
    minters: &[HumanAddr],
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

/// Returns a StdResult<CosmosMsg> used to execute RemoveMinters
///
/// # Arguments
///
/// * `minters` - slice of a list of addresses that are no longer allowed to mint
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn remove_minters_msg(
    minters: &[HumanAddr],
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

/// Returns a StdResult<CosmosMsg> used to execute SetMinters
///
/// # Arguments
///
/// * `minters` - slice of a list of the only addresses that are allowed to mint
/// * `padding` - Optional String used as padding if you don't want to use block padding
/// * `block_size` - pad the message to blocks of this size
/// * `callback_code_hash` - String holding the code hash of the contract being called
/// * `contract_addr` - address of the contract being called
pub fn set_minters_msg(
    minters: &[HumanAddr],
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
