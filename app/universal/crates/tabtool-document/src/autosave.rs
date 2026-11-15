//! The autosave scheduler.
//!
//! A file-backed in-memory document is written 0.8 s after its last change.
//! Untitled and large documents are never autosaved: untitled content has no
//! target, and large files use the explicit streaming save. A blocked gate or
//! a failed write leaves the edits dirty and reports the reason so the window
//! can explain it.
//!
//! The clock is injected through [`Autosave::tick`], so tests advance time
//! without sleeping.

use std::time::{Duration, Instant};

use crate::Document;
use crate::storage::Storage;
use crate::types::{DocumentError, DocumentPhase};

/// How long a change waits before it is written.
pub const AUTOSAVE_DELAY: Duration = Duration::from_millis(800);

/// What one scheduler tick did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutosaveOutcome {
    /// Nothing changed since the last observation, or nothing is due.
    Idle,
    /// A change is waiting for the rest of the debounce.
    Waiting {
        /// Time left before the save is attempted.
        remaining: Duration,
    },
    /// The document was written.
    Saved,
    /// The document is untitled, so there is nowhere to save it.
    Untitled,
    /// Large documents use the streaming save, never autosave.
    Large,
    /// Another operation owns the document; the next tick retries.
    Busy,
    /// The save was refused; the document stays dirty.
    Blocked(DocumentError),
}

/// The debounce state of one document.
#[derive(Debug)]
pub struct Autosave {
    observed_revision: u64,
    last_change: Option<Instant>,
}

impl Autosave {
    /// Starts observing `document` from its current revision.
    pub fn new(document: &Document) -> Self {
        Self {
            observed_revision: document.revision(),
            last_change: None,
        }
    }

    /// Advances the scheduler to `now` and writes the document when due.
    pub fn tick(
        &mut self,
        now: Instant,
        document: &mut Document,
        storage: &dyn Storage,
    ) -> AutosaveOutcome {
        let revision = document.revision();
        if revision != self.observed_revision {
            self.observed_revision = revision;
            self.last_change = Some(now);
        }

        if document.is_large_mode() {
            self.last_change = None;
            return AutosaveOutcome::Large;
        }
        if document.file_path().is_none() {
            self.last_change = None;
            return AutosaveOutcome::Untitled;
        }

        let Some(changed_at) = self.last_change else {
            return AutosaveOutcome::Idle;
        };
        let elapsed = now.saturating_duration_since(changed_at);
        if elapsed < AUTOSAVE_DELAY {
            return AutosaveOutcome::Waiting {
                remaining: AUTOSAVE_DELAY - elapsed,
            };
        }

        if !document.is_dirty() {
            self.last_change = None;
            return AutosaveOutcome::Idle;
        }

        match document.save(storage) {
            Ok(()) => {
                self.last_change = None;
                AutosaveOutcome::Saved
            }
            Err(DocumentError::Busy | DocumentError::RequiresConfirmation) => AutosaveOutcome::Busy,
            Err(DocumentError::NotReady) if matches!(document.phase(), DocumentPhase::Loading) => {
                AutosaveOutcome::Busy
            }
            Err(error) => {
                // A refused write will not succeed on the next tick; wait for
                // the next edit before trying again.
                self.last_change = None;
                AutosaveOutcome::Blocked(error)
            }
        }
    }

    /// Whether the document changed since the last observation.
    ///
    /// The window uses this to wake the root logic after a direct edit, so the
    /// debounce starts even when the root viewport is hidden.
    pub fn has_unobserved_change(&self, document: &Document) -> bool {
        document.revision() != self.observed_revision
    }

    /// The instant the pending change is due, if one is waiting.
    pub fn deadline(&self) -> Option<Instant> {
        self.last_change
            .map(|changed_at| changed_at + AUTOSAVE_DELAY)
    }

    /// How long until the pending save is due; `None` when nothing waits.
    pub fn repaint_after(&self, now: Instant) -> Option<Duration> {
        let deadline = self.deadline()?;
        Some(deadline.saturating_duration_since(now))
    }
}
