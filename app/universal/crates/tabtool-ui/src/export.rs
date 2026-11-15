//! Export planning and the export options panel.
//!
//! The decisions mirror the Apple `ExportPlan`: JSON and XML keep their own
//! extension; for CSV a tab separator writes TSV and every other separator
//! writes CSV. The suggested name is the document stem plus `-export`, and the
//! preflight runs on the converted table so a decimal mark change cannot hide
//! or create ambiguity.
//!
//! The panel edits a copy of the document configuration and the export format.
//! Nothing here mutates a document or writes a file.

pub(crate) mod controls;
mod panel;
mod plan;

pub(crate) use panel::caption_key;
pub use panel::{ExportPanelAction, export_panel};
pub use plan::{base_name, data, file_extension, filename, preflight};
