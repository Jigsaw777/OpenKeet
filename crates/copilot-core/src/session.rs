use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{
    boundary::{self, Relation},
    protocol::*,
};

const MAX_OPEN_UTTERANCES: usize = 12;
const MAX_TRANSCRIPT_ROWS: usize = 256;
const MAX_QUESTIONS: usize = 16;
const MAX_HISTORY: usize = 32;
const MAX_EVENTS: usize = 128;

#[derive(Clone, Copy, Debug)]
pub struct Policy {
    pub debounce_ms: u64,
    pub continuation_ms: u64,
    pub draft_after_ms: Option<u64>,
    pub answer_timeout_ms: u64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            debounce_ms: 3_200,
            continuation_ms: 5_000,
            draft_after_ms: None,
            answer_timeout_ms: 15_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Work {
    pub request: AnswerRequest,
    cancelled: Arc<AtomicBool>,
}

impl Work {
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

struct Pending {
    work: Work,
    deadline_ms: u64,
    completed: Option<String>,
}

struct Speech {
    span: Span,
    interim: Option<Utterance>,
}

struct Question {
    view: QuestionView,
    closed: bool,
    drafted: bool,
}

pub struct Session {
    id: u64,
    source: Source,
    policy: Policy,
    sequence: u64,
    next_question: u64,
    capture: CaptureState,
    last_final: u64,
    open: BTreeMap<u64, Speech>,
    transcript: VecDeque<Utterance>,
    transcript_truncated: bool,
    questions: VecDeque<Question>,
    history: VecDeque<HistoryItem>,
    events: VecDeque<Event>,
    pending: Option<Pending>,
    dropped_ms: u64,
    notice: Option<String>,
}

impl Session {
    pub fn new(id: u64, source: Source, policy: Policy) -> Self {
        Self {
            id,
            source,
            policy,
            sequence: 0,
            next_question: 1,
            capture: CaptureState::Idle,
            last_final: 0,
            open: BTreeMap::new(),
            transcript: VecDeque::new(),
            transcript_truncated: false,
            questions: VecDeque::new(),
            history: VecDeque::new(),
            events: VecDeque::new(),
            pending: None,
            dropped_ms: 0,
            notice: None,
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            protocol_version: PROTOCOL_VERSION,
            sequence: self.sequence,
            session_id: self.id,
            source: self.source.clone(),
            capture: self.capture,
            speech_active: !self.open.is_empty(),
            transcript: self
                .transcript
                .iter()
                .chain(self.open.values().filter_map(|s| s.interim.as_ref()))
                .cloned()
                .collect(),
            transcript_truncated: self.transcript_truncated,
            question: self.questions.back().map(|q| q.view.clone()),
            history: self.history.iter().cloned().collect(),
            dropped_ms: self.dropped_ms,
            notice: self.notice.clone(),
        }
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = Event> + '_ {
        self.events.drain(..)
    }

    pub fn set_capture(&mut self, state: CaptureState, now_ms: u64) {
        if state == self.capture {
            return;
        }
        self.capture = state;
        if state == CaptureState::Capturing {
            self.notice = None;
        }
        self.record(
            self.source.clone(),
            Span::at(now_ms),
            None,
            EventKind::CaptureChanged { state },
        );
    }

    pub fn observe_speech(&mut self, id: u64, span: Span) -> Result<(), &'static str> {
        if span.start_ms > span.end_ms {
            return Err("Invalid capture timestamps.");
        }
        if id <= self.last_final {
            return Ok(());
        }
        if let Some(speech) = self.open.get_mut(&id) {
            speech.span.end_ms = speech.span.end_ms.max(span.end_ms);
            return Ok(());
        }
        if self.open.len() == MAX_OPEN_UTTERANCES {
            return Err("Transcription backlog is full.");
        }
        self.open.insert(
            id,
            Speech {
                span,
                interim: None,
            },
        );
        self.record(
            self.source.clone(),
            span,
            None,
            EventKind::SpeechActive { utterance_id: id },
        );
        Ok(())
    }

    pub fn ingest(&mut self, mut utterance: Utterance) -> Result<(), &'static str> {
        let id = utterance.id;
        if id <= self.last_final {
            return Ok(());
        }
        if utterance.text.len() > MAX_UTTERANCE_BYTES || utterance.revision == 0 {
            return Err("Invalid transcript length or revision.");
        }
        if self
            .open
            .get(&id)
            .and_then(|s| s.interim.as_ref())
            .is_some_and(|last| last.revision >= utterance.revision)
        {
            return Ok(());
        }
        if utterance.stability == Stability::Final
            && self.open.keys().next().is_some_and(|old| *old < id)
        {
            return Err("Final transcript arrived out of order.");
        }
        let span = utterance.span;
        self.observe_speech(id, span)?;
        let stability = utterance.stability;
        utterance.text = utterance.text.trim().to_owned();
        let question = if stability == Stability::Final {
            self.open.remove(&id);
            self.last_final = id;
            if utterance.text.is_empty() {
                Ok(None)
            } else {
                let question = self.group(&utterance);
                if self.transcript.len() == MAX_TRANSCRIPT_ROWS {
                    self.transcript.pop_front();
                    self.transcript_truncated = true;
                }
                self.transcript.push_back(utterance);
                question
            }
        } else {
            self.open
                .get_mut(&id)
                .expect("speech was registered")
                .interim = Some(utterance);
            Ok(None)
        };
        self.record(
            self.source.clone(),
            span,
            question.as_ref().ok().copied().flatten(),
            EventKind::TranscriptChanged {
                utterance_id: id,
                stability,
            },
        );
        question.map(|_| ())
    }

    pub fn audio_gap(&mut self, span: Span, dropped_ms: u64, notice: impl Into<String>) {
        self.discard_interim();
        for q in &mut self.questions {
            if q.view.source.kind != SourceKind::Typed {
                q.closed = true;
            }
        }
        let affected: Vec<u64> = self
            .questions
            .iter()
            .filter(|q| {
                q.view.source.kind != SourceKind::Typed
                    && !matches!(
                        q.view.phase,
                        QuestionPhase::Checked | QuestionPhase::Cancelled | QuestionPhase::Error
                    )
            })
            .map(|q| q.view.id)
            .collect();
        for id in affected {
            self.invalidate(id, QuestionPhase::Cancelled);
        }
        self.note_audio_gap(span, dropped_ms, notice);
        self.set_capture(CaptureState::Recovery, span.end_ms);
    }

    pub fn note_audio_gap(&mut self, span: Span, dropped_ms: u64, notice: impl Into<String>) {
        self.dropped_ms = self.dropped_ms.saturating_add(dropped_ms);
        self.notice = Some(notice.into());
        self.record(
            self.source.clone(),
            span,
            None,
            EventKind::AudioGap { dropped_ms },
        );
    }

    pub fn discard_interim(&mut self) {
        self.last_final = self
            .last_final
            .max(self.open.keys().next_back().copied().unwrap_or(0));
        self.open.clear();
    }

    pub fn full_transcript(&self) -> Result<String, &'static str> {
        if self.transcript_truncated {
            return Err("Older transcript rows are outside the in-memory session limit.");
        }
        let text = self
            .transcript
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if text.len() > MAX_QUESTION_BYTES {
            return Err("Full transcript is too long. Select a shorter excerpt.");
        }
        Ok(text)
    }

    pub fn ask(&mut self, text: &str, now_ms: u64) -> Result<(), &'static str> {
        let text = text.trim();
        if text.is_empty() || text.len() > MAX_QUESTION_BYTES {
            return Err("Enter a question within the length limit.");
        }
        self.add_question(text, Source::TYPED, Span::at(now_ms), true)?;
        Ok(())
    }

    pub fn next_request(&mut self, now_ms: u64) -> Option<Work> {
        self.settle();
        if let Some(pending) = &self.pending {
            let key = pending.work.request.key;
            let expired = pending.completed.is_none() && now_ms >= pending.deadline_ms;
            if expired {
                self.fail(key.question_id, AnswerFailure::TimedOut);
            } else if key.mode == AnswerMode::Draft && self.ready(key.question_id, now_ms) {
                self.invalidate(key.question_id, QuestionPhase::Ready);
            }
        }
        if self.pending.is_some() {
            return None;
        }

        let eligible = |q: &Question| {
            matches!(
                q.view.phase,
                QuestionPhase::Candidate | QuestionPhase::Waiting | QuestionPhase::Ready
            )
        };
        let ready = self
            .questions
            .iter()
            .filter(|q| eligible(q) && self.ready(q.view.id, now_ms))
            .min_by_key(|q| (q.view.source.kind != SourceKind::Typed, q.view.id));
        let (id, mode) = if let Some(q) = ready {
            (q.view.id, AnswerMode::Checked)
        } else {
            let after = self.policy.draft_after_ms?;
            let q = self.questions.back()?;
            if !eligible(q)
                || q.drafted
                || self.speech_pending(q)
                || now_ms.saturating_sub(q.view.span.end_ms) < after
            {
                return None;
            }
            (q.view.id, AnswerMode::Draft)
        };
        let session_id = self.id;
        let q = self.question_mut(id).expect("selected a retained question");
        let key = AnswerKey {
            session_id,
            question_id: id,
            revision: q.view.revision,
            mode,
        };
        let work = Work {
            request: AnswerRequest {
                key,
                question: q.view.text.clone(),
                source: q.view.source.clone(),
                span: q.view.span,
            },
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        if mode == AnswerMode::Draft {
            q.drafted = true;
        }
        let phase = if mode == AnswerMode::Draft {
            QuestionPhase::Drafting
        } else {
            QuestionPhase::Answering
        };
        self.phase(id, phase);
        self.pending = Some(Pending {
            work: work.clone(),
            deadline_ms: now_ms.saturating_add(self.policy.answer_timeout_ms),
            completed: None,
        });
        Some(work)
    }

    pub fn append_answer(&mut self, key: AnswerKey, delta: &str) -> bool {
        if !self.accepts(key) {
            return false;
        }
        let q = self
            .question_mut(key.question_id)
            .expect("accepted a retained question");
        let answer = q.view.answer.get_or_insert_with(|| Answer {
            key,
            status: AnswerStatus::Provisional,
            text: String::new(),
        });
        if answer.key != key {
            *answer = Answer {
                key,
                status: AnswerStatus::Provisional,
                text: String::new(),
            };
        }
        if delta.len() > MAX_ANSWER_BYTES.saturating_sub(answer.text.len()) {
            self.fail(key.question_id, AnswerFailure::InvalidOutput);
            return false;
        }
        answer.text.push_str(delta);
        self.question_event(
            key.question_id,
            EventKind::AnswerChanged {
                status: AnswerStatus::Provisional,
            },
        );
        true
    }

    pub fn complete(
        &mut self,
        key: AnswerKey,
        now_ms: u64,
        result: Result<String, AnswerFailure>,
    ) -> bool {
        if !self.accepts(key) {
            return false;
        }
        let pending = self.pending.as_mut().expect("accepted pending work");
        if now_ms >= pending.deadline_ms {
            self.fail(key.question_id, AnswerFailure::TimedOut);
        } else {
            match result {
                Ok(text) if !text.trim().is_empty() && text.len() <= MAX_ANSWER_BYTES => {
                    pending.completed = Some(text);
                    self.settle();
                }
                Ok(_) => self.fail(key.question_id, AnswerFailure::InvalidOutput),
                Err(error) => self.fail(key.question_id, error),
            }
        }
        true
    }

    pub fn cancel_answer(&mut self) {
        if let Some(pending) = &self.pending {
            self.invalidate(
                pending.work.request.key.question_id,
                QuestionPhase::Cancelled,
            );
        }
    }

    fn group(&mut self, utterance: &Utterance) -> Result<Option<(u64, u64)>, &'static str> {
        let previous = self
            .questions
            .iter()
            .rev()
            .find(|q| !q.closed && q.view.source == self.source);
        let timing = previous.map(|q| {
            (
                q.view.text.as_str(),
                utterance.span.start_ms.saturating_sub(q.view.span.end_ms),
            )
        });
        match boundary::relation(timing, &utterance.text, self.policy.continuation_ms) {
            Relation::Ignore => Ok(None),
            Relation::New => {
                let prior = previous.map(|q| q.view.id);
                let id =
                    self.add_question(&utterance.text, self.source.clone(), utterance.span, false)?;
                if let Some(q) = prior.and_then(|prior_id| self.question_mut(prior_id)) {
                    q.closed = true;
                }
                Ok(Some((id, 1)))
            }
            Relation::Continue => {
                let id = previous
                    .expect("continuations require a preceding question")
                    .view
                    .id;
                let q = self
                    .question_mut(id)
                    .expect("preceding question was retained");
                if q.view.text.len() + 1 + utterance.text.len() > MAX_QUESTION_BYTES {
                    return Err("Question exceeded the length limit.");
                }
                q.view.text.push(' ');
                q.view.text.push_str(&utterance.text);
                q.view.span.end_ms = q.view.span.end_ms.max(utterance.span.end_ms);
                q.view.revision += 1;
                let revision = q.view.revision;
                let phase = if boundary::incomplete(&q.view.text) {
                    QuestionPhase::Incomplete
                } else {
                    QuestionPhase::Waiting
                };
                if let Some(answer) = &mut q.view.answer {
                    answer.status = AnswerStatus::Superseded;
                }
                self.invalidate(id, phase);
                Ok(Some((id, revision)))
            }
        }
    }

    fn add_question(
        &mut self,
        text: &str,
        source: Source,
        span: Span,
        manual: bool,
    ) -> Result<u64, &'static str> {
        if self.questions.len() == MAX_QUESTIONS {
            let Some(index) = self.questions.iter().position(|q| {
                matches!(
                    q.view.phase,
                    QuestionPhase::Checked
                        | QuestionPhase::Incomplete
                        | QuestionPhase::Cancelled
                        | QuestionPhase::Error
                )
            }) else {
                return Err(
                    "Too many unanswered questions. Cancel the pending answer or wait for it to finish.",
                );
            };
            self.questions.remove(index);
        }
        let id = self.next_question;
        self.next_question += 1;
        let phase = if manual {
            QuestionPhase::Ready
        } else if boundary::incomplete(text) {
            QuestionPhase::Incomplete
        } else {
            QuestionPhase::Candidate
        };
        self.questions.push_back(Question {
            closed: manual,
            drafted: false,
            view: QuestionView {
                id,
                revision: 1,
                phase,
                span,
                source,
                text: text.to_owned(),
                answer: None,
            },
        });
        self.question_event(id, EventKind::QuestionChanged { phase });
        Ok(id)
    }

    fn question_mut(&mut self, id: u64) -> Option<&mut Question> {
        self.questions.iter_mut().find(|q| q.view.id == id)
    }

    fn speech_pending(&self, q: &Question) -> bool {
        !q.closed
            && self.open.values().any(|speech| {
                speech.span.start_ms.saturating_sub(q.view.span.end_ms)
                    <= self.policy.continuation_ms
            })
    }

    fn ready(&self, id: u64, now_ms: u64) -> bool {
        self.questions
            .iter()
            .find(|q| q.view.id == id)
            .is_some_and(|q| {
                q.closed
                    || (now_ms.saturating_sub(q.view.span.end_ms) >= self.policy.debounce_ms
                        && !self.speech_pending(q))
            })
    }

    fn accepts(&self, key: AnswerKey) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|p| p.work.request.key == key && p.completed.is_none())
            && self
                .questions
                .iter()
                .any(|q| q.view.id == key.question_id && q.view.revision == key.revision)
    }

    fn settle(&mut self) {
        let Some(pending) = &self.pending else {
            return;
        };
        let key = pending.work.request.key;
        if pending.completed.is_none() {
            return;
        }
        let Some(q) = self.questions.iter().find(|q| q.view.id == key.question_id) else {
            return;
        };
        if key.mode == AnswerMode::Checked && self.speech_pending(q) {
            return;
        }
        let pending = self.pending.take().expect("completed pending work exists");
        let status = if key.mode == AnswerMode::Draft {
            AnswerStatus::Provisional
        } else {
            AnswerStatus::Checked
        };
        let answer = Answer {
            key,
            status,
            text: pending.completed.expect("answer completed"),
        };
        let q = self
            .question_mut(key.question_id)
            .expect("completed a retained question");
        q.view.answer = Some(answer.clone());
        let phase = if key.mode == AnswerMode::Draft {
            QuestionPhase::Waiting
        } else {
            let question = q.view.text.clone();
            let item = HistoryItem { question, answer };
            if let Some(previous) = self
                .history
                .iter_mut()
                .find(|old| old.answer.key.question_id == key.question_id)
            {
                *previous = item;
            } else {
                if self.history.len() == MAX_HISTORY {
                    self.history.pop_front();
                }
                self.history.push_back(item);
            }
            QuestionPhase::Checked
        };
        self.phase(key.question_id, phase);
        self.question_event(key.question_id, EventKind::AnswerChanged { status });
    }

    fn invalidate(&mut self, id: u64, phase: QuestionPhase) {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.work.request.key.question_id == id)
        {
            let pending = self.pending.take().expect("checked pending work");
            pending.work.cancelled.store(true, Ordering::Release);
        }
        let superseded = self
            .question_mut(id)
            .and_then(|q| q.view.answer.as_mut())
            .is_some_and(|answer| {
                if answer.status == AnswerStatus::Provisional {
                    answer.status = AnswerStatus::Superseded;
                }
                answer.status == AnswerStatus::Superseded
            });
        self.phase(id, phase);
        if superseded {
            self.question_event(
                id,
                EventKind::AnswerChanged {
                    status: AnswerStatus::Superseded,
                },
            );
        }
    }

    fn fail(&mut self, id: u64, error: AnswerFailure) {
        let draft = self
            .pending
            .as_ref()
            .is_some_and(|p| p.work.request.key.mode == AnswerMode::Draft);
        if draft {
            self.invalidate(id, QuestionPhase::Waiting);
        } else {
            self.notice = Some(error.message().to_owned());
            self.invalidate(id, QuestionPhase::Error);
        }
    }

    fn phase(&mut self, id: u64, phase: QuestionPhase) {
        if let Some(q) = self.question_mut(id) {
            q.view.phase = phase;
        }
        self.question_event(id, EventKind::QuestionChanged { phase });
    }

    fn question_event(&mut self, id: u64, kind: EventKind) {
        if let Some(q) = self.questions.iter().find(|q| q.view.id == id) {
            self.record(
                q.view.source.clone(),
                q.view.span,
                Some((id, q.view.revision)),
                kind,
            );
        }
    }

    fn record(
        &mut self,
        source: Source,
        span: Span,
        question: Option<(u64, u64)>,
        kind: EventKind,
    ) {
        self.sequence += 1;
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(Event {
            protocol_version: PROTOCOL_VERSION,
            sequence: self.sequence,
            session_id: self.id,
            question_id: question.map(|(id, _)| id),
            revision: question.map(|(_, revision)| revision),
            source,
            span,
            kind,
        });
    }
}
