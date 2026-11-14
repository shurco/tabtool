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

impl TextInputRequest {
    /// Whether the rectangle can hold an editor.
    ///
    /// A zero, negative or non-finite size would make the native editor refuse
    /// to open, so the platform checks this before starting a session.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let [left, top, width, height] = self.rect;

        [left, top, width, height]
            .iter()
            .all(|value| value.is_finite())
            && width > 0.0
            && height > 0.0
    }
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

/// The session state shared by a native editor.
///
/// The platform reports commits and cancels from its UI thread while the app
/// polls from its own thread, so the outcome is a mutex and the active flag is
/// atomic. A failed open leaves the state inactive instead of waiting forever.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
#[derive(Debug)]
pub(crate) struct EditorSessions {
    outcome: std::sync::Mutex<Option<TextInputOutcome>>,
    active: std::sync::atomic::AtomicBool,
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
impl EditorSessions {
    pub(crate) const fn new() -> Self {
        Self {
            outcome: std::sync::Mutex::new(None),
            active: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Clears an undrained outcome and marks no session active.
    pub(crate) fn reset(&self) {
        self.set_outcome(None);
        self.active
            .store(false, std::sync::atomic::Ordering::Release);
    }

    /// Marks the session active after the native editor opened.
    pub(crate) fn activate(&self) {
        self.active
            .store(true, std::sync::atomic::Ordering::Release);
    }

    /// Records the one outcome of the session and ends it.
    pub(crate) fn finish(&self, outcome: TextInputOutcome) {
        self.set_outcome(Some(outcome));
        self.active
            .store(false, std::sync::atomic::Ordering::Release);
    }

    pub(crate) fn poll(&self) -> Option<TextInputOutcome> {
        self.outcome
            .lock()
            .map(|mut slot| slot.take())
            .unwrap_or_else(|poisoned| poisoned.into_inner().take())
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Acquire)
    }

    fn set_outcome(&self, outcome: Option<TextInputOutcome>) {
        match self.outcome.lock() {
            Ok(mut slot) => *slot = outcome,
            Err(poisoned) => *poisoned.into_inner() = outcome,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorSessions, TextInputOutcome, TextInputRequest};

    fn request() -> TextInputRequest {
        TextInputRequest {
            rect: [0.0, 0.0, 200.0, 44.0],
            text: "cell".to_owned(),
            multiline: false,
        }
    }

    #[test]
    fn a_new_session_is_inactive_and_has_no_outcome() {
        let sessions = EditorSessions::new();

        assert!(!sessions.is_active());
        assert_eq!(sessions.poll(), None);
    }

    #[test]
    fn an_open_session_is_active_until_one_outcome_arrives() {
        let sessions = EditorSessions::new();
        sessions.activate();
        assert!(sessions.is_active());

        sessions.finish(TextInputOutcome::Committed("new".to_owned()));

        assert!(!sessions.is_active());
        assert_eq!(
            sessions.poll(),
            Some(TextInputOutcome::Committed("new".to_owned()))
        );
        assert_eq!(sessions.poll(), None);
    }

    #[test]
    fn a_cancel_ends_the_session() {
        let sessions = EditorSessions::new();
        sessions.activate();

        sessions.finish(TextInputOutcome::Cancelled);

        assert!(!sessions.is_active());
        assert_eq!(sessions.poll(), Some(TextInputOutcome::Cancelled));
    }

    #[test]
    fn reset_drops_the_outcome_and_the_active_flag() {
        let sessions = EditorSessions::new();
        sessions.activate();
        sessions.finish(TextInputOutcome::Committed("untaken".to_owned()));

        sessions.reset();

        assert!(!sessions.is_active());
        assert_eq!(sessions.poll(), None);
    }

    #[test]
    fn a_failed_open_stays_inactive_without_an_outcome() {
        let sessions = EditorSessions::new();

        sessions.reset();
        assert!(!sessions.is_active());
        assert_eq!(sessions.poll(), None);
    }

    #[test]
    fn a_usable_rect_needs_a_positive_finite_size() {
        assert!(request().is_valid());

        let mut invalid = request();
        invalid.rect[2] = 0.0;
        assert!(!invalid.is_valid());

        invalid.rect[2] = 200.0;
        invalid.rect[3] = -1.0;
        assert!(!invalid.is_valid());

        invalid.rect[3] = f32::NAN;
        assert!(!invalid.is_valid());

        invalid.rect[3] = f32::INFINITY;
        assert!(!invalid.is_valid());
    }
}
