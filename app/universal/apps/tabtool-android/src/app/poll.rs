//! Event polling, background saving and process-death state.

use super::*;

impl AndroidApp {
    /// Advances time, platform events and autosave without drawing.
    pub fn poll(&mut self, now: Instant) {
        self.sync_nav_notice();
        self.poll_platform_error();
        self.poll_intents();
        if self.host.take_back() {
            let _ = self.back();
        }
        if self.host.take_background_event() {
            self.background_pending = true;
            self.persist();
        }
        self.poll_editor(now);
        self.poll_pending();
        self.flush_background();
    }

    pub(super) fn flush_background(&mut self) {
        if !self.background_pending {
            return;
        }
        if self.text_input.is_active()
            || self
                .editor
                .as_ref()
                .is_some_and(|editor| editor.state.editing.is_some())
        {
            return;
        }
        self.background_pending = false;
        let mut failed = false;
        if let Some(editor) = self.editor.as_mut() {
            match lifecycle::save_for_background(&mut editor.document, &StdStorage) {
                Ok(true) => {}
                Ok(false) => {}
                Err(error) => {
                    log::error!("background save failed: {error:?}");
                    failed = true;
                }
            }
        }
        if failed {
            self.notice = Some(Notice::Error(ERROR_SAVE));
        }
        self.host.background_save_finished();
    }

    fn poll_editor(&mut self, now: Instant) {
        let mut failed = false;
        let mut copy_result = None;
        let mut after_save = None;
        let files = library::AppFiles::new(Rc::clone(&self.host));
        let language = self.language;
        let prompt_open = self.prompt.is_some();

        if let Some(editor) = self.editor.as_mut() {
            if !prompt_open
                && let Some(error) = lifecycle::tick_autosave(
                    &mut editor.autosave,
                    now,
                    &mut editor.document,
                    &StdStorage,
                )
                && !matches!(error, DocumentError::Csv(_))
            {
                failed = true;
            }

            editor.save_copy.poll(&files);
            copy_result = editor.save_copy.take_result();

            if matches!(editor.document.large_status(), LargeStatus::Saving(_)) {
                let finished = editor
                    .document
                    .large_save_progress()
                    .is_some_and(|progress| progress >= 1.0)
                    || editor.document.large_save_finished();
                if finished {
                    match editor.document.wait_for_large_save() {
                        Ok(()) => after_save = editor.pending_after_save.take(),
                        Err(error) => {
                            log::error!("large save failed: {error:?}");
                            editor.pending_after_save = None;
                            failed = true;
                        }
                    }
                }
            } else if let Some(action) = editor.pending_after_save.take() {
                // A save that settled between frames still needs its join so
                // the mapping is adopted before the app opens or closes.
                match editor.document.wait_for_large_save() {
                    Ok(()) => after_save = Some(action),
                    Err(error) => {
                        log::error!("large save failed: {error:?}");
                        failed = true;
                    }
                }
            }
        }

        if failed {
            self.notice = Some(Notice::Error(ERROR_SAVE));
        }
        self.apply_copy_result(copy_result, language);
        if let Some(action) = after_save {
            self.finish_action(action);
        }
    }

    fn poll_pending(&mut self) {
        if self.pending != Some(Pending::Back) || self.text_input.is_active() {
            return;
        }
        if self
            .editor
            .as_ref()
            .is_some_and(|editor| editor.state.editing.is_some())
        {
            return;
        }
        self.pending = None;
        let _ = self.back();
    }

    fn poll_platform_error(&mut self) {
        if let Some(error) = self.host.take_error() {
            self.notice = Some(Notice::Error(error.message_key));
        }
    }

    fn poll_intents(&mut self) {
        while let Some(event) = self.host.take_intent() {
            match event {
                intents::IntentEvent::Staged(staged) => {
                    match intents::adopt(&self.host.documents_dir(), &staged) {
                        Ok(path) => self.request_open(path),
                        Err(error) => {
                            log::error!("could not adopt the intent document: {error}");
                            self.notice = Some(Notice::Error(ERROR_OPEN));
                        }
                    }
                }
                intents::IntentEvent::Failed(detail) => {
                    log::error!("the intent copy failed: {detail}");
                    self.notice = Some(Notice::Error(ERROR_OPEN));
                }
            }
        }
    }

    fn sync_nav_notice(&mut self) {
        if let Some(key) = self.nav.take_notice() {
            self.notice = Some(Notice::Error(key));
        }
    }

    pub(super) fn persist(&self) {
        if let Err(error) =
            lifecycle::save_state(self.host.state_path().as_deref(), &nav::snapshot(&self.nav))
        {
            log::error!("could not persist the session: {error}");
        }
    }

    pub(super) fn restore(&mut self, snapshot: &NavSnapshot) {
        let mut missing = false;
        if let Some(path) = &snapshot.document {
            let directory = self.host.documents_dir();
            if lifecycle::restorable(&directory, path) {
                if !self.open_document(path) {
                    missing = true;
                }
            } else {
                missing = true;
            }
        }
        if missing {
            self.nav.document_missing();
            self.sync_nav_notice();
            self.persist();
        }
        for sheet in &snapshot.sheets {
            if self.editor.is_some() || lifecycle::library_sheet(*sheet) {
                self.open_sheet(*sheet);
            }
        }
    }
}
