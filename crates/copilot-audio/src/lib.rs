//! Local audio capture and recognition. Consumers receive text and health events only.

mod pipeline;
mod whisper;
#[cfg(windows)]
mod windows;

use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use copilot_core::{Source, SourceKind, Span, Utterance};
use serde::{Deserialize, Serialize};

pub use pipeline::{StreamOptions, recognize_wav};
pub use whisper::{LocalWhisper, MODELS, Model};

pub const SAMPLE_RATE: u32 = 16_000;
const FRAME_MS: u64 = 30;
const SAMPLES: usize = (SAMPLE_RATE as u64 * FRAME_MS / 1_000) as usize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Endpoint {
    pub id: String,
    pub name: String,
    pub kind: SourceKind,
    pub is_default: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AudioEvent {
    Ready {
        source: Source,
    },
    Speech {
        utterance_id: u64,
        span: Span,
    },
    Transcript {
        utterance: Utterance,
        at_ms: u64,
        decode_ms: u64,
        queue_ms: u64,
    },
    Meter {
        peak: u16,
        frames: u64,
    },
    Discontinuity {
        span: Span,
        dropped_ms: u64,
    },
}

#[derive(Clone, Debug)]
pub struct AudioError {
    pub message: String,
    pub dropped_ms: u64,
}

impl AudioError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            dropped_ms: 0,
        }
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AudioError {}

#[derive(Clone, Default)]
pub struct Stop(Arc<AtomicBool>);

impl Stop {
    pub fn request(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn requested(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub fn elapsed_ms(clock: Instant) -> u64 {
    clock.elapsed().as_millis() as u64
}

pub fn endpoints() -> Result<Vec<Endpoint>, AudioError> {
    #[cfg(windows)]
    {
        windows::endpoints()
    }
    #[cfg(not(windows))]
    {
        Err(AudioError::new(
            "Live audio capture is available on Windows.",
        ))
    }
}

pub fn capture(
    model: &mut LocalWhisper,
    source: Source,
    clock: Instant,
    options: StreamOptions,
    stop: &Stop,
    report: impl Fn(AudioEvent) + Sync,
) -> Result<(), AudioError> {
    #[cfg(windows)]
    {
        pipeline::run(model, clock, options, stop, &report, |producer| {
            windows::capture(source, clock, stop, producer, &report)
        })
    }
    #[cfg(not(windows))]
    {
        let _ = (model, source, clock, options, stop, report);
        Err(AudioError::new(
            "Live audio capture is available on Windows.",
        ))
    }
}
