//! The neutral [`TextInput`] contract, exercised through a fake.
//!
//! The mobile views only ever see the trait. This fake pins the behavior they
//! can rely on: one outcome per session, `poll` drains it once, `close` clears
//! the session, and `is_active` is true only while an outcome is still due.

use std::cell::RefCell;

use tabtool_platform::{TextInput, TextInputOutcome, TextInputRequest};

#[derive(Default)]
struct FakeTextInput {
    state: RefCell<FakeState>,
}

#[derive(Default)]
struct FakeState {
    request: Option<TextInputRequest>,
    outcome: Option<TextInputOutcome>,
}

impl FakeTextInput {
    fn finish(&self, outcome: TextInputOutcome) {
        let mut state = self.state.borrow_mut();
        assert!(state.request.is_some(), "no session to finish");
        state.request = None;
        state.outcome = Some(outcome);
    }

    fn request(&self) -> Option<TextInputRequest> {
        self.state.borrow().request.clone()
    }
}

impl TextInput for FakeTextInput {
    fn open(&self, request: TextInputRequest) {
        let mut state = self.state.borrow_mut();
        state.request = Some(request);
        state.outcome = None;
    }

    fn close(&self) {
        let mut state = self.state.borrow_mut();
        state.request = None;
        state.outcome = None;
    }

    fn poll(&self) -> Option<TextInputOutcome> {
        self.state.borrow_mut().outcome.take()
    }

    fn is_active(&self) -> bool {
        let state = self.state.borrow();
        state.request.is_some() && state.outcome.is_none()
    }
}

fn request(text: &str) -> TextInputRequest {
    TextInputRequest {
        rect: [10.0, 20.0, 200.0, 44.0],
        text: text.to_owned(),
        multiline: false,
    }
}

#[test]
fn opening_a_session_keeps_the_request_and_reports_active() {
    let input = FakeTextInput::default();
    assert!(!input.is_active());

    input.open(request("cell"));

    assert!(input.is_active());
    assert_eq!(input.request().as_ref(), Some(&request("cell")));
    assert_eq!(input.poll(), None);
}

#[test]
fn a_commit_is_delivered_once() {
    let input = FakeTextInput::default();
    input.open(request("old"));

    input.finish(TextInputOutcome::Committed("new".to_owned()));

    assert!(!input.is_active());
    assert_eq!(
        input.poll(),
        Some(TextInputOutcome::Committed("new".to_owned()))
    );
    assert_eq!(input.poll(), None);
}

#[test]
fn a_cancel_is_delivered_once() {
    let input = FakeTextInput::default();
    input.open(request("old"));

    input.finish(TextInputOutcome::Cancelled);

    assert_eq!(input.poll(), Some(TextInputOutcome::Cancelled));
    assert_eq!(input.poll(), None);
}

#[test]
fn closing_a_session_drops_its_outcome() {
    let input = FakeTextInput::default();
    input.open(request("old"));
    input.finish(TextInputOutcome::Committed("typed".to_owned()));

    input.close();

    assert!(!input.is_active());
    assert_eq!(input.poll(), None);
}

#[test]
fn opening_again_drops_the_previous_outcome() {
    let input = FakeTextInput::default();
    input.open(request("first"));
    input.finish(TextInputOutcome::Committed("untaken".to_owned()));

    input.open(request("second"));

    assert!(input.is_active());
    assert_eq!(input.poll(), None);
}

#[test]
fn the_next_session_delivers_its_own_outcome() {
    let input = FakeTextInput::default();
    input.open(request("first"));
    input.finish(TextInputOutcome::Cancelled);
    input.open(request("second"));

    input.finish(TextInputOutcome::Committed("taken".to_owned()));

    assert_eq!(
        input.poll(),
        Some(TextInputOutcome::Committed("taken".to_owned()))
    );
    assert_eq!(input.poll(), None);
}
