//! Pure CSV engine shared by every TabTool app.

#![forbid(unsafe_code)]

mod clipboard;
mod column_name;
mod configuration;
mod decimal;
mod detector;
mod encoding;
mod error;
mod export;
mod field_names;
mod filter;
mod json_writer;
mod line_ending;
mod number;
mod parser;
mod reorder;
mod row_source;
mod sort;
mod table;
mod table_extensions;
mod write_validation;
mod writer;
mod xml_writer;

pub use clipboard::{make_tsv, parse_tsv};
pub use column_name::column_name;
pub use configuration::CsvConfiguration;
pub use decimal::convert_decimal_mark;
pub use detector::CsvFormatDetector;
pub use encoding::Encoding;
pub use error::CsvError;
pub use export::{ExportFormat, ExportInput, export_data, export_preflight_issue};
pub use filter::{FilterCriterion, FilterOperator, RowFilter};
pub use line_ending::{LineEnding, Terminator};
pub use number::{is_numeric, localized};
pub use parser::CsvParser;
pub use reorder::{is_permutation, shifted};
pub use row_source::RowSource;
pub use sort::{SortDirection, row_order_sorted_by};
pub use table::{ColumnId, CriterionId, CsvColumn, CsvTable, RowSet, new_column_id};
pub use table_extensions::{TABLE_EXTENSIONS, has_table_extension, is_table_extension};
pub use write_validation::{CsvWriteIssue, is_ambiguous, issue};
pub use writer::CsvWriter;

/// Locale used by sorting and row filters (ICU4X).
pub use icu_locale_core::Locale;

/// Release version, `universal.name` in `app/version.json`.
pub const VERSION: &str = env!("TABTOOL_VERSION");
