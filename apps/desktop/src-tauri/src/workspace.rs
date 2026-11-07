use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use copilot_audio::{AudioError, AudioEvent, Engine, Stop, StreamOptions, elapsed_ms};
use copilot_core::{
    AnswerFailure, AnswerKey, CaptureState, MAX_QUESTION_BYTES, Policy, Session, Snapshot, Source,
    SourceKind, Span, Work,
    rehearsal::{self, Step},
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::providers::{ANSWER_TIMEOUT_MS, ProviderHub, Selection};

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
    #[serde(default)]
    engine: Engine,
    model_path: String,
    vocabulary: String,
    #[serde(default)]
    auto_answer: bool,
    #[serde(default)]
    short_followups: bool,
    #[serde(default)]
    consent: bool,
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
    started: Instant,
    stopping: Option<CaptureState>,
    failed: bool,
}

#[derive(Clone, Copy, Default, Serialize)]
struct AudioMetrics {
    frames: u64,
    ready_ms: Option<u64>,
    decode_ms: Option<u64>,
    queue_ms: Option<u64>,
}

#[derive(Clone, Serialize)]
struct MeterView {
    update: u64,
    session_id: u64,
    peak: u16,
    frames: u64,
}

enum AudioUpdate {
    Snapshot(Box<View>),
    Meter(MeterView),
}

struct Job {
    config: CaptureConfig,
    clock: Instant,
    session: u64,
    first_id: u64,
    stop: Stop,
}

pub struct Generation {
    route: Selection,
    work: Work,
}

#[derive(Clone, Serialize)]
pub struct View {
    update: u64,
    session: Snapshot,
    mode: Mode,
    meter: u16,
    audio_metrics: AudioMetrics,
    loading: bool,
    stopping: bool,
    retryable: bool,
    provider: Option<Selection>,
    shortcut_available: bool,
    auto_answer: bool,
}

pub struct Desktop {
    session: Session,
    instance_id: uuid::Uuid,
    serial: u64,
    update: u64,
    clock: Instant,
    mode: Mode,
    replay: Option<Replay>,
    audio: Option<AudioRun>,
    config: Option<CaptureConfig>,
    next_utterance: u64,
    meter: u16,
    audio_metrics: AudioMetrics,
    provider: Option<Selection>,
    shortcut_available: bool,
}

impl Desktop {
    pub fn shared() -> Shared {
        Arc::new(Mutex::new(Self {
            session: Session::new(0, Source::TYPED, provider_policy()),
            instance_id: uuid::Uuid::new_v4(),
            serial: 0,
            update: 0,
            clock: Instant::now(),
            mode: Mode::Idle,
            replay: None,
            audio: None,
            config: None,
            next_utterance: 1,
            meter: 0,
            audio_metrics: AudioMetrics::default(),
            provider: None,
            shortcut_available: false,
        }))
    }

    pub fn view(&self) -> View {
        View {
            update: self.update,
            session: self.session.snapshot(),
            mode: self.mode,
            meter: self.meter,
            audio_metrics: self.audio_metrics,
            loading: self.audio.as_ref().is_some_and(|audio| {
                self.audio_metrics.ready_ms.is_none() && audio.stopping.is_none()
            }),
            stopping: self
                .audio
                .as_ref()
                .is_some_and(|audio| audio.stopping.is_some()),
            retryable: self.audio.is_none()
                && self.config.is_some()
                && self.session.capture_state() == CaptureState::Recovery,
            provider: self.provider.clone(),
            shortcut_available: self.shortcut_available,
            auto_answer: self.session.auto_answer(),
        }
    }

    fn changed(&mut self) -> View {
        self.update += 1;
        self.view()
    }

    fn reset(&mut self, mode: Mode, source: Source, policy: Policy) -> Result<(), String> {
        if self.audio.is_some()
            || self.replay.is_some()
            || self.session.capture_state() == CaptureState::Paused
        {
            return Err("Stop the current session first.".into());
        }
        self.serial += 1;
        self.clock = Instant::now();
        self.session.cancel_answer();
        self.session = Session::new(self.serial, source, policy);
        self.mode = mode;
        self.config = None;
        self.next_utterance = 1;
        self.meter = 0;
        self.audio_metrics = AudioMetrics::default();
        Ok(())
    }

    pub fn start_sample(&mut self) -> Result<(), String> {
        let steps: VecDeque<Step> = serde_json::from_str(rehearsal::DEMO_JSON)
            .map_err(|_| "Could not read the bundled sample.")?;
        let policy = Policy {
            auto_answer: true,
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
            if !config.consent {
                return Err("Confirm that you have permission to capture this source.".into());
            }
            if config.auto_answer && self.provider.is_none() {
                return Err("Choose an answer provider before enabling automatic answers.".into());
            }
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
            let policy = Policy {
                auto_answer: config.auto_answer,
                ..provider_policy()
            };
            self.reset(Mode::Audio, config.source.clone(), policy)?;
            self.config = Some(config);
        } else if self.audio.is_some()
            || !matches!(
                self.session.capture_state(),
                CaptureState::Paused | CaptureState::Recovery
            )
        {
            return Err(
                "Pause capture or recover a disconnected device before restarting it.".into(),
            );
        }
        let config = self.config.clone().ok_or("Choose an audio source first.")?;
        let stop = Stop::default();
        self.audio_metrics = AudioMetrics::default();
        self.audio = Some(AudioRun {
            stop: stop.clone(),
            started: Instant::now(),
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
            if !pause || audio.stopping.is_none() {
                audio.stopping = Some(if pause {
                    CaptureState::Paused
                } else {
                    CaptureState::Stopped
                });
            }
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
        self.require_provider()?;
        if text.trim().is_empty() || text.trim().len() > MAX_QUESTION_BYTES {
            return Err("Enter a question within the length limit.".into());
        }
        if matches!(self.mode, Mode::Sample) {
            self.reset(Mode::Idle, Source::TYPED, provider_policy())?;
        }
        self.session
            .ask(text, elapsed_ms(self.clock))
            .map_err(Into::into)
    }

    pub fn full_transcript(&self) -> Result<String, String> {
        self.session.full_transcript().map_err(Into::into)
    }

    pub fn archive(&self) -> Result<(String, u64, Snapshot), String> {
        if self.audio.is_some()
            || self.replay.is_some()
            || self.session.capture_state() == CaptureState::Paused
        {
            return Err("Stop the session before saving it to history.".into());
        }
        let key = format!("{}:{}", self.instance_id, self.serial);
        Ok((key, self.update, self.session.snapshot()))
    }

    pub fn request_detected(&mut self) -> Result<(), String> {
        if !matches!(self.mode, Mode::Sample) {
            self.require_provider()?;
        }
        self.session.request_detected().map_err(Into::into)
    }

    fn require_provider(&self) -> Result<(), String> {
        self.provider
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| "Choose an answer provider in Providers.".into())
    }

    pub fn select_provider(
        &mut self,
        hub: &ProviderHub,
        route: Selection,
        consent: bool,
    ) -> Result<(), String> {
        hub.select(route, consent)?;
        self.sync_provider(hub.selection());
        Ok(())
    }

    pub fn sync_provider(&mut self, provider: Option<Selection>) {
        if self.provider != provider && !matches!(self.mode, Mode::Sample) {
            self.session.change_answer_route();
        }
        self.provider = provider;
    }

    pub fn set_shortcut_available(&mut self, available: bool) {
        self.shortcut_available = available;
    }

    pub fn pause_when_hidden(&mut self) -> Result<(), String> {
        if self
            .audio
            .as_ref()
            .is_some_and(|audio| audio.stopping.is_none())
            || self.replay.as_ref().is_some_and(|replay| !replay.paused)
        {
            self.stop(true)?;
        }
        Ok(())
    }

    pub fn cancel_answer(&mut self) {
        self.session.cancel_answer();
    }

    pub fn clear(&mut self) -> Result<(), String> {
        self.reset(Mode::Idle, Source::TYPED, provider_policy())
    }

    pub fn tick(&mut self) -> (Option<View>, Option<Generation>) {
        let before = self.session.sequence();
        let now = if let Some(replay) = &mut self.replay {
            if replay.paused {
                return (None, None);
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
                    return (Some(self.changed()), None);
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
        let generation = if matches!(self.mode, Mode::Sample) {
            rehearsal::answer_available(&mut self.session, now);
            None
        } else {
            self.provider.as_ref().and_then(|route| {
                self.session.next_request(now).map(|work| Generation {
                    route: route.clone(),
                    work,
                })
            })
        };
        (
            (self.session.sequence() != before).then(|| self.changed()),
            generation,
        )
    }

    fn answer_delta(&mut self, key: AnswerKey, delta: &str) -> Option<View> {
        self.session
            .append_answer(key, delta)
            .then(|| self.changed())
    }

    fn finish_answer(
        &mut self,
        key: AnswerKey,
        answer: Result<String, AnswerFailure>,
    ) -> Option<View> {
        self.session
            .complete(key, elapsed_ms(self.clock), answer)
            .then(|| self.changed())
    }

    fn audio_event(&mut self, event: AudioEvent) -> Option<AudioUpdate> {
        let audio = self.audio.as_mut()?;
        if audio.failed {
            return None;
        }
        let now = elapsed_ms(self.clock);
        let result = match event {
            AudioEvent::Ready { .. } => {
                if audio.stopping.is_some() {
                    return None;
                }
                self.audio_metrics.ready_ms = Some(elapsed_ms(audio.started));
                self.session.set_capture(CaptureState::Capturing, now);
                Ok(())
            }
            AudioEvent::Speech { utterance_id, span } => {
                self.next_utterance = self.next_utterance.max(utterance_id.saturating_add(1));
                self.session.observe_speech(utterance_id, span)
            }
            AudioEvent::Transcript {
                utterance,
                decode_ms,
                queue_ms,
                ..
            } => {
                self.next_utterance = self.next_utterance.max(utterance.id.saturating_add(1));
                self.audio_metrics.decode_ms = Some(decode_ms);
                self.audio_metrics.queue_ms = Some(queue_ms);
                self.session.ingest(utterance)
            }
            AudioEvent::Meter { peak, frames } => {
                self.audio_metrics.frames = frames;
                if audio.stopping.is_some() || (peak == self.meter && !frames.is_multiple_of(50)) {
                    return None;
                }
                self.meter = peak;
                self.update += 1;
                return Some(AudioUpdate::Meter(MeterView {
                    update: self.update,
                    session_id: self.serial,
                    peak,
                    frames,
                }));
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
        Some(AudioUpdate::Snapshot(Box::new(self.changed())))
    }

    fn finish_audio(&mut self, result: Result<(), AudioError>) -> Option<View> {
        let run = self.audio.take()?;
        self.meter = 0;
        let now = elapsed_ms(self.clock);
        if let Err(error) = result {
            if self.audio_metrics.ready_ms.is_none() || run.stopping == Some(CaptureState::Stopped)
            {
                self.config = None;
            }
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

fn provider_policy() -> Policy {
    Policy {
        answer_timeout_ms: ANSWER_TIMEOUT_MS,
        ..Policy::default()
    }
}

pub fn run_generation(
    app: AppHandle,
    state: Shared,
    providers: Arc<ProviderHub>,
    generation: Generation,
) {
    tauri::async_runtime::spawn(async move {
        let key = generation.work.request.key;
        let cancel = async {
            while !generation.work.is_cancelled() {
                tokio::time::sleep(Duration::from_millis(40)).await;
            }
        };
        let result = tokio::select! {
            biased;
            () = cancel => return,
            answer = providers.answer(&generation.route, &generation.work.request, |delta| {
                let view = {
                    let mut desktop = state.lock().expect("desktop session lock");
                    if generation.work.is_cancelled() {
                        return false;
                    }
                    desktop.answer_delta(key, delta)
                };
                if let Some(view) = view {
                    emit(&app, &view);
                    true
                } else {
                    false
                }
            }) => answer,
        };
        let view = {
            let mut desktop = state.lock().expect("desktop session lock");
            if generation.work.is_cancelled() {
                return;
            }
            desktop.finish_answer(key, result)
        };
        if let Some(view) = view {
            emit(&app, &view);
        }
    });
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
        let result = (|| {
            if job.stop.requested() {
                return Ok(());
            }
            let model = copilot_audio::load(
                job.config.engine,
                Path::new(&job.config.model_path),
                &job.config.vocabulary,
                job.config.short_followups,
            );
            if job.stop.requested() {
                return Ok(());
            }
            model.and_then(|mut model| {
                copilot_audio::capture(
                    model.as_mut(),
                    job.config.source,
                    job.clock,
                    StreamOptions {
                        first_id: job.first_id,
                        ..StreamOptions::default()
                    },
                    &job.stop,
                    |event| {
                        let update = {
                            let mut desktop = state.lock().expect("desktop session lock");
                            (desktop.serial == job.session)
                                .then(|| desktop.audio_event(event))
                                .flatten()
                        };
                        match update {
                            Some(AudioUpdate::Snapshot(view)) => emit(&app, &view),
                            Some(AudioUpdate::Meter(meter)) => {
                                let _ = app.emit("copilot-meter", meter);
                            }
                            None => {}
                        }
                    },
                )
            })
        })();
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

#[cfg(test)]
mod tests {
    use super::*;
    use copilot_core::{Stability, Utterance};

    fn begin(desktop: &mut Desktop) -> Job {
        desktop
            .begin_audio(Some(CaptureConfig {
                source: Source {
                    kind: SourceKind::SystemOutput,
                    endpoint_id: Some("test-output".into()),
                },
                engine: Engine::Whisper,
                model_path: "ggml-small.en.bin".into(),
                vocabulary: String::new(),
                auto_answer: false,
                short_followups: false,
                consent: true,
            }))
            .unwrap()
    }

    #[test]
    fn stop_supersedes_pause_and_late_ready_cannot_reopen_capture() {
        let shared = Desktop::shared();
        let mut desktop = shared.lock().unwrap();
        let job = begin(&mut desktop);
        desktop.stop(true).unwrap();
        desktop.stop(false).unwrap();
        desktop.stop(true).unwrap();
        assert!(job.stop.requested());
        assert!(
            desktop
                .audio_event(AudioEvent::Ready {
                    source: job.config.source
                })
                .is_none()
        );
        let view = desktop.finish_audio(Ok(())).unwrap();
        assert_eq!(view.session.capture, CaptureState::Stopped);
        assert!(!view.stopping);
        assert!(desktop.begin_audio(None).is_err());
    }

    #[test]
    fn late_transcript_drains_and_recovery_preserves_it() {
        let shared = Desktop::shared();
        let mut desktop = shared.lock().unwrap();
        let job = begin(&mut desktop);
        desktop.audio_event(AudioEvent::Ready {
            source: job.config.source,
        });
        desktop.stop(false).unwrap();
        desktop.audio_event(AudioEvent::Transcript {
            utterance: Utterance {
                id: 1,
                revision: 1,
                span: Span {
                    start_ms: 0,
                    end_ms: 1_000,
                },
                stability: Stability::Final,
                text: "How does a cache work?".into(),
            },
            at_ms: 1_002,
            decode_ms: 2,
            queue_ms: 0,
        });
        let view = desktop
            .finish_audio(Err(AudioError {
                message: "Device disconnected".into(),
                dropped_ms: 30,
            }))
            .unwrap();
        assert_eq!(view.session.capture, CaptureState::Recovery);
        assert_eq!(view.session.dropped_ms, 30);
        assert_eq!(view.session.transcript[0].text, "How does a cache work?");
        assert_eq!(view.audio_metrics.decode_ms, Some(2));
        begin(&mut desktop);
    }

    #[test]
    fn metering_payload_does_not_reserialize_the_transcript() {
        let shared = Desktop::shared();
        let mut desktop = shared.lock().unwrap();
        let job = begin(&mut desktop);
        desktop.audio_event(AudioEvent::Ready {
            source: job.config.source,
        });
        for id in 1..=80 {
            desktop
                .session
                .ingest(Utterance {
                    id,
                    revision: 1,
                    span: Span::at(id * 1_000),
                    stability: Stability::Final,
                    text: "This stable transcript line is already present in the renderer.".into(),
                })
                .unwrap();
        }
        let Some(AudioUpdate::Meter(meter)) = desktop.audio_event(AudioEvent::Meter {
            peak: 1_500,
            frames: 200,
        }) else {
            panic!("expected a standalone meter event");
        };
        let meter_size = serde_json::to_vec(&meter).unwrap().len();
        let view_size = serde_json::to_vec(&desktop.view()).unwrap().len();
        println!("Meter JSON {meter_size} bytes, full snapshot JSON {view_size} bytes.");
        assert_eq!(desktop.view().audio_metrics.frames, 200);
        assert!(meter_size * 10 < view_size);
    }

    #[test]
    fn retry_keeps_session_id_and_transcript_after_capture_failure() {
        let shared = Desktop::shared();
        let mut desktop = shared.lock().unwrap();
        let job = begin(&mut desktop);
        desktop.audio_event(AudioEvent::Ready {
            source: job.config.source,
        });
        desktop.audio_event(AudioEvent::Transcript {
            utterance: Utterance {
                id: 1,
                revision: 1,
                span: Span::at(200),
                stability: Stability::Final,
                text: "Explain an index.".into(),
            },
            at_ms: 300,
            decode_ms: 100,
            queue_ms: 0,
        });
        let error = AudioError {
            message: "Device disconnected".into(),
            dropped_ms: 30,
        };
        assert!(desktop.finish_audio(Err(error)).unwrap().retryable);
        assert_eq!(desktop.archive().unwrap().2.transcript.len(), 1);
        let retry = desktop.begin_audio(None).unwrap();
        assert_eq!(retry.session, job.session);
        assert_eq!(retry.first_id, 2);
        desktop.audio_event(AudioEvent::Ready {
            source: retry.config.source,
        });
        let view = desktop.view();
        assert_eq!(view.session.capture, CaptureState::Capturing);
        assert!(view.session.notice.is_none());
        assert_eq!(view.session.transcript[0].text, "Explain an index.");
        assert_eq!(view.session.dropped_ms, 30);
    }
}
