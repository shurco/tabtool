//! The touch editor's persistent state and the work a commit defers.

use std::time::Instant;

use egui::Rect;
use tabtool_core::ColumnId;
use tabtool_document::DocumentNotice;

use super::MobileToolbarResponse;
use crate::grid::{GridAction, GridEditRequest, MoveDirection};

/// The cell or column header the native editor edits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MobileEditTarget {
    /// A data cell, addressed by display row and visual column.
    Cell {
        /// Display row.
        row: usize,
        /// Visual column.
        column: usize,
    },
    /// A column header, addressed by its stable identity.
    Column(ColumnId),
}

/// One native text-editing session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MobileEdit {
    /// The surface the text is written to.
    pub target: MobileEditTarget,
    /// The text the session opened with.
    pub text: String,
    /// Whether the platform editor has been placed.
    pub opened: bool,
    /// Whether a commit was requested and its outcome is still due.
    pub awaiting_commit: bool,
}

/// The touch editor's persistent state.
pub struct MobileEditorState {
    /// The open native edit session, if any.
    pub editing: Option<MobileEdit>,
    /// A one-shot document notice the user has not dismissed.
    pub notice: Option<DocumentNotice>,
    /// When the large file was last checked for staleness.
    pub last_stale_check: Option<Instant>,
    /// The last rectangle the native editor was placed over, in screen points.
    pub last_editor_rect: Option<Rect>,
    /// The screen rectangle the grid occupied in the last frame.
    pub grid_rect: Rect,
    pub(crate) pending: Option<PendingWork>,
    pub(crate) deferred_toolbar: Option<MobileToolbarResponse>,
    pub(crate) deferred_format_bar: bool,
}

impl Default for MobileEditorState {
    fn default() -> Self {
        Self {
            editing: None,
            notice: None,
            last_stale_check: None,
            last_editor_rect: None,
            grid_rect: Rect::NOTHING,
            pending: None,
            deferred_toolbar: None,
            deferred_format_bar: false,
        }
    }
}

/// Work the editor must run after the open edit commits.
#[derive(Default)]
pub(crate) struct PendingWork {
    pub actions: Vec<GridAction>,
    pub toolbar: MobileToolbarResponse,
    pub open: Option<GridEditRequest>,
    pub move_to: Option<MoveDirection>,
    pub undo: bool,
    pub show_format_bar: bool,
}

impl PendingWork {
    /// Whether the work would do anything.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
            && self.toolbar.is_empty()
            && self.open.is_none()
            && self.move_to.is_none()
            && !self.undo
            && !self.show_format_bar
    }
}
