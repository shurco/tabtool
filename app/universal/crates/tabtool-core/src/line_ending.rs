//! The row terminator style of a CSV file.

/// A row terminator.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Terminator {
    /// Line feed.
    #[default]
    Lf,
    /// Carriage return followed by line feed.
    Crlf,
    /// Carriage return.
    Cr,
}

impl Terminator {
    /// The terminator text.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Crlf => "\r\n",
            Self::Cr => "\r",
        }
    }
}

/// The row terminator style of a CSV file and whether the file ends with one.
///
/// The default is a line feed without a trailing terminator.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LineEnding {
    /// The terminator that separates rows.
    pub terminator: Terminator,
    /// Whether the output ends with a terminator.
    pub ends_with_terminator: bool,
}
