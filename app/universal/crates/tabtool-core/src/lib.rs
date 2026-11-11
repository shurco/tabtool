//! Pure CSV engine shared by every TabTool app.

#![forbid(unsafe_code)]

mod column_name;
mod configuration;
mod encoding;
mod error;
mod number;
mod parser;
mod reorder;
mod row_source;
mod table;

pub use column_name::{column_name, column_name_checked};
pub use configuration::CsvConfiguration;
pub use encoding::Encoding;
pub use error::CsvError;
pub use number::{is_numeric, localized};
pub use parser::CsvParser;
pub use reorder::{is_permutation, shifted};
pub use row_source::RowSource;
pub use table::{ColumnId, CriterionId, CsvColumn, CsvTable, RowSet};

/// Crate version, taken from its package manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
