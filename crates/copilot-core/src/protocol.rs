use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_UTTERANCE_BYTES: usize = 4_000;
pub const MAX_QUESTION_BYTES: usize = 64_000;
pub const MAX_ANSWER_BYTES: usize = 64_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    SystemOutput,
    Microphone,
    Replay,
    Typed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Source {
    pub kind: SourceKind,
    pub endpoint_id: Option<String>,
}

impl Source {
    pub const REPLAY: Self = Self {
        kind: SourceKind::Replay,
        endpoint_id: None,
    };
    pub const TYPED: Self = Self {
        kind: SourceKind::Typed,
        endpoint_id: None,
    };
}

/// Monotonic time in milliseconds from the start of this session.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Span {
    pub start_ms: u64,
    pub end_ms: u64,
}

impl Span {
    pub const fn at(ms: u64) -> Self {
        Self {
            start_ms: ms,
            end_ms: ms,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    Interim,
    Stable,
    Final,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Utterance {
    pub id: u64,
    pub revision: u64,
    pub span: Span,
    pub stability: Stability,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureState {
    Idle,
    Capturing,
    Paused,
    Stopped,
    Recovery,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionPhase {
    Candidate,
    Waiting,
    Incomplete,
    Ready,
    Drafting,
    Answering,
    Checked,
    Cancelled,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerMode {
    Draft,
    Checked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerStatus {
    Provisional,
    Checked,
    Superseded,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerFailure {
    TimedOut,
    Unavailable,
    InvalidOutput,
}

impl AnswerFailure {
    pub fn message(self) -> &'static str {
        match self {
            Self::TimedOut => "Answer timed out. Review the question and try again.",
            Self::Unavailable => {
                "Answer provider is unavailable. Check its connection and try again."
            }
            Self::InvalidOutput => "Answer provider returned an incomplete or oversized response.",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnswerKey {
    pub session_id: u64,
    pub question_id: u64,
    pub revision: u64,
    pub mode: AnswerMode,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnswerRequest {
    pub key: AnswerKey,
    pub question: String,
    pub source: Source,
    pub span: Span,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Answer {
    pub key: AnswerKey,
    pub status: AnswerStatus,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QuestionView {
    pub id: u64,
    pub revision: u64,
    pub phase: QuestionPhase,
    pub span: Span,
    pub source: Source,
    pub text: String,
    pub answer: Option<Answer>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HistoryItem {
    pub question: String,
    pub answer: Answer,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snapshot {
    pub protocol_version: u16,
    pub sequence: u64,
    pub session_id: u64,
    pub source: Source,
    pub capture: CaptureState,
    pub speech_active: bool,
    pub transcript: Vec<Utterance>,
    pub transcript_truncated: bool,
    pub question: Option<QuestionView>,
    pub history: Vec<HistoryItem>,
    pub dropped_ms: u64,
    pub notice: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Event {
    pub protocol_version: u16,
    pub sequence: u64,
    pub session_id: u64,
    pub question_id: Option<u64>,
    pub revision: Option<u64>,
    pub source: Source,
    pub span: Span,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum EventKind {
    CaptureChanged {
        state: CaptureState,
    },
    SpeechActive {
        utterance_id: u64,
    },
    TranscriptChanged {
        utterance_id: u64,
        stability: Stability,
    },
    QuestionChanged {
        phase: QuestionPhase,
    },
    AnswerChanged {
        status: AnswerStatus,
    },
    AudioGap {
        dropped_ms: u64,
    },
}
