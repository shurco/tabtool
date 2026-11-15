//! The autosave scheduler, shared with every app.
//!
//! A file-backed in-memory document is written 0.8 s after its last change.
//! Untitled and large documents are never autosaved: untitled content has no
//! target, and large files use the explicit streaming save. The clock is
//! injected through [`Autosave::tick`], so tests advance time without sleeping.
//!
//! The scheduler lives in `tabtool-document` because it is document behavior,
//! not window behavior; the desktop app re-exports it for its existing callers.

pub use tabtool_document::{AUTOSAVE_DELAY, Autosave, AutosaveOutcome};
