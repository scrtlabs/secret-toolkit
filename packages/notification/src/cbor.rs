use cosmwasm_std::{CanonicalAddr, StdError, StdResult};
use minicbor::{data as cbor_data, encode as cbor_encode, Encoder};

/// Length of encoding an arry header that holds less than 24 items
pub const CBL_ARRAY_SHORT: usize = 1;

/// Length of encoding an arry header that holds between 24 and 255 items
pub const CBL_ARRAY_MEDIUM: usize = 2;

/// Length of encoding an arry header that holds more than 255 items
pub const CBL_ARRAY_LARGE: usize = 3;

/// Length of encoding a u8 value that is less than 24
pub const CBL_U8_LESS_THAN_24: usize = 1;

/// Length of encoding a u8 value that is greater than or equal to 24
pub const CBL_U8: usize = 1 + 1;

/// Length of encoding a u16 value
pub const CBL_U16: usize = 1 + 2;

/// Length of encoding a u32 value
pub const CBL_U32: usize = 1 + 4;

/// Length of encoding a u53 value (the maximum safe integer size for javascript)
pub const CBL_U53: usize = 1 + 8;

/// Length of encoding a u64 value (with the bignum tag attached)
pub const CBL_BIGNUM_U64: usize = 1 + 1 + 8;

// Length of encoding a timestamp
pub const CBL_TIMESTAMP: usize = 1 + 1 + 8;

// Length of encoding a 20-byte canonical address
pub const CBL_ADDRESS: usize = 1 + 20;

/// Wraps the CBOR error to CosmWasm StdError
pub fn cbor_to_std_error<T>(_e: cbor_encode::Error<T>) -> StdError {
    StdError::generic_err("CBOR encoding error")
}

/// Extends the minicbor encoder with wrapper functions that handle CBOR errors
pub trait EncoderExt {
    fn ext_tag(&mut self, tag: cbor_data::IanaTag) -> StdResult<&mut Self>;

    fn ext_u8(&mut self, value: u8) -> StdResult<&mut Self>;
    fn ext_u32(&mut self, value: u32) -> StdResult<&mut Self>;
    fn ext_u64_from_u128(&mut self, value: u128) -> StdResult<&mut Self>;
    fn ext_address(&mut self, value: CanonicalAddr) -> StdResult<&mut Self>;
    fn ext_bytes(&mut self, value: &[u8]) -> StdResult<&mut Self>;
    fn ext_timestamp(&mut self, value: u64) -> StdResult<&mut Self>;
}

impl<T: cbor_encode::Write> EncoderExt for Encoder<T> {
    #[inline]
    fn ext_tag(&mut self, tag: cbor_data::IanaTag) -> StdResult<&mut Self> {
        self.tag(cbor_data::Tag::from(tag))
            .map_err(cbor_to_std_error)
    }

    #[inline]
    fn ext_u8(&mut self, value: u8) -> StdResult<&mut Self> {
        self.u8(value).map_err(cbor_to_std_error)
    }

    #[inline]
    fn ext_u32(&mut self, value: u32) -> StdResult<&mut Self> {
        self.u32(value).map_err(cbor_to_std_error)
    }

    #[inline]
    fn ext_u64_from_u128(&mut self, value: u128) -> StdResult<&mut Self> {
        self.ext_tag(cbor_data::IanaTag::PosBignum)?
            .ext_bytes(&value.to_be_bytes()[8..])
    }

    #[inline]
    fn ext_address(&mut self, value: CanonicalAddr) -> StdResult<&mut Self> {
        self.ext_bytes(value.as_slice())
    }

    #[inline]
    fn ext_bytes(&mut self, value: &[u8]) -> StdResult<&mut Self> {
        self.bytes(value).map_err(cbor_to_std_error)
    }

    #[inline]
    fn ext_timestamp(&mut self, value: u64) -> StdResult<&mut Self> {
        self.ext_tag(cbor_data::IanaTag::Timestamp)?
            .u64(value)
            .map_err(cbor_to_std_error)
    }
}
