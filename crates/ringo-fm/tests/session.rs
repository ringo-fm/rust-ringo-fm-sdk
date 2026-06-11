//! Session lifecycle tests that exercise the native bridge without requiring
//! model availability (no live inference).

use ringo_fm::LanguageModelSession;

#[test]
fn session_default_is_not_responding() {
    let session = LanguageModelSession::default().expect("create default session");
    assert!(!session.is_responding());
}

#[test]
fn session_new_with_instructions() {
    let session = LanguageModelSession::new(None, Some("You are a helpful assistant."), Vec::new())
        .expect("create session with instructions");
    assert!(!session.is_responding());
}

#[test]
fn session_prewarm_is_safe() {
    let session = LanguageModelSession::default().expect("create default session");

    // Prewarm is a fire-and-forget hint; both forms must be safe and must not
    // flip the session into a responding state.
    session.prewarm(None).expect("prewarm without prefix");
    session
        .prewarm(Some("Summarize the following text:"))
        .expect("prewarm with prefix");
    assert!(!session.is_responding());
}

#[test]
fn session_prewarm_rejects_interior_nul() {
    let session = LanguageModelSession::default().expect("create default session");
    assert!(session.prewarm(Some("bad\0prefix")).is_err());
}

#[test]
fn session_reset_is_safe() {
    let session = LanguageModelSession::default().expect("create default session");
    session.reset();
    assert!(!session.is_responding());
}
