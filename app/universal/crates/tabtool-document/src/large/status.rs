//! Indexing progress and on-disk staleness of a large document.

use tabtool_large::Staleness;

use crate::Document;
use crate::types::{DocumentPhase, LargeStatus};

impl Document {
    /// Publishes the indexer's progress into [`Document::large_status`].
    ///
    /// While the scan runs the status is [`LargeStatus::Indexing`] with the
    /// indexer's fraction; once it finishes the status becomes
    /// [`LargeStatus::Ready`]. A stale or saving document keeps its status.
    /// Rows are readable through [`Document::row_source`] throughout, but
    /// [`Document::can_edit_cells`] only turns true when the index is complete.
    pub fn poll_large_indexing(&mut self) {
        if matches!(
            self.large_status,
            LargeStatus::Stale { .. } | LargeStatus::Saving(_)
        ) {
            return;
        }
        let Some(large) = self.large.as_ref() else {
            self.large_status = LargeStatus::None;
            return;
        };
        #[cfg(feature = "test-support")]
        if let Some(fraction) = self.pinned_indexing {
            self.phase = DocumentPhase::Ready;
            self.large_status = LargeStatus::Indexing(fraction);
            return;
        }
        let status = if large.indexer.is_finished() {
            LargeStatus::Ready
        } else {
            LargeStatus::Indexing(large.source.indexing_fraction())
        };
        self.phase = DocumentPhase::Ready;
        self.large_status = status;
    }

    /// Marks the document stale when the file changed on disk since it was
    /// mapped.
    ///
    /// A file that only grew reports [`LargeStatus::Stale`] with
    /// `appended = true`; a replaced or truncated file reports it with
    /// `appended = false`. The phase stays [`DocumentPhase::Ready`]. Only a
    /// ready document is checked, so indexing progress is not overwritten.
    pub fn refresh_large_staleness(&mut self) {
        let Some(large) = self.large.as_ref() else {
            return;
        };
        if self.large_status != LargeStatus::Ready {
            return;
        }
        match large.source.staleness() {
            Staleness::Unchanged => {}
            Staleness::Appended => self.large_status = LargeStatus::Stale { appended: true },
            Staleness::ReplacedOrTruncated => {
                self.large_status = LargeStatus::Stale { appended: false };
            }
        }
    }
}
