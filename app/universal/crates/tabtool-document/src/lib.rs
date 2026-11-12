//! UI-free document state for TabTool.
//!
//! The crate owns one [`Document`] value: its table, phase, selection, sort and
//! save state. It has no UI, no platform code and no async runtime; the desktop
//! and Android UIs call these values from their own threads. File I/O goes
//! through the [`Storage`] trait so tests and platforms can substitute it.

#![forbid(unsafe_code)]

mod document;
mod edit;
mod io;
mod snapshot;
mod storage;
mod types;
mod undo;

pub use document::Document;
pub use io::LoadOutcome;
pub use snapshot::DocumentSnapshot;
pub use storage::{StdStorage, Storage};
pub use types::{
    ActiveSort, DocumentError, DocumentNotice, DocumentPhase, GridSelection, LargeStatus,
};
pub use undo::UndoAction;
