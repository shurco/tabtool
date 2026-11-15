//! The native text editor shown over an app rectangle.

use jni::objects::{JValue, JValueOwned};
use log::error;

use super::AndroidPlatform;
use super::jni_bridge::{call_void, with_env};
use crate::text_input::{EditorSessions, TextInputOutcome, TextInputRequest};

pub(super) static SESSIONS: EditorSessions = EditorSessions::new();

/// The native text editor shown over an app rectangle.
///
/// A session is active from [`AndroidPlatform::open_text_editor`] until exactly
/// one outcome is reported. The platform queues that outcome;
/// [`AndroidPlatform::poll_text_editor`] drains it once, and every later poll
/// returns `None` until the next session. [`AndroidPlatform::close_text_editor`]
/// ends the session and drops an undrained outcome.
impl AndroidPlatform {
    /// Opens an editor over `request.rect`, preloaded with `request.text`.
    ///
    /// A session from an earlier [`AndroidPlatform::open_text_editor`] is
    /// replaced and its undrained outcome is dropped, so a stale commit cannot
    /// surface later.
    pub fn open_text_editor(self, request: TextInputRequest) {
        SESSIONS.reset();
        if !request.is_valid() {
            error!(
                "Refusing an editor request with rect {:?}: the size is not usable",
                request.rect
            );
            return;
        }

        let [left, top, width, height] = request.rect;
        let shown = with_env("showTextEditor", false, |env, bridge| {
            let text = env.new_string(&request.text)?;
            let args = [
                JValue::Float(left),
                JValue::Float(top),
                JValue::Float(width),
                JValue::Float(height),
                JValue::from(&text),
                JValue::Bool(u8::from(request.multiline)),
            ];
            match env.call_static_method(
                &bridge.class,
                "showTextEditor",
                "(FFFFLjava/lang/String;Z)Z",
                &args,
            )? {
                JValueOwned::Bool(value) => Ok(value != 0),
                _ => Err(jni::errors::Error::WrongJValueType("boolean", "other")),
            }
        });
        if shown {
            SESSIONS.activate();
        } else {
            error!("The native editor did not open; no session is active");
        }
    }

    /// Closes the active editor and drops its undrained outcome.
    pub fn close_text_editor(self) {
        SESSIONS.reset();
        call_void("hideTextEditor", "()V", &[]);
    }

    /// Asks the active editor to commit its current text.
    ///
    /// The platform reports the result through
    /// [`AndroidPlatform::poll_text_editor`] on a later call, exactly like a
    /// user commit. Without an active session this does nothing. Closing
    /// instead of committing would discard the typed text.
    pub fn commit_text_editor(self) {
        call_void("commitTextEditor", "()V", &[]);
    }

    /// Moves the active editor over `rect` without resetting its text.
    ///
    /// [`TextInputRequest::rect`] is in points and relative to the app's
    /// content view; the platform converts it to pixels. A keyboard inset or a
    /// scroll that moves the edited cell calls this.
    pub fn move_text_editor(self, rect: [f32; 4]) {
        let [left, top, width, height] = rect;
        if !rect.iter().all(|value| value.is_finite()) || width <= 0.0 || height <= 0.0 {
            return;
        }
        call_void(
            "moveTextEditor",
            "(FFFF)V",
            &[
                JValue::Float(left),
                JValue::Float(top),
                JValue::Float(width),
                JValue::Float(height),
            ],
        );
    }

    /// Takes the session's outcome, if the platform reported one.
    #[must_use]
    pub fn poll_text_editor(self) -> Option<TextInputOutcome> {
        SESSIONS.poll()
    }

    /// Whether a session is open and its outcome has not arrived yet.
    #[must_use]
    pub fn text_editor_active(self) -> bool {
        SESSIONS.is_active()
    }
}
