use cosmwasm_std::{Addr, Api, Binary, Env, StdError, StdResult, Uint64};
use minicbor::Encoder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{cbor_to_std_error, encrypt_notification_data, get_seed, notification_id};

#[derive(Serialize, Debug, Deserialize, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct Notification<T: DirectChannel> {
    /// Recipient address of the notification
    pub notification_for: Addr,
    /// Typed notification data
    pub data: T,
}

pub trait DirectChannel {
    const CHANNEL_ID: &'static str;
    const CDDL_SCHEMA: &'static str;
    const ELEMENTS: u64;
    const PAYLOAD_SIZE: usize;

    fn channel_id(&self) -> String {
        Self::CHANNEL_ID.to_string()
    }

    fn cddl_schema(&self) -> String {
        Self::CDDL_SCHEMA.to_string()
    }

    fn to_cbor(&self, api: &dyn Api) -> StdResult<Vec<u8>> {
        // dynamically allocate output buffer
        let mut buffer = vec![0u8; Self::PAYLOAD_SIZE];

        // create CBOR encoder
        let mut encoder = Encoder::new(&mut buffer[..]);

        // encode number of elements
        encoder.array(Self::ELEMENTS).map_err(cbor_to_std_error)?;

        // encode CBOR data
        self.encode_cbor(api, &mut encoder)?;

        // return buffer (already right-padded with zero bytes)
        Ok(buffer)
    }

    /// CBOR encodes notification data into the encoder
    fn encode_cbor(&self, api: &dyn Api, encoder: &mut Encoder<&mut [u8]>) -> StdResult<()>;
}

impl<T: DirectChannel> Notification<T> {
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
        // extract and normalize tx hash
        let tx_hash = env
            .transaction
            .clone()
            .ok_or(StdError::generic_err("no tx hash found"))?
            .hash
            .to_ascii_uppercase();

        // canonicalize notification recipient address
        let notification_for_raw = api.addr_canonicalize(self.notification_for.as_str())?;

        // derive recipient's notification seed
        let seed = get_seed(&notification_for_raw, secret)?;

        // derive notification id
        let id = notification_id(&seed, self.data.channel_id().as_str(), &tx_hash)?;

        // use CBOR to encode the data
        let cbor_data = self.data.to_cbor(api)?;

        // encrypt the receiver message
        let encrypted_data = encrypt_notification_data(
            &env.block.height,
            &tx_hash,
            &seed,
            self.data.channel_id().as_str(),
            cbor_data,
            block_size,
        )?;

        // enstruct
        Ok(TxHashNotification { id, encrypted_data })
    }
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

pub trait GroupChannel<D: DirectChannel> {
    const CHANNEL_ID: &'static str;
    const BLOOM_N: usize;
    const BLOOM_M: u32;
    const BLOOM_K: u32;
    const PACKET_SIZE: usize;

    const BLOOM_M_LOG2: u32 = Self::BLOOM_M.ilog2();

    fn build_packet(&self, api: &dyn Api, data: &D) -> StdResult<Vec<u8>>;

    fn notifications(&self) -> &Vec<Notification<D>>;
}
