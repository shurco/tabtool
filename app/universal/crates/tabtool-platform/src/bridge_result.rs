//! Encoding of the Kotlin bridge's typed results.
//!
//! The bridge answers `OK`, `OK:<value>`, `CANCELLED` or `ERROR:<message>`.
//! Only Android uses this protocol; the module is compiled everywhere so its
//! parser is unit-tested on the host.

#![cfg_attr(not(target_os = "android"), allow(dead_code))]

/// The bridge's typed result.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BridgeResult {
    /// The operation succeeded, with an optional payload.
    Ok(Option<String>),
    /// The user dismissed the picker.
    Cancelled,
    /// The operation failed with a message for the log.
    Failed(String),
}

/// Parses one encoded bridge result.
pub(crate) fn parse_result(raw: &str) -> BridgeResult {
    if raw == "CANCELLED" {
        return BridgeResult::Cancelled;
    }
    if let Some(message) = raw.strip_prefix("ERROR:") {
        return BridgeResult::Failed(message.to_owned());
    }
    if raw == "OK" {
        return BridgeResult::Ok(None);
    }
    if let Some(value) = raw.strip_prefix("OK:") {
        return BridgeResult::Ok(Some(value.to_owned()));
    }
    BridgeResult::Failed(format!("malformed bridge result: {raw}"))
}

#[cfg(test)]
mod tests {
    use super::{BridgeResult, parse_result};

    #[test]
    fn plain_ok_carries_no_payload() {
        assert_eq!(parse_result("OK"), BridgeResult::Ok(None));
    }

    #[test]
    fn ok_with_a_payload_keeps_every_colon() {
        assert_eq!(
            parse_result("OK:report:final.csv"),
            BridgeResult::Ok(Some("report:final.csv".to_owned()))
        );
    }

    #[test]
    fn an_empty_payload_is_kept() {
        assert_eq!(parse_result("OK:"), BridgeResult::Ok(Some(String::new())));
    }

    #[test]
    fn a_cancel_is_typed() {
        assert_eq!(parse_result("CANCELLED"), BridgeResult::Cancelled);
    }

    #[test]
    fn an_error_keeps_its_message() {
        assert_eq!(
            parse_result("ERROR:The file type is not supported"),
            BridgeResult::Failed("The file type is not supported".to_owned())
        );
    }

    #[test]
    fn an_unknown_answer_is_a_failure() {
        assert_eq!(
            parse_result("ok"),
            BridgeResult::Failed("malformed bridge result: ok".to_owned())
        );
        assert_eq!(
            parse_result("OKAY"),
            BridgeResult::Failed("malformed bridge result: OKAY".to_owned())
        );
        assert_eq!(
            parse_result(""),
            BridgeResult::Failed("malformed bridge result: ".to_owned())
        );
    }
}
