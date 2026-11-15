//! The quote style of a new document.

use tabtool_core::CsvConfiguration;

/// The quote style of a new document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuoteStyle {
    /// Quotes inside a quoted field are doubled.
    Doubled,
    /// Quotes inside a quoted field are escaped with a backslash.
    Backslash,
    /// The document has no quote character.
    None,
}

impl QuoteStyle {
    /// Every offered quote style.
    pub const ALL: [Self; 3] = [Self::Doubled, Self::Backslash, Self::None];

    /// The style a configuration currently represents.
    pub fn of(configuration: &CsvConfiguration) -> Self {
        if configuration.quote_character.is_empty() {
            Self::None
        } else if configuration.escape_character == "\\" {
            Self::Backslash
        } else {
            Self::Doubled
        }
    }

    /// The index of this style in [`Self::ALL`].
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|style| *style == self)
            .expect("ALL lists every quote style")
    }

    /// Applies this style to `configuration`.
    pub fn apply(self, configuration: &mut CsvConfiguration) {
        configuration.quote_character = self.quote_character().to_owned();
        configuration.escape_character = self.escape_character().to_owned();
    }

    /// The value persisted in the settings file.
    pub fn persisted(self) -> &'static str {
        match self {
            Self::Doubled => "doubled",
            Self::Backslash => "backslash",
            Self::None => "none",
        }
    }

    /// The style stored as `value`, if it is a known one.
    pub fn from_persisted(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|style| style.persisted() == value)
    }

    /// The quote character, or an empty string when quoting is off.
    pub fn quote_character(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Doubled | Self::Backslash => "\"",
        }
    }

    /// The character that escapes a quote inside a quoted field.
    pub fn escape_character(self) -> &'static str {
        match self {
            Self::Doubled => "\"",
            Self::Backslash => "\\",
            Self::None => "",
        }
    }
}
