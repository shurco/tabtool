//! The touch editor: mobile toolbar, touch grid, contextual action bar, status
//! line, large-file banners and the keyboard accessory.
//!
//! The view owns no text field. Cell and header edits open the platform editor
//! through [`TouchTextInput`]; the editor places it over the cell, waits for
//! the commit and then runs any action that arrived during the edit.

use egui::{Frame, Panel, Ui};
use tabtool_core::Locale;
use tabtool_document::{Document, LargeStatus};
use tabtool_i18n::Language;

use super::editor_chrome::touch_options;
use super::editor_session::EditorSession;
use super::editor_state::{MobileEditorState, PendingWork};
use super::{
    AccessoryAction, FoldRegion, MobileMetrics, MobileToolbar, MobileToolbarResponse,
    TouchTextInput, actions_for, editor_accessory,
};
use crate::document::{LargeUiAction, can_export, enabled_filter_count};
use crate::grid::{
    Clipboard, GridAction, GridAppearance, GridEditRequest, GridOptions, GridPresenter, GridState,
    GridView,
};
use crate::theme::current;

/// What the touch editor asks its host to do.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MobileEditorResponse {
    /// Navigation and app-level toolbar requests.
    pub toolbar: MobileToolbarResponse,
    /// A large-file control routed to the document.
    pub large: Option<LargeUiAction>,
    /// The needs-attention banner asked for the format controls.
    pub show_format_bar: bool,
}

/// The touch document editor.
///
/// The app owns the document, the grid state and the platform text input; the
/// view composes the chrome and routes every document change through the
/// model. A single instance exists per open document.
pub struct MobileEditor<'a> {
    /// The document the editor changes.
    pub document: &'a mut Document,
    /// The persistent scroll, selection, width and focus state.
    pub state: &'a mut GridState,
    /// The open native edit, deferred work and notices.
    pub editor_state: &'a mut MobileEditorState,
    /// The platform text editor.
    pub text_input: &'a mut dyn TouchTextInput,
    /// The platform clipboard for grid commands.
    pub clipboard: &'a mut dyn Clipboard,
    /// The phone metrics, including the safe area.
    pub metrics: MobileMetrics,
    /// The interface language.
    pub language: Language,
    /// The document name shown in the toolbar.
    pub title: &'a str,
    /// The system locale for counts and sorting.
    pub locale: &'a Locale,
    /// The system decimal separator for clipboard formatting.
    pub decimal_separator: &'a str,
    /// The row height, alternating rows and row numbers.
    pub appearance: GridAppearance,
    /// The keyboard inset the platform reports, in points.
    pub keyboard_height: f32,
    /// The active fold region, in screen points.
    pub fold: Option<FoldRegion>,
}

impl MobileEditor<'_> {
    /// Draws the editor and returns the app-level requests it collected.
    pub fn show(mut self, ui: &mut Ui) -> MobileEditorResponse {
        let ctx = ui.ctx().clone();
        let options = self.options();
        let mut response = MobileEditorResponse::default();

        self.with_session(&options, |session| {
            session.poll_large(&ctx);
            session.take_notice();
            session.poll_outcome();
            session.sync_selection();
        });

        self.show_toolbar(ui, &options, &mut response);

        let editing = self.editor_state.editing.is_some();
        let status = self.document.large_status();
        let saving = match status {
            LargeStatus::Saving(fraction) => Some(fraction),
            _ => None,
        };
        let actions = actions_for(self.document, &self.state.selection);
        let show_action_bar = !editing && !actions.is_empty();

        // A keyboard slot is reserved from the frame the native editor opens,
        // so the accessory and the edited cell stay above it.
        if editing && self.keyboard_height > 0.0 {
            Panel::bottom("mobile-editor-keyboard")
                .exact_size(self.keyboard_height)
                .frame(Frame::new().fill(current(&ctx).colors.window_background))
                .show_separator_line(false)
                .show(ui, |_| {});
        }
        if editing {
            Panel::bottom("mobile-editor-accessory")
                .exact_size(self.metrics.accessory_height)
                .frame(Frame::NONE)
                .show_separator_line(false)
                .show(ui, |ui| {
                    if let Some(action) = editor_accessory(ui, self.metrics, self.language) {
                        self.run_accessory(action, &options);
                    }
                });
        }

        self.show_status(ui, &mut response, saving);
        self.show_banners(ui, &mut response, &options);
        if show_action_bar {
            self.show_actions(ui, &actions, &options);
        }

        let (grid_actions, request) = self.show_grid(ui, &options);
        let mut work = PendingWork {
            actions: grid_actions,
            ..PendingWork::default()
        };
        work.open = request;
        if !work.is_empty() {
            self.with_session(&options, |session| session.submit(work));
        }
        self.with_session(&options, |session| session.maybe_open());

        if let Some(deferred) = self.editor_state.deferred_toolbar.take() {
            response.toolbar.merge(deferred);
        }
        response.show_format_bar |= std::mem::take(&mut self.editor_state.deferred_format_bar);

        self.with_session(&options, |session| session.sync_document());
        response
    }

    fn options(&self) -> GridOptions {
        touch_options(self.document, self.language, self.appearance, self.metrics)
    }

    pub(super) fn with_session<R>(
        &mut self,
        options: &GridOptions,
        f: impl FnOnce(&mut EditorSession<'_>) -> R,
    ) -> R {
        let mut session = EditorSession {
            document: &mut *self.document,
            state: &mut *self.state,
            editor: &mut *self.editor_state,
            text_input: &mut *self.text_input,
            clipboard: &mut *self.clipboard,
            locale: self.locale,
            decimal_separator: self.decimal_separator,
            options: options.clone(),
        };
        f(&mut session)
    }

    fn show_toolbar(
        &mut self,
        ui: &mut Ui,
        options: &GridOptions,
        response: &mut MobileEditorResponse,
    ) {
        let mut toolbar = Panel::top("mobile-editor-toolbar")
            .exact_size(self.metrics.toolbar_height)
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                MobileToolbar::new(self.metrics, self.language, self.title)
                    .modified(self.document.is_dirty())
                    .history(self.document.can_undo(), self.document.can_redo())
                    .filter(
                        self.document.can_filter(),
                        enabled_filter_count(self.document),
                    )
                    .export(can_export(self.document))
                    .large(self.document.is_large_mode())
                    .show(ui)
            })
            .inner;
        if toolbar.is_empty() {
            return;
        }
        if self.editor_state.editing.is_some() {
            let work = PendingWork {
                toolbar,
                ..PendingWork::default()
            };
            self.with_session(options, |session| session.submit(work));
            return;
        }
        if toolbar.undo {
            self.document.undo();
            toolbar.undo = false;
        }
        if toolbar.redo {
            self.document.redo();
            toolbar.redo = false;
        }
        response.toolbar.merge(toolbar);
    }

    fn run_accessory(&mut self, action: AccessoryAction, options: &GridOptions) {
        let work = match action {
            AccessoryAction::Done => PendingWork::default(),
            AccessoryAction::Next => PendingWork {
                move_to: Some(crate::grid::MoveDirection::Right),
                ..PendingWork::default()
            },
            AccessoryAction::Previous => PendingWork {
                move_to: Some(crate::grid::MoveDirection::Left),
                ..PendingWork::default()
            },
        };
        self.with_session(options, |session| session.submit(work));
    }

    fn show_grid(
        &mut self,
        ui: &mut Ui,
        options: &GridOptions,
    ) -> (Vec<GridAction>, Option<GridEditRequest>) {
        let background = current(ui.ctx()).colors.content_background;
        egui::CentralPanel::default()
            .frame(Frame::new().fill(background))
            .show(ui, |ui| {
                self.editor_state.grid_rect = ui.max_rect();
                let Some(source) = self.document.row_source() else {
                    return (Vec::new(), None);
                };
                let visible = self.document.row_filter().visible_rows.as_deref();
                let presenter = GridPresenter::new(source, visible);
                GridView {
                    source: &presenter,
                    state: &mut *self.state,
                    options: options.clone(),
                }
                .show_touch(ui)
            })
            .inner
    }
}
