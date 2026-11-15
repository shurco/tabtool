//! The Filters sheet's native text session and the filter actions it runs.

use tabtool_document::Document;

use super::FiltersSheet;
use crate::filter::FilterAction;
use crate::mobile::{TouchTextInput, TouchTextInputOutcome};

impl FiltersSheet {
    /// Requests a native commit before the host closes the sheet from
    /// navigation.
    ///
    /// Returns whether the sheet can close now. When it returns `false`, the
    /// host must keep presenting the sheet so [`Self::show`] can drain the
    /// commit and then report its close.
    pub fn finish(&mut self, text_input: &mut dyn TouchTextInput) -> bool {
        if self.editing.is_some() {
            self.pending_close = true;
            text_input.commit();
            false
        } else {
            true
        }
    }

    pub(super) fn apply(
        &mut self,
        action: FilterAction,
        document: &mut Document,
        text_input: &mut dyn TouchTextInput,
    ) {
        match action {
            FilterAction::ApplyAll => document.apply_all_filters(),
            FilterAction::Apply(id) => document.apply_filter(id),
            FilterAction::Clear => {
                document.clear_filter();
                if self.editing.is_some() {
                    self.editing = None;
                    text_input.close();
                }
            }
            FilterAction::Add(id) => document.row_filter_mut().add_criterion(id),
            FilterAction::Remove(id) => {
                document.row_filter_mut().remove_criterion(id);
                if self.editing == Some(id) {
                    self.editing = None;
                    text_input.close();
                }
            }
            FilterAction::Hide => {}
        }
    }

    pub(super) fn poll_text_input(
        &mut self,
        document: &mut Document,
        text_input: &mut dyn TouchTextInput,
    ) -> bool {
        let Some(id) = self.editing else {
            return false;
        };
        match text_input.poll() {
            Some(TouchTextInputOutcome::Committed(text)) => {
                self.editing = None;
                if let Some(criterion) = document
                    .row_filter_mut()
                    .criteria
                    .iter_mut()
                    .find(|criterion| criterion.id == id)
                {
                    criterion.value = text;
                }
                true
            }
            Some(TouchTextInputOutcome::Cancelled) => {
                self.editing = None;
                self.pending_action = None;
                false
            }
            None if !text_input.is_active() => {
                self.editing = None;
                self.pending_action = None;
                false
            }
            None => false,
        }
    }
}
