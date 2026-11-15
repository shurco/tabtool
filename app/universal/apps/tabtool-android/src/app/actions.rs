//! Navigation, sheet and document actions.

use super::*;

impl AndroidApp {
    /// Handles one back gesture from the toolbar or the system.
    pub fn back(&mut self) -> BackResult {
        if self.prompt.is_some() {
            self.prompt = None;
            return BackResult::CancelledPrompt;
        }
        if let Some(sheet) = self.nav.sheet() {
            match sheet {
                SheetKind::Filters => {
                    let can_close = self
                        .editor
                        .as_mut()
                        .is_none_or(|editor| editor.filters.finish(self.text_input.as_mut()));
                    if can_close {
                        self.close_sheet();
                        BackResult::ClosedSheet
                    } else {
                        BackResult::Deferred
                    }
                }
                SheetKind::Export => {
                    let pending = self
                        .editor
                        .as_ref()
                        .and_then(|editor| editor.export.as_ref())
                        .is_some_and(ExportSheet::is_pending);
                    if pending {
                        BackResult::Ignored
                    } else {
                        self.close_sheet();
                        BackResult::ClosedSheet
                    }
                }
                _ => {
                    self.close_sheet();
                    BackResult::ClosedSheet
                }
            }
        } else if self.nav.document().is_some() {
            self.back_from_editor()
        } else {
            self.host.finish_activity();
            BackResult::Exit
        }
    }

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

    /// Opens a sheet from the toolbar, the overflow menu or the library.
    pub fn open_sheet(&mut self, sheet: SheetKind) {
        if self.nav.sheet() == Some(sheet) {
            return;
        }
        let Some(editor) = self.editor.as_mut() else {
            if !lifecycle::library_sheet(sheet) {
                return;
            }
            self.nav.open_sheet(sheet);
            self.persist();
            return;
        };
        match sheet {
            SheetKind::Format => editor.format = FormatSheet::new(),
            SheetKind::Filters => editor.filters = FiltersSheet::new(),
            SheetKind::Export => editor.export = Some(ExportSheet::new(&editor.document)),
            SheetKind::Settings | SheetKind::About => {}
        }
        self.nav.open_sheet(sheet);
        self.persist();
    }

    /// Shares the current document, saving it first when it is safe to do so.
    ///
    /// A Large or write-blocked dirty document asks Save / Don't Save / Cancel;
    /// Don't Save shares the last file on disk.
    pub fn request_share(&mut self) {
        let Some(editor) = self.editor.as_mut() else {
            self.notice = Some(Notice::Error(ERROR_GENERIC));
            return;
        };
        if editor.document.file_path().is_none() {
            self.notice = Some(Notice::Error(ERROR_GENERIC));
            return;
        }
        if lifecycle::close_needs_prompt(&editor.document) {
            self.prompt = Some(SavePrompt {
                action: AfterSaveAction::Share,
                name: editor.title.clone(),
            });
            return;
        }
        if editor.document.is_dirty()
            && let Err(error) = editor.document.save(&StdStorage)
        {
            log::error!("could not save before sharing: {error:?}");
            self.notice = Some(Notice::Error(ERROR_SAVE));
            return;
        }
        self.share_current_document();
    }

    /// Shares the current document through the platform.
    pub fn share_current_document(&mut self) {
        let path = self
            .editor
            .as_ref()
            .and_then(|editor| editor.document.file_path())
            .map(Path::to_path_buf);
        match path {
            Some(path) => self.host.share(&path),
            None => self.notice = Some(Notice::Error(ERROR_GENERIC)),
        }
    }

    /// Applies a toolbar response without drawing it.
    ///
    /// The mobile editor defers toolbar taps until an open edit commits, so
    /// the response already describes settled content.
    pub fn apply_editor_response(&mut self, response: &MobileEditorResponse) {
        if response.toolbar.back {
            let _ = self.back();
            return;
        }
        if response.toolbar.filter {
            self.open_sheet(SheetKind::Filters);
        }
        if response.toolbar.export {
            self.open_sheet(SheetKind::Export);
        }
        if let Some(item) = response.toolbar.overflow {
            match item {
                OverflowItem::Format => self.open_sheet(SheetKind::Format),
                OverflowItem::Settings => self.open_sheet(SheetKind::Settings),
                OverflowItem::About => self.open_sheet(SheetKind::About),
                OverflowItem::Share => self.request_share(),
                OverflowItem::SaveAs => self.start_save_copy(),
            }
        }
        match response.large {
            Some(tabtool_ui::document::LargeUiAction::CancelSave) => self.cancel_large_save(),
            Some(tabtool_ui::document::LargeUiAction::ReloadStale) => self.reload_stale(),
            None => {}
        }
        if response.show_format_bar {
            self.open_sheet(SheetKind::Format);
        }
    }

    /// Applies a library response without drawing it.
    pub fn apply_library_response(&mut self, response: &LibraryResponse) {
        if let Some(path) = &response.open {
            self.request_open(path.clone());
        }
        if response.settings {
            self.open_sheet(SheetKind::Settings);
        }
        if response.about {
            self.open_sheet(SheetKind::About);
        }
    }

    /// Opens `path` in the editor and returns whether it loaded.
    pub fn open_document(&mut self, path: &Path) -> bool {
        let mut document = Document::new(self.settings.new_document_configuration());
        let opened = document.open_file(
            path,
            &StdStorage,
            None,
            self.prefers_chinese,
            self.settings.large_file_threshold_bytes(),
        );
        let ready = matches!(
            document.phase(),
            tabtool_document::DocumentPhase::Ready
                | tabtool_document::DocumentPhase::NeedsAttention(_)
        );
        if opened.is_err() || !ready {
            log::error!("could not open {}: {:?}", path.display(), opened);
            self.notice = Some(Notice::Error(ERROR_OPEN));
            return false;
        }
        self.editor = Some(Editor::new(document, path));
        self.nav.open_document(path.to_path_buf());
        self.prompt = None;
        self.pending = None;
        self.persist();
        true
    }

    pub(super) fn close_sheet(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            match self.nav.sheet() {
                Some(SheetKind::Filters) => editor.filters = FiltersSheet::new(),
                Some(SheetKind::Format) => editor.format = FormatSheet::new(),
                Some(SheetKind::Export) => editor.export = None,
                _ => {}
            }
        }
        self.nav.close_sheet();
        self.persist();
    }

    fn back_from_editor(&mut self) -> BackResult {
        if self.text_input.is_active() {
            self.text_input.commit();
            self.pending = Some(Pending::Back);
            return BackResult::Deferred;
        }
        let Some(editor) = self.editor.as_mut() else {
            self.nav.back();
            return BackResult::ClosedDocument;
        };
        if matches!(editor.document.large_status(), LargeStatus::Saving(_)) {
            return BackResult::Ignored;
        }
        if !editor.document.is_dirty() {
            self.close_editor();
            return BackResult::ClosedDocument;
        }
        if lifecycle::close_needs_prompt(&editor.document) {
            self.prompt = Some(SavePrompt {
                action: AfterSaveAction::CloseEditor,
                name: editor.title.clone(),
            });
            return BackResult::Prompt;
        }
        match editor.document.save(&StdStorage) {
            Ok(()) => {
                self.close_editor();
                BackResult::ClosedDocument
            }
            Err(error) => {
                log::error!("could not save before closing: {error:?}");
                self.prompt = Some(SavePrompt {
                    action: AfterSaveAction::CloseEditor,
                    name: editor.title.clone(),
                });
                BackResult::Prompt
            }
        }
    }

    fn close_editor(&mut self) {
        self.editor = None;
        self.pending = None;
        self.prompt = None;
        while self.nav.close_sheet() {}
        self.nav.back();
        self.persist();
    }

    pub(super) fn request_open(&mut self, path: PathBuf) {
        if let Some(editor) = self.editor.as_mut() {
            if matches!(editor.document.large_status(), LargeStatus::Saving(_)) {
                self.notice = Some(Notice::Error(ERROR_GENERIC));
                return;
            }
            if editor.document.is_dirty() {
                if lifecycle::close_needs_prompt(&editor.document) {
                    self.prompt = Some(SavePrompt {
                        action: AfterSaveAction::Open(path),
                        name: editor.title.clone(),
                    });
                    return;
                }
                if let Err(error) = editor.document.save(&StdStorage) {
                    log::error!("could not save before opening another document: {error:?}");
                    self.notice = Some(Notice::Error(ERROR_SAVE));
                    return;
                }
            }
        }
        self.open_document(&path);
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

    pub(super) fn finish_action(&mut self, action: AfterSaveAction) {
        match action {
            AfterSaveAction::CloseEditor => self.close_editor(),
            AfterSaveAction::Open(path) => {
                self.open_document(&path);
            }
            AfterSaveAction::Share => self.share_current_document(),
        }
    }

    fn start_save_copy(&mut self) {
        let files = library::AppFiles::new(Rc::clone(&self.host));
        let language = self.language;
        let mut result = None;
        if let Some(editor) = self.editor.as_mut() {
            if !matches!(editor.document.large_status(), LargeStatus::Ready) {
                return;
            }
            editor.save_copy.start(&editor.document, &files);
            result = editor.save_copy.take_result();
        }
        self.apply_copy_result(result, language);
    }

    pub(super) fn apply_copy_result(&mut self, result: Option<CopyResult>, language: Language) {
        match result {
            Some(CopyResult::Saved(name)) => {
                self.notice = Some(Notice::Message(saved_copy_message(language, &name)));
            }
            Some(CopyResult::Failed) => self.notice = Some(Notice::Error(ERROR_SAVE)),
            Some(CopyResult::Cancelled) | None => {}
        }
    }

    fn cancel_large_save(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            editor.document.cancel_large_save();
            let _ = editor.document.wait_for_large_save();
            editor.pending_after_save = None;
        }
    }

    fn reload_stale(&mut self) {
        let result = self
            .editor
            .as_mut()
            .map(|editor| editor.document.reload_large(&StdStorage));
        if matches!(result, Some(Err(_))) {
            self.notice = Some(Notice::Error(ERROR_OPEN));
        }
    }
}
