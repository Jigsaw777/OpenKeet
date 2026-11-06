use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use copilot_audio::{AudioError, AudioEvent, LocalWhisper, Stop, StreamOptions, elapsed_ms};
use copilot_core::{
    CaptureState, Policy, Session, Snapshot, Source, SourceKind, Span,
    rehearsal::{self, Step},
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

pub type Shared = Arc<Mutex<Desktop>>;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Idle,
    Sample,
    Audio,
}

#[derive(Clone, Deserialize)]
pub struct CaptureConfig {
    source: Source,
    model_path: String,
    vocabulary: String,
}

struct Replay {
    steps: VecDeque<Step>,
    elapsed: u64,
    finish_at_ms: u64,
    tick: Instant,
    paused: bool,
}

struct AudioRun {
    stop: Stop,
    ready: bool,
    stopping: Option<CaptureState>,
    failed: bool,
}

struct Job {
    config: CaptureConfig,
    clock: Instant,
    session: u64,
    first_id: u64,
    stop: Stop,
}

#[derive(Clone, Serialize)]
pub struct View {
    update: u64,
    session: Snapshot,
    mode: Mode,
    meter: u16,
    loading: bool,
    stopping: bool,
}

pub struct Desktop {
    session: Session,
    serial: u64,
    update: u64,
    clock: Instant,
    mode: Mode,
    replay: Option<Replay>,
    audio: Option<AudioRun>,
    config: Option<CaptureConfig>,
    next_utterance: u64,
    meter: u16,
}

impl Desktop {
    pub fn shared() -> Shared {
        Arc::new(Mutex::new(Self {
            session: Session::new(0, Source::TYPED, Policy::default()),
            serial: 0,
            update: 0,
            clock: Instant::now(),
            mode: Mode::Idle,
            replay: None,
            audio: None,
            config: None,
            next_utterance: 1,
            meter: 0,
        }))
    }

    pub fn view(&self) -> View {
        View {
            update: self.update,
            session: self.session.snapshot(),
            mode: self.mode,
            meter: self.meter,
            loading: self
                .audio
                .as_ref()
                .is_some_and(|audio| !audio.ready && audio.stopping.is_none()),
            stopping: self
                .audio
                .as_ref()
                .is_some_and(|audio| audio.stopping.is_some()),
        }
    }

    fn changed(&mut self) -> View {
        self.update += 1;
        self.view()
    }

    fn reset(&mut self, mode: Mode, source: Source, policy: Policy) -> Result<(), String> {
        if self.audio.is_some() || self.replay.is_some() || self.config.is_some() {
            return Err("Stop the current session first.".into());
        }
        self.serial += 1;
        self.clock = Instant::now();
        self.session = Session::new(self.serial, source, policy);
        self.mode = mode;
        self.next_utterance = 1;
        self.meter = 0;
        Ok(())
    }

    pub fn start_sample(&mut self) -> Result<(), String> {
        let steps: VecDeque<Step> = serde_json::from_str(rehearsal::DEMO_JSON)
            .map_err(|_| "Could not read the bundled sample.")?;
        let policy = Policy {
            draft_after_ms: Some(600),
            ..Policy::default()
        };
        let finish_at_ms = steps.back().map_or(0, |last| last.at_ms) + policy.debounce_ms + 200;
        self.reset(Mode::Sample, Source::REPLAY, policy)?;
        self.session.set_capture(CaptureState::Capturing, 0);
        self.replay = Some(Replay {
            steps,
            elapsed: 0,
            finish_at_ms,
            tick: Instant::now(),
            paused: false,
        });
        Ok(())
    }

    fn begin_audio(&mut self, config: Option<CaptureConfig>) -> Result<Job, String> {
        if let Some(config) = config {
            if !matches!(
                config.source.kind,
                SourceKind::SystemOutput | SourceKind::Microphone
            ) || config
                .source
                .endpoint_id
                .as_ref()
                .is_none_or(|id| id.is_empty() || id.len() > 1_024)
                || config.model_path.is_empty()
                || config.model_path.len() > 32_768
            {
                return Err("Choose a Windows audio endpoint and a local speech model.".into());
            }
            self.reset(Mode::Audio, config.source.clone(), Policy::default())?;
            self.config = Some(config);
        } else if self.audio.is_some() || self.session.snapshot().capture != CaptureState::Paused {
            return Err("Pause capture before resuming it.".into());
        }
        let config = self.config.clone().ok_or("Choose an audio source first.")?;
        let stop = Stop::default();
        self.audio = Some(AudioRun {
            stop: stop.clone(),
            ready: false,
            stopping: None,
            failed: false,
        });
        Ok(Job {
            config,
            stop,
            clock: self.clock,
            session: self.serial,
            first_id: self.next_utterance,
        })
    }

    pub fn stop(&mut self, pause: bool) -> Result<(), String> {
        if let Some(audio) = &mut self.audio {
            audio.stopping = Some(if pause {
                CaptureState::Paused
            } else {
                CaptureState::Stopped
            });
            audio.stop.request();
        } else if pause {
            let replay = self
                .replay
                .as_mut()
                .ok_or("Start a session before pausing it.")?;
            replay.paused = true;
            self.session
                .set_capture(CaptureState::Paused, replay.elapsed);
        } else {
            self.replay = None;
            self.config = None;
            self.session.discard_interim();
            self.session
                .set_capture(CaptureState::Stopped, elapsed_ms(self.clock));
        }
        Ok(())
    }

    pub fn ask(&mut self, text: &str) -> Result<(), String> {
        let now = self
            .replay
            .as_ref()
            .map_or_else(|| elapsed_ms(self.clock), |r| r.elapsed);
        self.session.ask(text, now)?;
        rehearsal::answer_available(&mut self.session, now);
        Ok(())
    }

    pub fn full_transcript(&self) -> Result<String, String> {
        self.session.full_transcript().map_err(Into::into)
    }

    pub fn clear(&mut self) -> Result<(), String> {
        self.reset(Mode::Idle, Source::TYPED, Policy::default())
    }

    pub fn tick(&mut self) -> Option<View> {
        let before = self.session.sequence();
        let now = if let Some(replay) = &mut self.replay {
            if replay.paused {
                return None;
            }
            let instant = Instant::now();
            replay.elapsed += instant.duration_since(replay.tick).as_millis() as u64;
            replay.tick = instant;
            while replay
                .steps
                .front()
                .is_some_and(|step| step.at_ms <= replay.elapsed)
            {
                if let Err(message) = replay
                    .steps
                    .pop_front()
                    .expect("due step")
                    .apply(&mut self.session)
                {
                    self.session.audio_gap(Span::at(replay.elapsed), 0, message);
                    self.replay = None;
                    return Some(self.changed());
                }
            }
            let elapsed = replay.elapsed;
            if replay.steps.is_empty() && elapsed >= replay.finish_at_ms {
                self.session.set_capture(CaptureState::Stopped, elapsed);
                self.replay = None;
            }
            elapsed
        } else {
            elapsed_ms(self.clock)
        };
        rehearsal::answer_available(&mut self.session, now);
        (self.session.sequence() != before).then(|| self.changed())
    }

    fn audio_event(&mut self, event: AudioEvent) -> Option<View> {
        let audio = self.audio.as_mut()?;
        if audio.failed {
            return None;
        }
        let now = elapsed_ms(self.clock);
        let result = match event {
            AudioEvent::Ready { .. } => {
                audio.ready = true;
                self.session.set_capture(CaptureState::Capturing, now);
                Ok(())
            }
            AudioEvent::Speech { utterance_id, span } => {
                self.next_utterance = self.next_utterance.max(utterance_id + 1);
                self.session.observe_speech(utterance_id, span)
            }
            AudioEvent::Transcript { utterance, .. } => self.session.ingest(utterance),
            AudioEvent::Meter { peak, .. } => {
                if peak == self.meter {
                    return None;
                }
                self.meter = peak;
                Ok(())
            }
            AudioEvent::Discontinuity { span, dropped_ms } => {
                self.session.note_audio_gap(
                    span,
                    dropped_ms,
                    "Windows reported a short audio gap. Review the affected transcript.",
                );
                Ok(())
            }
        };
        if let Err(message) = result {
            audio.failed = true;
            audio.stop.request();
            self.session.audio_gap(Span::at(now), 0, message);
        }
        Some(self.changed())
    }

    fn finish_audio(&mut self, result: Result<(), AudioError>) -> Option<View> {
        let run = self.audio.take()?;
        self.meter = 0;
        let now = elapsed_ms(self.clock);
        if let Err(error) = result {
            self.config = None;
            self.session
                .audio_gap(Span::at(now), error.dropped_ms, error.message);
        } else if run.failed {
            self.config = None;
        } else {
            let state = run.stopping.unwrap_or(CaptureState::Stopped);
            if state == CaptureState::Stopped {
                self.config = None;
            }
            self.session.set_capture(state, now);
        }
        Some(self.changed())
    }
}

pub fn emit(app: &AppHandle, view: &View) {
    let _ = app.emit("copilot-snapshot", view);
}

pub fn update(
    app: &AppHandle,
    state: &Shared,
    change: impl FnOnce(&mut Desktop) -> Result<(), String>,
) -> Result<View, String> {
    let view = {
        let mut desktop = state.lock().expect("desktop session lock");
        change(&mut desktop)?;
        desktop.changed()
    };
    emit(app, &view);
    Ok(view)
}

pub fn resume_sample(app: &AppHandle, state: &Shared) -> Option<View> {
    let view = {
        let mut desktop = state.lock().expect("desktop session lock");
        let replay = desktop.replay.as_mut()?;
        replay.paused = false;
        replay.tick = Instant::now();
        let elapsed = replay.elapsed;
        desktop
            .session
            .set_capture(CaptureState::Capturing, elapsed);
        desktop.changed()
    };
    emit(app, &view);
    Some(view)
}

pub fn start_audio(
    app: AppHandle,
    state: Shared,
    config: Option<CaptureConfig>,
) -> Result<View, String> {
    let (job, view) = {
        let mut desktop = state.lock().expect("desktop session lock");
        let job = desktop.begin_audio(config)?;
        (job, desktop.changed())
    };
    emit(&app, &view);
    thread::spawn(move || {
        let result = LocalWhisper::load(Path::new(&job.config.model_path), &job.config.vocabulary)
            .and_then(|mut model| {
                if job.stop.requested() {
                    return Ok(());
                }
                copilot_audio::capture(
                    &mut model,
                    job.config.source,
                    job.clock,
                    StreamOptions {
                        first_id: job.first_id,
                        ..StreamOptions::default()
                    },
                    &job.stop,
                    |event| {
                        let view = {
                            let mut desktop = state.lock().expect("desktop session lock");
                            (desktop.serial == job.session)
                                .then(|| desktop.audio_event(event))
                                .flatten()
                        };
                        if let Some(view) = view {
                            emit(&app, &view);
                        }
                    },
                )
            });
        let view = state
            .lock()
            .expect("desktop session lock")
            .finish_audio(result);
        if let Some(view) = view {
            emit(&app, &view);
        }
    });
    Ok(view)
}
