//! Native Android text editing overlay.
//!
//! winit 0.30.13's Android backend drops GameActivity `TextEvent`s, so egui
//! never receives soft-keyboard input. The spike uses a real Android `EditText`
//! instead and receives commits/cancels through JNI callbacks.

use std::sync::Mutex;

use jni::JNIEnv;
use jni::objects::{JObject, JString};
use log::error;

use crate::platform_bridge;

/// The outcome of an editor session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputOutcome {
    Committed(String),
    Cancelled,
}

/// A request to edit text in the native overlay.
#[derive(Debug, Clone, PartialEq)]
pub struct TextInputRequest {
    /// `[left, top, width, height]` in physical pixels.
    pub rect: [f32; 4],
    pub text: String,
    pub multiline: bool,
}

static OUTCOME: Mutex<Option<TextInputOutcome>> = Mutex::new(None);

/// Opens the native editor over `request.rect` and preloads `request.text`.
pub fn open(request: TextInputRequest) {
    set_outcome(None);

    #[allow(clippy::cast_possible_truncation)]
    let [left, top, width, height] = request.rect.map(|value| value.round() as i32);
    platform_bridge::show_text_editor(left, top, width, height, &request.text, request.multiline);
}

/// Removes the native editor without reporting a commit or cancel.
///
/// A pending outcome is cleared so a stale commit from an earlier session
/// cannot be delivered after the editor is closed.
pub fn close() {
    set_outcome(None);
    platform_bridge::hide_text_editor();
}

/// Drains the outcome reported by the native editor, if any.
pub fn poll() -> Option<TextInputOutcome> {
    OUTCOME
        .lock()
        .map(|mut slot| slot.take())
        .unwrap_or_else(|poisoned| poisoned.into_inner().take())
}

fn set_outcome(outcome: Option<TextInputOutcome>) {
    match OUTCOME.lock() {
        Ok(mut slot) => *slot = outcome,
        Err(poisoned) => *poisoned.into_inner() = outcome,
    }
}

/// JNI callback from `TabToolTextEditor.nativeTextCommitted`.
///
/// # Safety
///
/// The JVM calls this on the UI thread of the attached app; the `JNIEnv` and
/// `JString` arguments are valid for the duration of the call.
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_shurco_tabtool_TabToolTextEditor_nativeTextCommitted(
    mut env: JNIEnv<'_>,
    _this: JObject<'_>,
    text: JString<'_>,
) {
    match env.get_string(&text) {
        Ok(text) => set_outcome(Some(TextInputOutcome::Committed(text.into()))),
        Err(error) => {
            platform_bridge::log_jni_error(&mut env, "nativeTextCommitted", &error);
            error!("Dropping a text commit that could not be read from the JVM");
        }
    }
}

/// JNI callback from `TabToolTextEditor.nativeTextCancelled`.
///
/// # Safety
///
/// The JVM calls this on the UI thread of the attached app; both references are
/// valid for the duration of the call.
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_shurco_tabtool_TabToolTextEditor_nativeTextCancelled(
    _env: JNIEnv<'_>,
    _this: JObject<'_>,
) {
    set_outcome(Some(TextInputOutcome::Cancelled));
}
