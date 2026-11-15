//! The unsaved-changes prompt, saves, Save As copies and large-file saves.

use super::*;

impl AndroidApp {
    /// Answers the unsaved-changes prompt.
    pub fn answer_prompt(&mut self, choice: PromptChoice) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        match choice {
            PromptChoice::Cancel => {}
            PromptChoice::DontSave => {
                if let Some(editor) = self.editor.as_mut()
                    && editor.document.is_large_mode()
                {
                    editor.document.revert_large_edits();
                }
                self.finish_action(prompt.action);
            }
            PromptChoice::Save => self.save_for_prompt(prompt.action),
        }
    }

    fn save_for_prompt(&mut self, action: AfterSaveAction) {
        let mut saved = false;
        let mut failed = false;
        if let Some(editor) = self.editor.as_mut() {
            if editor.document.is_large_mode() {
                match editor.document.start_large_save(None) {
                    Ok(job) => {
                        drop(job);
                        editor.pending_after_save = Some(action.clone());
                    }
                    Err(error) => {
                        log::error!("could not start the large save: {error:?}");
                        failed = true;
                    }
                }
            } else {
                match editor.document.save(&StdStorage) {
                    Ok(()) => saved = true,
                    Err(error) => {
                        log::error!("could not save: {error:?}");
                        failed = true;
                    }
                }
            }
        } else {
            saved = true;
        }
        if failed {
            self.notice = Some(Notice::Error(ERROR_SAVE));
        }
        if saved {
            self.finish_action(action);
        }
    }

    pub(in crate::app) fn finish_action(&mut self, action: AfterSaveAction) {
        match action {
            AfterSaveAction::CloseEditor => self.close_editor(),
            AfterSaveAction::Open(path) => {
                self.open_document(&path);
            }
            AfterSaveAction::Share => self.share_current_document(),
        }
    }

    pub(super) fn start_save_copy(&mut self) {
        let language = self.language;
        let mut result = None;
        if let Some(editor) = self.editor.as_mut() {
            if !matches!(editor.document.large_status(), LargeStatus::Ready) {
                return;
            }
            editor.save_copy.start(&editor.document, &*self.host);
            result = editor.save_copy.take_result();
        }
        self.apply_copy_result(result, language);
    }

    pub(in crate::app) fn apply_copy_result(
        &mut self,
        result: Option<CopyResult>,
        language: Language,
    ) {
        match result {
            Some(CopyResult::Saved(name)) => {
                self.notice = Some(Notice::Message(saved_copy_message(language, &name)));
            }
            Some(CopyResult::Failed) => self.notice = Some(Notice::Error(ERROR_SAVE)),
            Some(CopyResult::Cancelled) | None => {}
        }
    }

    pub(super) fn cancel_large_save(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            editor.document.cancel_large_save();
            let _ = editor.document.wait_for_large_save();
            editor.pending_after_save = None;
        }
    }

    pub(super) fn reload_stale(&mut self) {
        let result = self
            .editor
            .as_mut()
            .map(|editor| editor.document.reload_large(&StdStorage));
        if matches!(result, Some(Err(_))) {
            self.notice = Some(Notice::Error(ERROR_OPEN));
        }
    }
}
