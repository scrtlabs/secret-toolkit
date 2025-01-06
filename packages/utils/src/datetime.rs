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
        Ok(Timestamp::from_seconds(
            datetime.timestamp().try_into().unwrap_or_default(),
        ))
    } else {
        Err(StdError::generic_err("ISO 8601 string not in Zulu (UTC+0)"))
    }
}

/// Converts an ISO 8601 date-time string in Zulu (UTC+0) format to seconds.
///
/// String format: {YYYY}-{MM}-{DD}T{hh}:{mm}:{ss}.{uuu}Z
///
/// * `iso8601_str` - The ISO 8601 date-time string to convert.
///
/// Returns StdResult<u64>
pub fn iso8601_utc0_to_seconds(iso8601_str: &str) -> StdResult<u64> {
    let Ok(datetime) = iso8601_str.parse::<DateTime<Utc>>() else {
        return Err(StdError::generic_err("ISO 8601 string could not be parsed"));
    };

    // Verify the timezone is UTC (Zulu time)
    if iso8601_str.ends_with("Z") {
        let seconds = datetime.timestamp();
        if seconds < 0 {
            return Err(StdError::generic_err(
                "Date time before January 1, 1970 0:00:00 UTC not supported",
            ));
        }
        Ok(seconds as u64)
    } else {
        Err(StdError::generic_err("ISO 8601 string not in Zulu (UTC+0)"))
    }
}

#[cfg(test)]
mod tests {
    use super::iso8601_utc0_to_timestamp;

    #[test]
    fn test_iso8601_utc0_to_timestamp() {
        let dt_string = "2024-12-17T16:59:00.000Z";
        let timestamp = iso8601_utc0_to_timestamp(dt_string).unwrap();
        println!("{:?}", timestamp);
        assert_eq!(timestamp.nanos(), 1734454740000000000);

        let dt_string = "2024-12-17T16:59:00.000";
        let timestamp = iso8601_utc0_to_timestamp(dt_string);
        assert!(
            timestamp.is_err(),
            "datetime string without Z Ok: {:?}",
            timestamp
        );

        let dt_string = "not a datetime";
        let timestamp = iso8601_utc0_to_timestamp(dt_string);
        assert!(
            timestamp.is_err(),
            "invalid datetime string Ok: {:?}",
            timestamp
        );
    }
}
