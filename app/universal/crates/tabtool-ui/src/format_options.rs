//! The separator, decimal and quote choices shared by the format bar and the
//! export options.

use tabtool_core::CsvConfiguration;

/// The separators the bars offer, with their catalog keys.
pub(crate) const SEPARATORS: [(&str, &str); 4] = [
    (",", "Comma (,)"),
    (";", "Semicolon (;)"),
    ("\t", "Tab (⇥)"),
    ("|", "Pipe (|)"),
];

/// The decimal marks the bars offer, with their catalog keys.
pub(crate) const DECIMALS: [(&str, &str); 2] = [(".", "Dot (.)"), (",", "Comma (,)")];

/// The three quote styles the bars offer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QuoteStyle {
    Doubled,
    Backslash,
    None,
}

impl QuoteStyle {
    /// The style a configuration currently represents.
    pub(crate) fn of(configuration: &CsvConfiguration) -> Self {
        if configuration.quote_character.is_empty() {
            Self::None
        } else if configuration.escape_character == "\\" {
            Self::Backslash
        } else {
            Self::Doubled
        }
    }

    /// The index of this style in the picker.
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Doubled => 0,
            Self::Backslash => 1,
            Self::None => 2,
        }
    }

    /// The style at a picker index.
    pub(crate) fn at(index: usize) -> Self {
        match index {
            1 => Self::Backslash,
            2 => Self::None,
            _ => Self::Doubled,
        }
    }

    /// Applies this style to `configuration`.
    pub(crate) fn apply(self, configuration: &mut CsvConfiguration) {
        match self {
            Self::Doubled => {
                configuration.quote_character = "\"".to_owned();
                configuration.escape_character = "\"".to_owned();
            }
            Self::Backslash => {
                configuration.quote_character = "\"".to_owned();
                configuration.escape_character = "\\".to_owned();
            }
            Self::None => {
                configuration.quote_character.clear();
                configuration.escape_character.clear();
            }
        }
    }
}
