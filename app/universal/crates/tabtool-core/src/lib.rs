//! Pure CSV engine shared by every TabTool app.

#![forbid(unsafe_code)]

mod column_name;
mod configuration;
mod detector;
mod encoding;
mod error;
mod filter;
mod number;
mod parser;
mod reorder;
mod row_source;
mod sort;
mod table;
mod write_validation;
mod writer;

pub use column_name::{column_name, column_name_checked};
pub use configuration::CsvConfiguration;
pub use detector::CsvFormatDetector;
pub use encoding::Encoding;
pub use error::CsvError;
pub use filter::{FilterCriterion, FilterOperator, RowFilter};
pub use number::{is_numeric, localized};
pub use parser::CsvParser;
pub use reorder::{is_permutation, shifted};
pub use row_source::RowSource;
pub use sort::{SortDirection, row_order_sorted_by};
pub use table::{ColumnId, CriterionId, CsvColumn, CsvTable, RowSet};
pub use write_validation::{CsvWriteIssue, is_ambiguous, issue};
pub use writer::CsvWriter;

/// Locale used by sorting and row filters (ICU4X).
pub use icu_locale_core::Locale;

/// Crate version, taken from its package manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
