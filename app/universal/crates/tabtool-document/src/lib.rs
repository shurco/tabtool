//! UI-free document state for TabTool.
//!
//! The crate owns one [`Document`] value: its table, phase, selection, sort and
//! save state. It has no UI, no platform code and no async runtime; the desktop
//! and Android UIs call these values from their own threads. File I/O goes
//! through the [`Storage`] trait so tests and platforms can substitute it.

#![forbid(unsafe_code)]

mod clipboard;
mod document;
mod edit;
mod format;
mod format_planner;
mod io;
mod revert;
mod snapshot;
mod sort;
mod storage;
mod types;
mod undo;

pub use document::Document;
pub use format::ParsedTable;
pub use format_planner::{FormatChange, plan_format_change};
pub use io::LoadOutcome;
pub use revert::{RevertDecision, plan_revert};
pub use snapshot::DocumentSnapshot;
pub use storage::{StdStorage, Storage};
pub use types::{
    ActiveSort, DocumentError, DocumentNotice, DocumentPhase, GridSelection, LargeStatus,
};
pub use undo::UndoAction;
