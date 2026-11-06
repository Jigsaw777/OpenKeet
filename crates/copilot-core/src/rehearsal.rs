//! Synthetic text and practice responses. This path requires no model, audio permission, or network.

use serde::{Deserialize, Serialize};

use crate::{AnswerRequest, Session, Span, Stability, Utterance};

pub const DEMO_JSON: &str = include_str!("../../../tests/fixtures/rehearsal.json");

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Step {
    pub at_ms: u64,
    pub id: u64,
    pub revision: u64,
    pub start_ms: u64,
    pub end_ms: u64,
    pub stability: Stability,
    pub text: String,
}

impl Step {
    pub fn apply(self, session: &mut Session) -> Result<(), &'static str> {
        if self.end_ms > self.at_ms {
            return Err("Transcript arrives before the end of its capture span.");
        }
        session.ingest(Utterance {
            id: self.id,
            revision: self.revision,
            span: Span {
                start_ms: self.start_ms,
                end_ms: self.end_ms,
            },
            stability: self.stability,
            text: self.text,
        })
    }
}

pub fn practice_answer(request: &AnswerRequest) -> String {
    let question = request.question.to_lowercase();
    let previous = request
        .follow_up
        .as_ref()
        .map(|prior| prior.question.to_lowercase());
    let answer = if previous
        .as_deref()
        .is_some_and(|prior| prior.contains("coroutine"))
        && question.contains("block")
    {
        "Keep the API call outside the mutex. If the client exposes a suspending API, await it directly. If it only exposes a blocking API, run that call on `Dispatchers.IO`.\n\n\
        1. Take a snapshot of the state and its version while holding the mutex, then release it.\n\
        2. Make the API call with a client timeout. Use a cancellable client, or `runInterruptible` when the blocking API honors thread interruption. Moving the call to `Dispatchers.IO` alone does not make the call interruptible.\n\
        3. Reacquire the mutex and recheck the version before applying the result. Decide whether to retry or discard a stale result.\n\n\
        Test that a slow or cancelled request leaves other coroutines free to read and update the shared state."
    } else if previous
        .as_deref()
        .is_some_and(|prior| prior.contains("join"))
        && question.contains("no match")
    {
        "Use a left join when the left-side rows must survive. Unmatched right-side columns are `NULL`. Put filters on the optional right-side rows in the `ON` clause when unmatched left-side rows should remain. A right-side filter in `WHERE` can remove them."
    } else if question.contains("coroutine") {
        "Use a suspending mutex around the smallest update to shared state. A waiting coroutine suspends, leaving the thread free to run other work. Keep I/O outside the lock, and release it with a scoped operation such as Kotlin’s `withLock`."
    } else if question.contains("join") {
        "Start with the join keys and which rows must survive. An inner join returns matching rows from both inputs. Use table aliases and explicitly select the requested columns. For performance, inspect the query plan and check indexes on the join and filter keys."
    } else if question.contains("mutex") {
        "A mutex protects a critical section with exclusive ownership. Use a scoped lock, keep the protected update short, and choose a suspending lock if the caller is a coroutine."
    } else {
        "Practice outline: state the assumption that matters most, explain your approach with one concrete example, then describe the main trade-off and how you would verify it."
    };
    answer.to_owned()
}

pub fn answer_available(session: &mut Session, now_ms: u64) {
    while let Some(work) = session.next_request(now_ms) {
        let answer = practice_answer(&work.request);
        session.complete(work.request.key, now_ms, Ok(answer));
    }
}
