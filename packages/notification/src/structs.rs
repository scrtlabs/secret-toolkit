use cosmwasm_std::{Addr, Api, Binary, Env, StdError, StdResult, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{encrypt_notification_data, get_seed, notification_id};

#[derive(Serialize, Debug, Deserialize, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct Notification<T: NotificationData> {
    // target for the notification
    pub notification_for: Addr,
    // data
    pub data: T,
}

impl<T: NotificationData> Notification<T> {
    pub fn new(notification_for: Addr, data: T) -> Self {
        Notification {
            notification_for,
            data,
        }
    }

    pub fn to_txhash_notification(
        &self,
        api: &dyn Api,
        env: &Env,
        secret: &[u8],
        block_size: Option<usize>,
    ) -> StdResult<TxHashNotification> {
        let tx_hash = env.transaction.clone().ok_or(StdError::generic_err("no tx hash found"))?.hash;
        let notification_for_raw = api.addr_canonicalize(self.notification_for.as_str())?;
        let seed = get_seed(&notification_for_raw, secret)?;

        // get notification id
        let id = notification_id(&seed, self.data.channel_id(), &tx_hash)?;

        // use CBOR to encode the data
        let cbor_data = self.data.to_cbor(api)?;

        // encrypt the receiver message
        let encrypted_data = encrypt_notification_data(
            &env.block.height,
            &tx_hash,
            &seed,
            self.data.channel_id(),
            cbor_data,
            block_size,
        )?;

        Ok(TxHashNotification {
            id,
            encrypted_data,
        })
    }
}

pub trait NotificationData {
    fn to_cbor(&self, api: &dyn Api) -> StdResult<Vec<u8>>;
    fn channel_id(&self) -> &str;
    fn id_to_string() -> String;
}

#[derive(Serialize, Debug, Deserialize, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct TxHashNotification {
    pub id: Binary,
    pub encrypted_data: Binary,
}

impl TxHashNotification {
    pub fn id_plaintext(&self) -> String {
        format!("snip52:{}", self.id.to_base64())
    }

    pub fn data_plaintext(&self) -> String {
        self.encrypted_data.to_base64()
    }
}

// types for channel info response

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ChannelInfoData {
    /// same as query input
    pub channel: String,
    /// "counter", "txhash", "bloom"
    pub mode: String,

    /// txhash / bloom fields only
    /// if txhash argument was given, this will be its computed Notification ID
    pub answer_id: Option<Binary>,

    /// bloom fields only
    /// bloom filter parameters
    pub parameters: Option<BloomParameters>,
    /// bloom filter data
    pub data: Option<Descriptor>,

    /// counter fields only
    /// current counter value
    pub counter: Option<Uint64>,
    /// the next Notification ID
    pub next_id: Option<Binary>,

    /// counter / txhash field only
    /// optional CDDL schema definition string for the CBOR-encoded notification data
    pub cddl: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct BloomParameters {
    pub m: u32,
    pub k: u32,
    pub h: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Descriptor {
    pub r#type: String,
    pub version: String,
    pub packet_size: u32,
    pub data: StructDescriptor,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct StructDescriptor {
    pub r#type: String,
    pub label: String,
    pub members: Vec<FlatDescriptor>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct FlatDescriptor {
    pub r#type: String,
    pub label: String,
    pub description: Option<String>,
}