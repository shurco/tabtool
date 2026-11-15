//! Android platform services through the Kotlin `TabToolBridge`.
//!
//! The JavaVM, the bridge class and the documents directory are resolved once
//! and cached. Every call checks, describes and clears a pending Java
//! exception before it is logged, so a failed bridge method can never abort the
//! native app.
//!
//! The bridge encodes its typed results as strings: `OK`, `OK:<value>`,
//! `CANCELLED` or `ERROR:<message>`. Pending file operations return `null` and
//! are polled again later.

mod documents;
mod jni_bridge;
mod text_editor;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use jni::JNIEnv;
use jni::objects::{JObject, JString, JValue};
use log::error;
use tabtool_i18n::{ERROR_GENERIC, LocalizedError};

use crate::bridge_result::{BridgeResult, parse_result};
use crate::locale::{SystemInfo, decimal_separator, normalize_tag};
use crate::services::OpenUrl;
use crate::text_input::TextInputOutcome;
use jni_bridge::{
    call_bool, call_int_array, call_string, call_string_1, call_string_array, call_void,
    log_jni_error, with_env,
};
use text_editor::SESSIONS;

static ERRORS: Mutex<VecDeque<LocalizedError>> = Mutex::new(VecDeque::new());

/// A document another app asked TabTool to open.
///
/// `Staged` names the copied file inside the app's private documents
/// directory and the original display name. The app picks a collision-free
/// final name before it opens the copy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntentImport {
    /// The content was copied and waits to be adopted.
    Staged {
        /// The file name inside the documents directory.
        file_name: String,
        /// The display name reported by the sending app.
        display_name: String,
    },
    /// The copy failed; the string is the diagnostic detail.
    Failed(String),
}

/// The Android platform services.
#[derive(Clone, Copy, Debug, Default)]
pub struct AndroidPlatform;

impl AndroidPlatform {
    /// Closes the activity when the app's navigation stack cannot take back.
    pub fn finish_activity(self) {
        call_void("finishActivity", "()V", &[]);
    }

    /// Whether an import picker started earlier is still open.
    #[must_use]
    pub fn import_pending(self) -> bool {
        call_bool("isImportPending")
    }

    /// Whether an export picker or copy started earlier is still running.
    #[must_use]
    pub fn export_pending(self) -> bool {
        call_bool("isExportPending")
    }

    /// The app's data directory: settings and process-death state live here.
    #[must_use]
    pub fn files_dir(self) -> Option<PathBuf> {
        call_string("filesDir", "()Ljava/lang/String;", &[]).map(PathBuf::from)
    }

    /// Whether the system is in night mode.
    ///
    /// winit answers `None` on Android, so the app asks the activity for the
    /// configuration's night bit instead.
    #[must_use]
    pub fn dark_mode(self) -> bool {
        call_bool("isDarkMode")
    }

    /// The device's UTC offset in minutes and whether it uses a 12-hour clock.
    #[must_use]
    pub fn clock(self) -> Option<(i32, bool)> {
        let values = call_int_array("clock")?;
        let offset = *values.first()?;
        let twelve_hour = values.get(1).is_some_and(|value| *value != 0);
        Some((offset, twelve_hour))
    }

    /// Takes one transition to the background.
    #[must_use]
    pub fn take_background_event(self) -> bool {
        call_bool("takeBackgroundEvent")
    }

    /// Takes one document another app asked TabTool to open.
    ///
    /// The first call after the copy completes reports the staged file; later
    /// calls report `None` until another intent arrives.
    #[must_use]
    pub fn take_intent_import(self) -> Option<IntentImport> {
        let raw = call_string("takeIntentImport", "()Ljava/lang/String;", &[])?;
        match parse_result(&raw) {
            BridgeResult::Ok(Some(value)) => match value.split_once('\n') {
                Some((file_name, display_name)) if !file_name.is_empty() => {
                    Some(IntentImport::Staged {
                        file_name: file_name.to_owned(),
                        display_name: display_name.to_owned(),
                    })
                }
                _ => Some(IntentImport::Failed(format!(
                    "the intent result is malformed: {value}"
                ))),
            },
            BridgeResult::Ok(None) | BridgeResult::Cancelled => None,
            BridgeResult::Failed(message) => Some(IntentImport::Failed(message)),
        }
    }
}

impl AndroidPlatform {
    /// `[left, top, right, bottom]` safe-area insets.
    pub fn safe_area(self) -> [f32; 4] {
        match call_int_array("insets") {
            Some(values) if values.len() >= 5 => [
                values[0] as f32,
                values[1] as f32,
                values[2] as f32,
                values[3] as f32,
            ],
            _ => [0.0; 4],
        }
    }

    /// The soft keyboard height, or zero when it is hidden.
    pub fn keyboard_height(self) -> f32 {
        call_int_array("insets")
            .and_then(|values| values.get(4).copied())
            .map_or(0.0, |height| height as f32)
    }

    /// Opens the system share sheet for `path`.
    pub fn share(self, path: &Path) {
        let path = path.to_string_lossy();
        let Some(raw) = call_string_1("shareFile", &path, "(Ljava/lang/String;)Ljava/lang/String;")
        else {
            return;
        };
        if let BridgeResult::Failed(message) = parse_result(&raw) {
            record_error(ERROR_GENERIC, format!("share: {message}"));
        }
    }

    /// Takes a pending back event, if the system reported one.
    ///
    /// The platform consumes the system gesture and queues it; the app decides
    /// what to close and exits at the root itself.
    pub fn take_back(self) -> bool {
        call_bool("takeBack")
    }

    /// Takes the next failure, if any.
    pub fn take_error(self) -> Option<LocalizedError> {
        ERRORS
            .lock()
            .map(|mut errors| errors.pop_front())
            .unwrap_or_else(|poisoned| poisoned.into_inner().pop_front())
    }
}

/// The text clipboard.
impl AndroidPlatform {
    /// The clipboard text, if there is any.
    #[must_use]
    pub fn clipboard_text(self) -> Option<String> {
        call_string("clipboardGet", "()Ljava/lang/String;", &[])
    }

    /// Replaces the clipboard text.
    pub fn set_clipboard_text(self, text: &str) {
        with_env("clipboardSet", (), |env, bridge| {
            let text = env.new_string(text)?;
            env.call_static_method(
                &bridge.class,
                "clipboardSet",
                "(Ljava/lang/String;)V",
                &[JValue::from(&text)],
            )?;
            Ok(())
        });
    }
}

impl OpenUrl for AndroidPlatform {
    fn open(&self, url: &str) {
        let Some(raw) = call_string_1("openUrl", url, "(Ljava/lang/String;)Ljava/lang/String;")
        else {
            return;
        };
        if let BridgeResult::Failed(message) = parse_result(&raw) {
            record_error(ERROR_GENERIC, format!("open {url}: {message}"));
        }
    }
}

impl SystemInfo for AndroidPlatform {
    fn languages(&self) -> Vec<String> {
        call_string_array("languages").unwrap_or_default()
    }

    fn locale(&self) -> String {
        self.languages()
            .first()
            .map_or_else(|| "en_US".to_owned(), |tag| normalize_tag(tag))
    }

    fn decimal_separator(&self) -> String {
        decimal_separator(&self.locale())
    }
}

/// Queues a failure for the UI and logs its diagnostic detail.
fn record_error(message_key: &'static str, detail: String) {
    error!("Platform error ({message_key}): {detail}");
    let mut errors = match ERRORS.lock() {
        Ok(errors) => errors,
        Err(poisoned) => poisoned.into_inner(),
    };
    if errors.len() >= 8 {
        let _ = errors.pop_front();
    }
    errors.push_back(LocalizedError::new(message_key, detail));
}

// --- JNI callbacks ----------------------------------------------------------

/// JNI callback from `TabToolBridge.nativeTextCommitted`.
///
/// # Safety
///
/// The JVM calls this on the UI thread of the attached app; the `JNIEnv` and
/// `JString` arguments are valid for the duration of the call.
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_shurco_tabtool_TabToolBridge_nativeTextCommitted(
    mut env: JNIEnv<'_>,
    _this: JObject<'_>,
    text: JString<'_>,
) {
    match env.get_string(&text) {
        Ok(text) => SESSIONS.finish(TextInputOutcome::Committed(text.into())),
        Err(error) => {
            log_jni_error(&mut env, "nativeTextCommitted", &error);
            error!("Dropping a text commit that could not be read from the JVM");
        }
    }
}

/// JNI callback from `TabToolBridge.nativeTextCancelled`.
///
/// # Safety
///
/// The JVM calls this on the UI thread of the attached app; both references are
/// valid for the duration of the call.
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_shurco_tabtool_TabToolBridge_nativeTextCancelled(
    _env: JNIEnv<'_>,
    _this: JObject<'_>,
) {
    SESSIONS.finish(TextInputOutcome::Cancelled);
}
