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
    let answer = if question.contains("coroutine") {
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
