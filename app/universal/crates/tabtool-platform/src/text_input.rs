//! Platform-neutral text editing.
//!
//! Android cannot receive IME text through egui on this stack, so it opens a
//! native editor over the control. Every mobile view talks to [`TextInput`]
//! instead of egui's `TextEdit`, which keeps the platform difference inside
//! this crate; desktop keeps using egui directly.

/// A request to edit `text` over a rectangle in points.
///
/// The rectangle is relative to the app's content view, which is the same
/// origin egui draws from. Platforms that position native views convert the
/// points to their own pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct TextInputRequest {
    /// `[left, top, width, height]` in points.
    pub rect: [f32; 4],
    /// The text the session starts with.
    pub text: String,
    /// Whether the editor accepts line breaks.
    pub multiline: bool,
}

/// The result of one editor session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputOutcome {
    /// The user finished editing and produced this text.
    Committed(String),
    /// The user dismissed the editor without committing.
    Cancelled,
}

/// A native text editor shown over an app rectangle.
///
/// A session is active from [`TextInput::open`] until exactly one outcome is
/// reported. The platform queues that outcome; [`TextInput::poll`] drains it
/// once, and every later poll returns `None` until the next session.
/// [`TextInput::close`] ends the session and drops an undrained outcome.
pub trait TextInput {
    /// Opens an editor over `request.rect`, preloaded with `request.text`.
    ///
    /// A session from an earlier [`TextInput::open`] is replaced and its
    /// undrained outcome is dropped, so a stale commit cannot surface later.
    fn open(&self, request: TextInputRequest);

    /// Closes the active editor and drops its undrained outcome.
    fn close(&self);

    /// Takes the session's outcome, if the platform reported one.
    fn poll(&self) -> Option<TextInputOutcome>;

    /// Whether a session is open and its outcome has not arrived yet.
    fn is_active(&self) -> bool;
}
