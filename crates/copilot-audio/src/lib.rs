//! Local audio capture and recognition. Consumers receive text and health events only.

mod parakeet;
mod pipeline;
mod whisper;
#[cfg(windows)]
mod windows;

use std::{
    fmt,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use copilot_core::{Source, SourceKind, Span, Utterance};
use serde::{Deserialize, Serialize};

pub use parakeet::{LocalParakeet, PARAKEET_FILES, PARAKEET_FOLDER, PARAKEET_REVISION};
pub use pipeline::{StreamOptions, recognize_wav};
pub use whisper::{LocalWhisper, MODELS, Model};

/// Local speech engine. Whisper `small.en` stays the default.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    #[default]
    Whisper,
    Parakeet,
}

/// Loads a verified model. Vocabulary and short follow-ups only change Whisper decoding.
pub fn load(
    engine: Engine,
    path: &Path,
    vocabulary: &str,
    short_followups: bool,
) -> Result<Box<dyn SpeechRecognizer + Send>, AudioError> {
    Ok(match engine {
        Engine::Whisper => {
            let mut model = LocalWhisper::load(path, vocabulary)?;
            if short_followups {
                model.enable_short_followups();
            }
            Box::new(model)
        }
        Engine::Parakeet => Box::new(LocalParakeet::load(path)?),
    })
}

/// Opens a model file so nothing can write, rename or delete it while the handle lives.
pub(crate) fn open_locked(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    std::os::windows::fs::OpenOptionsExt::share_mode(&mut options, 1); // FILE_SHARE_READ
    options.open(path)
}

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
    /// The optional microphone lane ended. The session's main source keeps running.
    MicrophoneStopped {
        message: String,
    },
}

#[derive(Clone, Debug)]
pub struct AudioError {
    pub message: String,
    pub dropped_ms: u64,
    /// The audio device failed. Reopening it can bring capture back.
    pub device: bool,
}

impl AudioError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            dropped_ms: 0,
            device: false,
        }
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    fn device(message: impl Into<String>) -> Self {
        Self {
            device: true,
            ..Self::new(message)
        }
    }
}

/// Waits before reopening a failed device. After the last one, the default device of the same
/// kind is tried.
const REOPEN_MS: [u64; 3] = [0, 500, 1_500];

/// Keeps one capture alive across device faults. `open` runs a capture until it stops or fails
/// and is told how long audio was lost before it. Only a capture that has delivered audio is
/// reopened: a device that never worked is the user's to fix.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn reconnecting(
    mut source: Source,
    stop: &Stop,
    connected: &AtomicBool,
    mut open: impl FnMut(&Source, Option<u64>) -> Result<(), AudioError>,
) -> Result<(), AudioError> {
    let (mut worked, mut tries, mut lost) = (false, 0, None::<Instant>);
    loop {
        let result = open(&source, lost.map(|at| at.elapsed().as_millis() as u64));
        if connected.swap(false, Ordering::AcqRel) {
            (worked, tries, lost) = (true, 0, None);
        }
        let Err(error) = result else {
            return Ok(());
        };
        if !worked || !error.device {
            return Err(error);
        }
        lost.get_or_insert_with(Instant::now);
        match REOPEN_MS.get(tries) {
            Some(wait) => {
                let until = Instant::now() + std::time::Duration::from_millis(*wait);
                while !stop.requested() && Instant::now() < until {
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
            }
            // Last resort: the device Windows now uses for this kind of audio.
            None if source.endpoint_id.is_some() => source.endpoint_id = None,
            None => return Err(error),
        }
        if stop.requested() {
            return Ok(());
        }
        tries += 1;
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AudioError {}

/// Decodes one bounded 16 kHz PCM segment on the inference worker.
pub trait SpeechRecognizer {
    fn decode(&mut self, samples: &[i16], final_result: bool) -> Result<String, AudioError>;

    /// Receives a flag that is set while a final result waits for the decoder. An engine that
    /// can cut a preview decode short returns empty text for it. The default keeps decoding.
    fn yield_previews_to(&mut self, _final_waiting: Arc<AtomicBool>) {}
}

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

/// Captures `source` until `stop`. With `microphone`, the user's own voice is transcribed beside
/// it: those lines carry `you` and a microphone failure never ends the capture.
pub fn capture(
    model: &mut dyn SpeechRecognizer,
    source: Source,
    microphone: Option<Source>,
    clock: Instant,
    options: StreamOptions,
    stop: &Stop,
    report: impl Fn(AudioEvent) + Sync,
) -> Result<(), AudioError> {
    #[cfg(windows)]
    {
        let report = &report;
        let connected = &AtomicBool::new(false);
        let main = &|event: AudioEvent| {
            if matches!(event, AudioEvent::Ready { .. }) {
                connected.store(true, Ordering::Release);
            }
            report(event)
        };
        pipeline::run(
            model,
            clock,
            options,
            stop,
            report,
            |producer| {
                reconnecting(source, stop, connected, |source, lost_ms| {
                    windows::capture(source.clone(), clock, stop, producer, main, lost_ms)
                })
            },
            microphone.map(|microphone| -> pipeline::SideFeed<'_> {
                Box::new(move |producer, halt| {
                    windows::capture(microphone, clock, halt, producer, &|_| {}, None)
                })
            }),
        )
    }
    #[cfg(not(windows))]
    {
        let _ = (model, source, microphone, clock, options, stop, report);
        Err(AudioError::new(
            "Live audio capture is available on Windows.",
        ))
    }
}
