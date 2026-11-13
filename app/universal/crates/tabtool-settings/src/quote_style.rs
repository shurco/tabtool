//! The quote style of a new document.

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
