//! Grid action dispatch and the egui clipboard for a document window.
//!
//! The grid works in display rows and never touches the document. The shared
//! dispatcher in `tabtool-ui` translates a filtered grid's display rows to
//! model rows, applies the action, and maps the resulting selection and focus
//! back to display rows. This module supplies the window lock and the egui
//! clipboard.

use egui::{Context, Event};
use tabtool_document::DocumentError;
use tabtool_ui::document::apply_grid_action as dispatch_grid_action;
use tabtool_ui::grid::{ActionOutcome, Clipboard, GridAction, GridFocus, GridSelection};

use super::DocumentWindow;

/// The platform clipboard as egui exposes it.
///
/// Copy is queued as an egui output command. Paste arrives as an
/// [`Event::Paste`] from the backend; the event is taken so a text field cannot
/// paste the same text twice. A menu-initiated paste has no event yet: the
/// window requests the platform paste once and leaves the paste pending until
/// the event arrives.
pub struct EguiClipboard<'a> {
    ctx: &'a Context,
}

impl<'a> EguiClipboard<'a> {
    /// Wraps this frame's clipboard context.
    ///
    /// Construction does not take a paste event: only an actual paste attempt
    /// consumes it, so a focused text field can still receive the event when
    /// the document did not ask for a paste.
    pub fn new(ctx: &'a Context) -> Self {
        Self { ctx }
    }
}

impl Clipboard for EguiClipboard<'_> {
    fn copy(&mut self, text: &str) {
        self.ctx.copy_text(text.to_owned());
    }

    fn paste(&mut self) -> Option<String> {
        take_paste(self.ctx)
    }
}

fn take_paste(ctx: &Context) -> Option<String> {
    ctx.input_mut(|input| {
        let index = input
            .events
            .iter()
            .position(|event| matches!(event, Event::Paste(_)))?;
        match input.events.remove(index) {
            Event::Paste(text) => Some(text),
            _ => None,
        }
    })
}

impl DocumentWindow {
    /// Applies one grid action to the document.
    ///
    /// The grid selection and action rows are display rows. They are translated
    /// through the document's visible-row filter before dispatch, and the
    /// resulting model selection and focus are translated back to display
    /// rows.
    pub fn apply_grid_action(
        &self,
        action: GridAction,
        clipboard: &mut dyn Clipboard,
    ) -> Result<ActionOutcome, DocumentError> {
        let mut state = self.lock();
        let locale = state.locale.clone();
        let decimal_separator = state.decimal_separator.clone();
        let state = &mut *state;
        dispatch_grid_action(
            &mut state.document,
            &mut state.grid,
            action,
            clipboard,
            &locale,
            &decimal_separator,
        )
    }

    /// The grid's row/column selection in display rows.
    pub fn grid_selection(&self) -> GridSelection {
        self.lock().grid.selection.clone()
    }

    /// Replaces the grid selection with display rows and columns.
    pub fn set_grid_selection(&self, selection: GridSelection) {
        let mut state = self.lock();
        state.grid.selection = selection;
        state.grid.anchor = None;
    }

    /// The focused grid cell in display coordinates.
    pub fn grid_focus(&self) -> Option<GridFocus> {
        self.lock().grid.focus
    }

    /// Sets the focused grid cell and requests keyboard focus for it.
    pub fn set_grid_focus(&self, focus: Option<GridFocus>) {
        self.lock().grid.focus_cell(focus);
    }

    /// Takes the document's one-shot notice for the window to present.
    pub fn take_notice(&self) -> Option<tabtool_document::DocumentNotice> {
        self.lock().document.take_notice()
    }

    /// Whether a platform paste is still waiting for its event.
    pub fn has_pending_paste(&self) -> bool {
        self.lock().chrome.pending_paste
    }
}

#[cfg(test)]
mod tests {
    use egui::{Context, Event, OutputCommand};
    use tabtool_ui::grid::Clipboard;

    use super::EguiClipboard;

    #[test]
    fn paste_reads_and_consumes_the_event() {
        let ctx = Context::default();
        ctx.begin_pass(egui::RawInput {
            events: vec![Event::Paste("x\ty".to_owned())],
            ..Default::default()
        });
        let mut clipboard = EguiClipboard::new(&ctx);

        assert_eq!(clipboard.paste().as_deref(), Some("x\ty"));
        assert_eq!(clipboard.paste(), None);
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
    }

    #[test]
    fn constructing_the_clipboard_leaves_the_paste_event() {
        let ctx = Context::default();
        ctx.begin_pass(egui::RawInput {
            events: vec![Event::Paste("keep".to_owned())],
            ..Default::default()
        });
        let _clipboard = EguiClipboard::new(&ctx);

        assert!(
            ctx.input(|input| input
                .events
                .iter()
                .any(|event| matches!(event, Event::Paste(text) if text == "keep"))),
            "an unused clipboard must not consume a field's paste event"
        );

        let mut output = ctx.end_pass();
        output.textures_delta.clear();
    }

    #[test]
    fn copy_queues_an_egui_output_command() {
        let ctx = Context::default();
        ctx.begin_pass(egui::RawInput::default());
        EguiClipboard::new(&ctx).copy("a\tb");
        let commands = ctx.output(|output| output.commands.clone());
        let mut output = ctx.end_pass();
        output.textures_delta.clear();

        assert!(matches!(
            commands.first(),
            Some(OutputCommand::CopyText(text)) if text == "a\tb"
        ));
    }
}
