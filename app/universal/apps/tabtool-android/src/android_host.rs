//! The Android implementation of the app's platform seam.
//!
//! Every call forwards to `tabtool_platform::android`, which owns the JNI
//! bridge. The native text editor is placed in the activity's content view,
//! while the app draws inside the safe area, so the request rectangle is
//! shifted by the content origin before it reaches the platform.

use std::io;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use tabtool_platform::android::{AndroidPlatform, IntentImport};
use tabtool_platform::{
    BackHandler, Clipboard, DocumentFiles, Insets, OpenUrl, PlatformErrors, Share, SystemInfo,
    TextInput, TextInputRequest,
};
use tabtool_ui::grid::Clipboard as GridClipboard;
use tabtool_ui::mobile::{
    DocumentMetadata, ExportOutcome, LibraryClock, LibraryError, TouchTextInput,
    TouchTextInputOutcome, TouchTextInputRequest,
};

use crate::app::{AndroidApp, Host};
use crate::intents::{IntentEvent, StagedIntent};

/// The real Android host.
#[derive(Clone, Copy, Debug, Default)]
pub struct AndroidHost;

/// Set by the JNI pause callback before the activity pauses.
static BACKGROUND_SAVE_REQUESTED: AtomicBool = AtomicBool::new(false);
/// Set once the native app has flushed the requested background save.
static BACKGROUND_SAVE_FINISHED: AtomicBool = AtomicBool::new(false);

/// Takes a background-save request made synchronously from the pause path.
pub fn take_background_save_request() -> bool {
    BACKGROUND_SAVE_REQUESTED.swap(false, Ordering::AcqRel)
}

/// Acknowledges the pause-path save so the UI thread can stop waiting.
pub fn finish_background_save() {
    BACKGROUND_SAVE_FINISHED.store(true, Ordering::Release);
}

/// Resets the acknowledgement before a new pause-path request.
pub fn begin_background_save() {
    BACKGROUND_SAVE_FINISHED.store(false, Ordering::Release);
    BACKGROUND_SAVE_REQUESTED.store(true, Ordering::Release);
}

/// Whether the app acknowledged the last pause-path request.
pub fn background_save_acknowledged() -> bool {
    BACKGROUND_SAVE_FINISHED.load(Ordering::Acquire)
}

impl Host for AndroidHost {
    fn documents_dir(&self) -> PathBuf {
        DocumentFiles::documents_dir(&AndroidPlatform)
    }

    fn list_documents(&self) -> Vec<PathBuf> {
        DocumentFiles::list_documents(&AndroidPlatform)
    }

    fn existing_names(&self) -> Vec<String> {
        std::fs::read_dir(self.documents_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect()
    }

    fn create_document(&self, name: &str, data: &[u8]) -> io::Result<PathBuf> {
        let candidate = Path::new(name);
        if name.is_empty() || candidate.file_name().and_then(|name| name.to_str()) != Some(name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the document name is not a single file name",
            ));
        }
        let directory = self.documents_dir();
        std::fs::create_dir_all(&directory)?;
        let path = directory.join(name);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(data)?;
        Ok(path)
    }

    fn import_from_picker(&self) -> Option<PathBuf> {
        DocumentFiles::import_from_picker(&AndroidPlatform)
    }

    fn import_in_progress(&self) -> bool {
        AndroidPlatform.import_pending()
    }

    fn export_copy(&self, source: &Path, suggested_name: &str) -> ExportOutcome {
        let copied = DocumentFiles::export_copy(&AndroidPlatform, source, suggested_name);
        crate::app::export_outcome(
            copied,
            AndroidPlatform.export_pending(),
            AndroidPlatform.take_export_outcome(),
        )
    }

    fn export_in_progress(&self) -> bool {
        AndroidPlatform.export_pending()
    }

    fn delete(&self, path: &Path) -> io::Result<()> {
        DocumentFiles::delete(&AndroidPlatform, path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        DocumentFiles::rename(&AndroidPlatform, from, to)
    }

    fn take_error(&self) -> Option<LibraryError> {
        PlatformErrors::take_error(&AndroidPlatform)
            .map(|error| LibraryError::new(error.message_key, error.detail))
    }

    fn metadata(&self, path: &Path) -> io::Result<DocumentMetadata> {
        let metadata = std::fs::metadata(path)?;
        if !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} is not a regular file", path.display()),
            ));
        }
        Ok(DocumentMetadata {
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
        })
    }

    fn safe_area(&self) -> [f32; 4] {
        Insets::safe_area(&AndroidPlatform)
    }

    fn keyboard_height(&self) -> f32 {
        Insets::keyboard_height(&AndroidPlatform)
    }

    fn take_back(&self) -> bool {
        BackHandler::take_back(&AndroidPlatform)
    }

    fn share(&self, path: &Path) {
        Share::share(&AndroidPlatform, path);
    }

    fn finish_activity(&self) {
        AndroidPlatform.finish_activity();
    }

    fn languages(&self) -> Vec<String> {
        SystemInfo::languages(&AndroidPlatform)
    }

    fn dark_mode(&self) -> Option<bool> {
        Some(AndroidPlatform.dark_mode())
    }

    fn locale(&self) -> String {
        SystemInfo::locale(&AndroidPlatform)
    }

    fn clock(&self) -> LibraryClock {
        AndroidPlatform
            .clock()
            .map_or_else(LibraryClock::default, |(offset, twelve_hour)| {
                LibraryClock::new(offset, twelve_hour)
            })
    }

    fn settings_path(&self) -> Option<PathBuf> {
        AndroidPlatform
            .files_dir()
            .map(|directory| directory.join("settings.json"))
    }

    fn state_path(&self) -> Option<PathBuf> {
        AndroidPlatform
            .files_dir()
            .map(|directory| directory.join("app-state.json"))
    }

    fn take_background_event(&self) -> bool {
        take_background_save_request() || AndroidPlatform.take_background_event()
    }

    fn background_save_finished(&self) {
        finish_background_save();
    }

    fn take_intent(&self) -> Option<IntentEvent> {
        match AndroidPlatform.take_intent_import()? {
            IntentImport::Staged {
                file_name,
                display_name,
            } => Some(IntentEvent::Staged(StagedIntent {
                file_name,
                display_name,
            })),
            IntentImport::Failed(detail) => Some(IntentEvent::Failed(detail)),
        }
    }

    fn open_url(&self, url: &str) {
        OpenUrl::open(&AndroidPlatform, url);
    }
}

/// The platform text editor, shifted into window coordinates.
#[derive(Clone, Copy, Debug, Default)]
pub struct AndroidTextInput;

impl TouchTextInput for AndroidTextInput {
    fn open(&mut self, request: TouchTextInputRequest) {
        self.open_at(request.rect, request.text, request.multiline);
    }

    fn close(&mut self) {
        TextInput::close(&AndroidPlatform);
    }

    fn commit(&mut self) {
        TextInput::commit(&AndroidPlatform);
    }

    fn move_to(&mut self, rect: eframe::egui::Rect) {
        TextInput::move_to(
            &AndroidPlatform,
            crate::app::text_input_rect(rect, self.safe_area()),
        );
    }

    fn poll(&mut self) -> Option<TouchTextInputOutcome> {
        TextInput::poll(&AndroidPlatform).map(|outcome| match outcome {
            tabtool_platform::TextInputOutcome::Committed(text) => {
                TouchTextInputOutcome::Committed(text)
            }
            tabtool_platform::TextInputOutcome::Cancelled => TouchTextInputOutcome::Cancelled,
        })
    }

    fn is_active(&self) -> bool {
        TextInput::is_active(&AndroidPlatform)
    }
}

impl AndroidTextInput {
    fn safe_area(&self) -> [f32; 4] {
        Insets::safe_area(&AndroidPlatform)
    }

    fn open_at(&self, rect: eframe::egui::Rect, text: String, multiline: bool) {
        TextInput::open(
            &AndroidPlatform,
            TextInputRequest {
                rect: crate::app::text_input_rect(rect, self.safe_area()),
                text,
                multiline,
            },
        );
    }
}

/// The platform clipboard adapted to the grid's seam.
#[derive(Clone, Copy, Debug, Default)]
pub struct AndroidClipboard;

impl GridClipboard for AndroidClipboard {
    fn copy(&mut self, text: &str) {
        Clipboard::set(&AndroidPlatform, text);
    }

    fn paste(&mut self) -> Option<String> {
        Clipboard::get(&AndroidPlatform)
    }
}

/// Builds the real app for the eframe entry point.
pub fn create_app() -> AndroidApp {
    AndroidApp::new(
        Rc::new(AndroidHost),
        Box::new(AndroidTextInput),
        Box::new(AndroidClipboard),
    )
}
