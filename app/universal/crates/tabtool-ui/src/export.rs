//! Export planning and the export options panel.
//!
//! The decisions mirror the Apple `ExportPlan`: a tab separator writes TSV,
//! every other separator writes CSV, the suggested name is the document stem
//! plus `-export`, and the preflight runs on the converted table so a decimal
//! mark change cannot hide or create ambiguity.
//!
//! The panel edits a copy of the document configuration. Nothing here mutates a
//! document or writes a file.

mod panel;
mod plan;

use tabtool_core::CsvConfiguration;

pub use panel::{ExportPanelAction, export_panel};
pub use plan::{ExportDecision, ExportPlan};

/// The format the export panel edits.
///
/// It starts from the document configuration and stays local to the export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportOptions {
    /// The candidate export format.
    pub configuration: CsvConfiguration,
}

impl ExportOptions {
    /// Seeds the options from a document configuration.
    pub fn new(configuration: CsvConfiguration) -> Self {
        Self { configuration }
    }

    /// The complete suggested file name for `base_name`.
    pub fn suggested_name(&self, base_name: &str) -> String {
        ExportPlan::filename(base_name, &self.configuration)
    }

    /// The extension this export writes.
    pub fn file_extension(&self) -> &'static str {
        ExportPlan::file_extension(&self.configuration)
    }
}
