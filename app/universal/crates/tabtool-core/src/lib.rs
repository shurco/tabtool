//! Pure CSV engine shared by every TabTool app.

#![forbid(unsafe_code)]

mod configuration;
mod encoding;
mod error;
mod number;

pub use configuration::CsvConfiguration;
pub use encoding::Encoding;
pub use error::CsvError;
pub use number::{is_numeric, localized};

/// Crate version, taken from its package manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
