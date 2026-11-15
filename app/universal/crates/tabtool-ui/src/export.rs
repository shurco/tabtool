//! Export planning and the export options panel.
//!
//! The decisions mirror the Apple `ExportPlan`: a tab separator writes TSV,
//! every other separator writes CSV, the suggested name is the document stem
//! plus `-export`, and the preflight runs on the converted table so a decimal
//! mark change cannot hide or create ambiguity.
//!
//! The panel edits a copy of the document configuration. Nothing here mutates a
//! document or writes a file.

pub(crate) mod controls;
mod panel;
mod plan;

pub use panel::{ExportPanelAction, export_panel};
pub use plan::{base_name, data, file_extension, filename, preflight};
