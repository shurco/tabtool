//! The row-filter controls: an inline bar between the toolbar and the grid.
//!
//! The bar edits the document's draft criteria. Rows only change when Apply
//! All or a criterion's Apply is pressed; hiding the bar keeps both the draft
//! and the applied filter.

pub mod bar;

pub use bar::{FilterAction, operator_title, show};
