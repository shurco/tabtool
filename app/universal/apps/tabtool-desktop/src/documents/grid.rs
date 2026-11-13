//! Grid action dispatch and the egui clipboard for a document window.
//!
//! The grid returns [`GridAction`]s and never touches the document. This module
//! keeps the document selection in step with the grid, applies the action, and
//! mirrors the resulting selection back so painted selection and commands agree.

use egui::{Context, Event};
use tabtool_document::DocumentError;
use tabtool_ui::grid::{Clipboard, GridAction, apply_action};

use super::DocumentWindow;

/// The platform clipboard as egui exposes it.
///
/// Copy is queued as an egui output command. Paste arrives as an
/// [`Event::Paste`] from the backend; the event is taken so a text field cannot
/// paste the same text twice.
pub struct EguiClipboard<'a> {
    ctx: &'a Context,
    paste: Option<String>,
}

impl<'a> EguiClipboard<'a> {
    /// Wraps this frame's clipboard events.
    pub fn new(ctx: &'a Context) -> Self {
        Self {
            ctx,
            paste: take_paste(ctx),
        }
    }
}

impl Clipboard for EguiClipboard<'_> {
    fn copy(&mut self, text: &str) {
        self.ctx.copy_text(text.to_owned());
    }

    fn paste(&mut self) -> Option<String> {
        self.paste.take()
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
    /// The grid selection is authoritative for the visible grid, so it is
    /// copied into the document before the action and the document's resulting
    /// selection is copied back. A grid over a filtered presenter must
    /// translate its display rows before calling this method.
    pub fn apply_grid_action(
        &self,
        action: GridAction,
        clipboard: &mut dyn Clipboard,
    ) -> Result<(), DocumentError> {
        let mut state = self.lock();
        let grid_selection = state.grid.document_selection();
        *state.document.selection_mut() = grid_selection;
        let locale = state.locale.clone();
        let decimal_separator = state.decimal_separator.clone();
        let result = apply_action(
            &mut state.document,
            action,
            &locale,
            &decimal_separator,
            clipboard,
        );
        let document_selection = state.document.selection().clone();
        state.grid.apply_document_selection(&document_selection);
        result
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
