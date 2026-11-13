//! The Foundation encoding IDs the Apple app persisted.

use tabtool_core::Encoding;

/// Foundation's `String.Encoding` raw values, keyed by the engine's encoding.
const FOUNDATION_ENCODING_IDS: [(Encoding, i64); 18] = [
    (Encoding::Utf8, 4),
    (Encoding::MacRoman, 30),
    (Encoding::Windows1252, 12),
    (Encoding::Gbk, 2_147_485_234),
    (Encoding::IsoLatin2, 9),
    (Encoding::Windows1250, 15),
    (Encoding::Windows1251, 11),
    (Encoding::Windows1253, 13),
    (Encoding::Windows1254, 14),
    (Encoding::EucJp, 3),
    (Encoding::ShiftJis, 8),
    (Encoding::Iso2022Jp, 21),
    (Encoding::Utf16, 10),
    (Encoding::Utf16Be, 2_415_919_360),
    (Encoding::Utf16Le, 2_483_028_224),
    (Encoding::Utf32, 2_348_810_496),
    (Encoding::Utf32Be, 2_550_137_088),
    (Encoding::Utf32Le, 2_617_245_952),
];

/// The encoding for a stored Foundation raw value, if it is one we support.
pub(crate) fn encoding_from_foundation_id(id: i64) -> Option<Encoding> {
    FOUNDATION_ENCODING_IDS
        .iter()
        .find(|(_, value)| *value == id)
        .map(|(encoding, _)| *encoding)
}

/// The Foundation raw value of an encoding.
pub(crate) fn foundation_encoding_id(encoding: Encoding) -> i64 {
    FOUNDATION_ENCODING_IDS
        .iter()
        .find(|(value, _)| *value == encoding)
        .map(|(_, id)| *id)
        .expect("every encoding has a Foundation raw value")
}

#[cfg(test)]
mod tests {
    use super::{encoding_from_foundation_id, foundation_encoding_id};
    use tabtool_core::Encoding;

    #[test]
    fn every_encoding_round_trips_through_its_foundation_id() {
        for encoding in Encoding::ALL {
            let id = foundation_encoding_id(encoding);
            assert_eq!(encoding_from_foundation_id(id), Some(encoding));
        }
    }
}
