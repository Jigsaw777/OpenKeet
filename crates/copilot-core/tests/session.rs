use copilot_core::{
    AnswerFailure, AnswerMode, AnswerStatus, CaptureState, Policy, QuestionPhase, Session, Source,
    Span, Stability, Utterance, rehearsal,
};

fn setup(policy: Policy) -> Session {
    let mut session = Session::new(17, Source::REPLAY, policy);
    session.set_capture(CaptureState::Capturing, 0);
    session
}

fn transcript(
    id: u64,
    revision: u64,
    start: u64,
    end: u64,
    stability: Stability,
    text: &str,
) -> Utterance {
    Utterance {
        id,
        revision,
        span: Span {
            start_ms: start,
            end_ms: end,
        },
        stability,
        text: text.to_owned(),
    }
}

fn final_text(session: &mut Session, id: u64, start: u64, end: u64, text: &str) {
    session
        .ingest(transcript(id, 1, start, end, Stability::Final, text))
        .unwrap();
}

#[test]
fn late_constraints_consolidate_across_the_supported_short_pause_range() {
    let fixtures = [
        (
            "How would you protect shared state",
            "without blocking the coroutine thread?",
        ),
        (
            "Explain an inner join",
            "but select only certain columns from both tables.",
        ),
        (
            "What is an API latency bottleneck?",
            "specifically in PostgreSQL joins and caching.",
        ),
    ];
    for gap in (500..=3_000).step_by(100) {
        for (head, tail) in fixtures {
            let mut session = setup(Policy::default());
            final_text(&mut session, 1, 100, 800, head);
            session.observe_speech(2, Span::at(800 + gap)).unwrap();
            assert!(
                session.next_request(5_000).is_none(),
                "wait for the delayed final at gap {gap}"
            );
            final_text(&mut session, 2, 800 + gap, 1_200 + gap, tail);
            rehearsal::answer_available(&mut session, 5_000 + gap);
            let snapshot = session.snapshot();
            assert_eq!(snapshot.history.len(), 1);
            assert_eq!(snapshot.history[0].question, format!("{head} {tail}"));
            assert_eq!(snapshot.history[0].answer.key.revision, 2);
        }
    }
}

#[test]
fn a_distinct_question_after_a_short_pause_gets_its_own_answer() {
    let mut session = setup(Policy::default());
    final_text(&mut session, 1, 100, 800, "Why use a mutex?");
    final_text(&mut session, 2, 1_300, 1_800, "Explain an inner join.");
    rehearsal::answer_available(&mut session, 6_000);
    let history = session.snapshot().history;
    assert_eq!(history.len(), 2);
    assert_ne!(
        history[0].answer.key.question_id,
        history[1].answer.key.question_id
    );
}

#[test]
fn correction_revises_one_checked_entry_and_keeps_the_literal_transcript() {
    let mut session = setup(Policy::default());
    final_text(&mut session, 1, 100, 800, "When should I use a Semaphore?");
    rehearsal::answer_available(&mut session, 4_000);
    final_text(
        &mut session,
        2,
        4_500,
        5_300,
        "Actually, I meant a Mutex, not a Semaphore.",
    );
    let before = session.snapshot();
    assert_eq!(
        before.question.unwrap().answer.unwrap().status,
        AnswerStatus::Superseded
    );
    assert_eq!(before.history[0].answer.key.revision, 1);
    rehearsal::answer_available(&mut session, 9_000);
    let after = session.snapshot();
    assert_eq!(after.history.len(), 1);
    assert_eq!(after.history[0].answer.key.revision, 2);
    assert!(after.history[0].answer.text.contains("A mutex protects"));
    assert!(
        session
            .full_transcript()
            .unwrap()
            .contains("\nActually, I meant a Mutex")
    );
}

#[test]
fn filler_is_transcribed_but_never_queued_for_an_answer() {
    let mut session = setup(Policy::default());
    final_text(
        &mut session,
        1,
        100,
        800,
        "Thanks for joining us this morning.",
    );
    assert!(session.next_request(10_000).is_none());
    assert!(session.snapshot().question.is_none());
    assert_eq!(session.snapshot().transcript.len(), 1);
}

#[test]
fn unfinished_clause_can_continue_after_nearly_five_seconds() {
    let mut session = setup(Policy::default());
    final_text(
        &mut session,
        1,
        100,
        800,
        "Can you explain why cache invalidation is hard when",
    );
    assert_eq!(
        session.snapshot().question.unwrap().phase,
        QuestionPhase::Incomplete
    );
    assert!(session.next_request(5_000).is_none());
    final_text(
        &mut session,
        2,
        5_700,
        6_000,
        "multiple processes share the same keys?",
    );
    rehearsal::answer_available(&mut session, 10_000);
    assert_eq!(session.snapshot().history[0].answer.key.revision, 2);
}

#[test]
fn interim_revisions_and_duplicate_finals_do_not_duplicate_transcript() {
    let mut session = setup(Policy::default());
    let partial = transcript(1, 1, 100, 400, Stability::Interim, "What is a cash");
    session.ingest(partial.clone()).unwrap();
    let sequence = session.snapshot().sequence;
    session.ingest(partial.clone()).unwrap();
    assert_eq!(session.snapshot().sequence, sequence);
    let final_row = transcript(1, 2, 100, 700, Stability::Final, "What is a cache?");
    session.ingest(final_row.clone()).unwrap();
    session.ingest(partial).unwrap();
    session.ingest(final_row).unwrap();
    assert_eq!(session.full_transcript().unwrap(), "What is a cache?");
}

#[test]
fn checked_result_waits_for_speech_then_is_rejected_on_revision() {
    let mut session = setup(Policy::default());
    final_text(&mut session, 1, 100, 800, "Explain an inner join.");
    let old = session.next_request(4_000).unwrap();
    session.observe_speech(2, Span::at(4_100)).unwrap();
    assert!(session.complete(old.request.key, 4_300, Ok("Unrevised answer".into())));
    assert!(session.snapshot().history.is_empty());
    final_text(
        &mut session,
        2,
        4_100,
        4_900,
        "but select only certain columns.",
    );
    assert!(old.is_cancelled());
    assert!(!session.append_answer(old.request.key, "late token"));
    assert!(!session.complete(old.request.key, 5_000, Ok("late answer".into())));
    rehearsal::answer_available(&mut session, 8_200);
    assert_eq!(session.snapshot().history[0].answer.key.revision, 2);
}

#[test]
fn superseded_draft_cannot_write_into_checked_request_for_same_revision() {
    let mut session = setup(Policy {
        draft_after_ms: Some(500),
        ..Policy::default()
    });
    final_text(&mut session, 1, 100, 800, "Explain an inner join.");
    let draft = session.next_request(1_300).unwrap();
    assert_eq!(draft.request.key.mode, AnswerMode::Draft);
    assert!(session.append_answer(draft.request.key, "Draft token"));
    let checked = session.next_request(4_000).unwrap();
    assert!(draft.is_cancelled());
    assert_eq!(checked.request.key.mode, AnswerMode::Checked);
    assert!(!session.complete(draft.request.key, 4_001, Ok("stale".into())));
    assert!(session.complete(checked.request.key, 4_002, Ok("verified".into())));
    assert_eq!(session.snapshot().history[0].answer.text, "verified");
}

#[test]
fn timeout_and_cancel_clear_busy_state_and_reject_late_results() {
    let mut session = setup(Policy {
        answer_timeout_ms: 100,
        ..Policy::default()
    });
    session.ask("Explain SQL", 100).unwrap();
    let old = session.next_request(100).unwrap();
    assert!(session.next_request(200).is_none());
    assert!(old.is_cancelled());
    assert_eq!(
        session.snapshot().question.unwrap().phase,
        QuestionPhase::Error
    );
    assert_eq!(
        session.snapshot().notice.as_deref(),
        Some(AnswerFailure::TimedOut.message())
    );
    session.ask("Explain Java", 300).unwrap();
    let new = session.next_request(300).unwrap();
    session.cancel_answer();
    assert!(new.is_cancelled());
    assert!(!session.complete(new.request.key, 320, Ok("late".into())));
    assert_eq!(
        session.snapshot().question.unwrap().phase,
        QuestionPhase::Cancelled
    );
    assert!(session.snapshot().history.is_empty());
}

#[test]
fn gap_cancels_uncertain_audio_work_and_allows_new_questions() {
    let mut session = setup(Policy::default());
    final_text(&mut session, 1, 100, 800, "Explain an inner join.");
    let work = session.next_request(4_000).unwrap();
    session.observe_speech(2, Span::at(4_050)).unwrap();
    session.audio_gap(
        Span {
            start_ms: 4_100,
            end_ms: 4_130,
        },
        30,
        "Device disconnected.",
    );
    assert!(work.is_cancelled());
    assert_eq!(session.snapshot().capture, CaptureState::Recovery);
    assert_eq!(session.snapshot().dropped_ms, 30);
    final_text(&mut session, 2, 4_100, 4_200, "without blocking.");
    session.set_capture(CaptureState::Capturing, 5_000);
    final_text(&mut session, 3, 5_100, 5_700, "Why use a mutex?");
    rehearsal::answer_available(&mut session, 10_000);
    let snapshot = session.snapshot();
    assert_eq!(snapshot.history.len(), 1);
    assert_eq!(snapshot.history[0].question, "Why use a mutex?");
    let events: Vec<_> = session.drain_events().collect();
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].sequence + 1 == pair[1].sequence)
    );
    assert!(
        events
            .iter()
            .all(|event| event.session_id == snapshot.session_id && event.protocol_version == 1)
    );
}

#[test]
fn full_transcript_and_manual_question_support_legacy_length() {
    let mut session = setup(Policy::default());
    let body = "Practice context. ".repeat(200);
    final_text(&mut session, 1, 100, 800, &body);
    final_text(&mut session, 2, 1_500, 2_400, &body);
    let full = session.full_transcript().unwrap();
    assert!(full.len() > 4_000);
    session.ask(&full, 3_000).unwrap();
    let work = session.next_request(3_000).unwrap();
    assert_eq!(work.request.question, full);
    assert_eq!(work.request.source, Source::TYPED);
}
