#![allow(clippy::field_reassign_with_default)] // This is triggered in `#[derive(JsonSchema)]`

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::pubkey_to_account;
use cosmwasm_std::{Binary, CanonicalAddr, HumanAddr, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Permit<Permission: Permissions = TokenPermissions> {
    #[serde(bound = "")]
    pub params: PermitParams<Permission>,
    pub signature: PermitSignature,
}

impl<Permission: Permissions> Permit<Permission> {
    pub fn check_token(&self, token: &HumanAddr) -> bool {
        self.params.allowed_tokens.contains(token)
    }

    pub fn check_permission(&self, permission: &Permission) -> bool {
        self.params.permissions.contains(permission)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PermitParams<Permission: Permissions = TokenPermissions> {
    pub allowed_tokens: Vec<HumanAddr>,
    pub permit_name: String,
    pub chain_id: String,
    #[serde(bound = "")]
    pub permissions: Vec<Permission>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PermitSignature {
    pub pub_key: PubKey,
    pub signature: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PubKey {
    /// ignored, but must be "tendermint/PubKeySecp256k1" otherwise the verification will fail
    pub r#type: String,
    /// Secp256k1 PubKey
    pub value: Binary,
}

impl PubKey {
    pub fn canonical_address(&self) -> CanonicalAddr {
        pubkey_to_account(&self.value)
    }
}

// Note: The order of fields in this struct is important for the permit signature verification!
#[remain::sorted]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SignedPermit<Permission: Permissions = TokenPermissions> {
    /// ignored
    pub account_number: Uint128,
    /// ignored, no Env in query
    pub chain_id: String,
    /// ignored
    pub fee: Fee,
    /// ignored
    pub memo: String,
    /// the signed message
    #[serde(bound = "")]
    pub msgs: Vec<PermitMsg<Permission>>,
    /// ignored
    pub sequence: Uint128,
}

impl<Permission: Permissions> SignedPermit<Permission> {
    pub fn from_params(params: &PermitParams<Permission>) -> Self {
        Self {
            account_number: Uint128::zero(),
            chain_id: params.chain_id.clone(),
            fee: Fee::new(),
            memo: String::new(),
            msgs: vec![PermitMsg::from_content(PermitContent::from_params(params))],
            sequence: Uint128::zero(),
        }
    }
}

// Note: The order of fields in this struct is important for the permit signature verification!
#[remain::sorted]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Fee {
    pub amount: Vec<Coin>,
    pub gas: Uint128,
}

impl Fee {
    pub fn new() -> Self {
        Self {
            amount: vec![Coin::new()],
            gas: Uint128(1),
        }
    }
}

impl Default for Fee {
    fn default() -> Self {
        Self::new()
    }
}

// Note: The order of fields in this struct is important for the permit signature verification!
#[remain::sorted]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Coin {
    pub amount: Uint128,
    pub denom: String,
}

impl Coin {
    pub fn new() -> Self {
        Self {
            amount: Uint128::zero(),
            denom: "uscrt".to_string(),
        }
    }
}

impl Default for Coin {
    fn default() -> Self {
        Self::new()
    }
}

// Note: The order of fields in this struct is important for the permit signature verification!
#[remain::sorted]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PermitMsg<Permission: Permissions = TokenPermissions> {
    pub r#type: String,
    #[serde(bound = "")]
    pub value: PermitContent<Permission>,
}

impl<Permission: Permissions> PermitMsg<Permission> {
    pub fn from_content(content: PermitContent<Permission>) -> Self {
        Self {
            r#type: "query_permit".to_string(),
            value: content,
        }
    }
}

// Note: The order of fields in this struct is important for the permit signature verification!
#[remain::sorted]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PermitContent<Permission: Permissions = TokenPermissions> {
    pub allowed_tokens: Vec<HumanAddr>,
    #[serde(bound = "")]
    pub permissions: Vec<Permission>,
    pub permit_name: String,
}

impl<Permission: Permissions> PermitContent<Permission> {
    pub fn from_params(params: &PermitParams<Permission>) -> Self {
        Self {
            allowed_tokens: params.allowed_tokens.clone(),
            permit_name: params.permit_name.clone(),
            permissions: params.permissions.clone(),
        }
    }
}

/// This trait is an alias for all the other traits it inherits from.
/// It does this by providing a blanket implementation for all types that
/// implement the same set of traits
pub trait Permissions:
    Clone + PartialEq + Serialize + for<'d> Deserialize<'d> + JsonSchema
{
}

impl<T> Permissions for T where
    T: Clone + PartialEq + Serialize + for<'d> Deserialize<'d> + JsonSchema
{
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TokenPermissions {
    /// Allowance for SNIP-20 - Permission to query allowance of the owner & spender
    Allowance,
    /// Balance for SNIP-20 - Permission to query balance
    Balance,
    /// History for SNIP-20 - Permission to query transfer_history & transaction_hisotry
    History,
    /// Owner permission indicates that the bearer of this permit should be granted all
    /// the access of the creator/signer of the permit.  SNIP-721 uses this to grant
    /// viewing access to all data that the permit creator owns and is whitelisted for.
    /// For SNIP-721 use, a permit with Owner permission should NEVER be given to
    /// anyone else.  If someone wants to share private data, they should whitelist
    /// the address they want to share with via a SetWhitelistedApproval tx, and that
    /// address will view the data by creating their own permit with Owner permission
    Owner,
}
