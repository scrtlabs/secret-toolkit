use chrono::{DateTime, Utc};
use cosmwasm_std::{StdError, StdResult, Timestamp};


/// Converts an ISO 8601 date-time string in Zulu (UTC+0) format to a Timestamp.
/// 
/// String format: {YYYY}-{MM}-{DD}T{hh}:{mm}:{ss}.{uuu}Z
/// 
/// * `iso8601_str` - The ISO 8601 date-time string to convert.
/// 
/// Returns StdResult<Timestamp>
pub fn iso8601_utc0_to_timestamp(iso8601_str: &str) -> StdResult<Timestamp> {
    let Ok(datetime) = iso8601_str.parse::<DateTime<Utc>>() else {
        return Err(StdError::generic_err("ISO 8601 string could not be parsed"));
    };

    // Verify the timezone is UTC (Zulu time)
    if iso8601_str.ends_with("Z") {
        Ok(Timestamp::from_seconds(datetime.timestamp().try_into().unwrap_or_default()))
    } else {
        Err(StdError::generic_err("ISO 8601 string not in Zulu (UTC+0)"))
    }
}