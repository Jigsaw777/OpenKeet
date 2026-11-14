use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use copilot_audio::{
    AudioError, AudioEvent, Engine, SpeechRecognizer, Stop, StreamOptions, elapsed_ms,
};
use copilot_core::{
    AnswerFailure, AnswerKey, CaptureState, MAX_QUESTION_BYTES, Policy, Session, Snapshot, Source,
    SourceKind, Span, Work,
    prep::{
        Length,
        resolve::{Anticipation, Resolution, Resolver, anticipate, complete_from},
        words,
    },
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::providers::{ANSWER_TIMEOUT_MS, ProviderHub, Selection};

pub type Shared = Arc<Mutex<Desktop>>;
type Recognizer = Box<dyn SpeechRecognizer + Send>;

/// How long a paused session keeps its speech model in memory for an instant resume.
const WARM_MODEL: Duration = Duration::from_secs(600);

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Idle,
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
    /// A microphone endpoint to transcribe beside system audio, as the user's own side.
    #[serde(default)]
    microphone: Option<String>,
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
    model: Option<Recognizer>,
    clock: Instant,
    session: u64,
    first_id: u64,
    stop: Stop,
}

pub struct Generation {
    route: Option<Selection>,
    work: Work,
    /// Set when the active pack has paraphrase matching: tried before any provider.
    semantic: Option<Arc<Resolver>>,
}

/// Provenance of an answer that came from the active interview pack, keyed to its exact turn.
#[derive(Clone, Serialize)]
pub struct PreparedAnswer {
    key: AnswerKey,
    label: String,
    question: String,
    attribution: Option<String>,
    pack_revision: u32,
}

#[derive(Clone, Serialize)]
struct PackRef {
    id: String,
    name: String,
    revision: u32,
    paraphrase: Option<String>,
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
    pack: Option<PackRef>,
    anticipation: Option<Anticipation>,
    prepared_answers: Vec<PreparedAnswer>,
}

pub struct Desktop {
    session: Session,
    instance_id: uuid::Uuid,
    serial: u64,
    update: u64,
    clock: Instant,
    mode: Mode,
    audio: Option<AudioRun>,
    config: Option<CaptureConfig>,
    next_utterance: u64,
    meter: u16,
    audio_metrics: AudioMetrics,
    provider: Option<Selection>,
    shortcut_available: bool,
    prepared: Option<Arc<Resolver>>,
    anticipation: Option<Anticipation>,
    prepared_answers: Vec<PreparedAnswer>,
    /// Interpretation the user picked: (question id, revision, family id).
    chosen: Option<(u64, u64, String)>,
    query_vectors: HashMap<String, Vec<f32>>,
    /// The loaded speech model of a paused session, and when it was parked.
    warm: Option<(Recognizer, Instant)>,
    /// Streamed answer text the next tick still has to publish.
    answer_dirty: bool,
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
            audio: None,
            config: None,
            next_utterance: 1,
            meter: 0,
            audio_metrics: AudioMetrics::default(),
            provider: None,
            shortcut_available: false,
            prepared: None,
            anticipation: None,
            prepared_answers: Vec::new(),
            chosen: None,
            query_vectors: HashMap::new(),
            warm: None,
            answer_dirty: false,
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
            pack: self.prepared.as_ref().map(|r| PackRef {
                id: r.pack_id.clone(),
                name: r.pack_name.clone(),
                revision: r.revision,
                paraphrase: r.semantic_model().map(String::from),
            }),
            anticipation: self.anticipation.clone(),
            prepared_answers: self.prepared_answers.clone(),
        }
    }

    fn changed(&mut self) -> View {
        self.update += 1;
        self.answer_dirty = false;
        self.refresh_anticipation();
        self.view()
    }

    /// Prefetches local candidates for a forming question; stale revisions are recomputed.
    fn refresh_anticipation(&mut self) {
        let (Some(resolver), Some(question)) = (&self.prepared, self.session.current_question())
        else {
            self.anticipation = None;
            return;
        };
        if self
            .anticipation
            .as_ref()
            .is_some_and(|a| a.is_current(self.serial, question, resolver.revision))
        {
            return;
        }
        self.anticipation = anticipate(resolver, self.serial, question);
    }

    /// Uses an interview pack's approved answers in sessions, or stops using one.
    pub fn set_prepared(&mut self, resolver: Option<Arc<Resolver>>) -> Option<View> {
        let unchanged = match (&self.prepared, &resolver) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a.pack_id == b.pack_id
                    && a.revision == b.revision
                    && a.semantic_model() == b.semantic_model()
            }
            _ => false,
        };
        if unchanged {
            return None;
        }
        self.prepared = resolver;
        self.anticipation = None;
        Some(self.changed())
    }

    pub fn prepared(&self) -> Option<&Arc<Resolver>> {
        self.prepared.as_ref()
    }

    /// Answers the forming question with an interpretation the user picked from the list.
    pub fn choose_candidate(&mut self, family_id: &str) -> Result<(), String> {
        let question = self
            .session
            .current_question()
            .ok_or("Wait for a question first.")?;
        let listed = self.anticipation.as_ref().is_some_and(|a| {
            a.question_id == question.id
                && a.revision == question.revision
                && a.candidates.iter().any(|c| c.family_id == family_id)
        });
        if !listed {
            return Err("That suggestion is out of date. Wait for the question to settle.".into());
        }
        self.chosen = Some((question.id, question.revision, family_id.into()));
        self.session.request_detected().map_err(Into::into)
    }

    fn remember(&mut self, key: AnswerKey, hit: Resolution) {
        let Some(answer) = hit.answer else {
            return;
        };
        if self.prepared_answers.len() == 32 {
            self.prepared_answers.remove(0);
        }
        self.prepared_answers.push(PreparedAnswer {
            key,
            label: answer.label,
            question: answer.question,
            attribution: answer.attribution,
            pack_revision: hit.pack_revision,
        });
    }

    fn reset(&mut self, mode: Mode, source: Source, policy: Policy) -> Result<(), String> {
        if self.audio.is_some() || self.session.capture_state() == CaptureState::Paused {
            return Err("Stop the current session first.".into());
        }
        self.serial += 1;
        self.clock = Instant::now();
        self.session.cancel_answer();
        self.session = Session::new(self.serial, source, policy);
        self.mode = mode;
        self.config = None;
        self.warm = None;
        self.next_utterance = 1;
        self.meter = 0;
        self.audio_metrics = AudioMetrics::default();
        self.anticipation = None;
        self.prepared_answers.clear();
        self.chosen = None;
        Ok(())
    }

    fn begin_audio(&mut self, config: Option<CaptureConfig>) -> Result<Job, String> {
        if let Some(config) = config {
            if !config.consent {
                return Err("Confirm that you have permission to capture this source.".into());
            }
            if config.auto_answer && self.provider.is_none() && self.prepared.is_none() {
                return Err(
                    "Choose an answer provider or an interview pack before enabling automatic answers."
                        .into(),
                );
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
            if config.microphone.as_ref().is_some_and(|id| {
                config.source.kind != SourceKind::SystemOutput || id.is_empty() || id.len() > 1_024
            }) {
                return Err(
                    "Your microphone can be added only while listening to system audio.".into(),
                );
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
            model: self.warm.take().map(|(model, _)| model),
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
            return Err("Start a session before pausing it.".into());
        } else {
            self.config = None;
            self.warm = None;
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
        self.session
            .ask(text, elapsed_ms(self.clock))
            .map_err(Into::into)
    }

    pub fn full_transcript(&self) -> Result<String, String> {
        self.session.full_transcript().map_err(Into::into)
    }

    pub fn archive(&self) -> Result<(String, u64, Snapshot), String> {
        if self.audio.is_some() || self.session.capture_state() == CaptureState::Paused {
            return Err("Stop the session before saving it to history.".into());
        }
        let key = format!("{}:{}", self.instance_id, self.serial);
        Ok((key, self.update, self.session.snapshot()))
    }

    pub fn request_detected(&mut self) -> Result<(), String> {
        self.require_provider()?;
        self.session.request_detected().map_err(Into::into)
    }

    fn require_provider(&self) -> Result<(), String> {
        if self.provider.is_some() || self.prepared.is_some() {
            Ok(())
        } else {
            Err(
                "Choose an answer provider in Settings, or use an interview pack in this session."
                    .into(),
            )
        }
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
        if self.provider != provider {
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
        if self
            .warm
            .as_ref()
            .is_some_and(|(_, parked)| parked.elapsed() >= WARM_MODEL)
        {
            self.warm = None;
        }
        let before = self.session.sequence();
        let now = elapsed_ms(self.clock);
        let mut generation = None;
        if (self.provider.is_some() || self.prepared.is_some())
            && let Some(work) = self.session.next_request(now)
        {
            // An approved prepared answer is used at once, with no model call.
            let key = work.request.key;
            let chosen = self
                .chosen
                .take()
                .filter(|c| c.0 == key.question_id && c.1 == key.revision);
            let resolver = self.prepared.clone();
            let local = resolver.as_ref().and_then(|r| {
                let resolution = match &chosen {
                    Some((_, _, family)) => {
                        r.choose(&work.request.question, family, Length::Standard)?
                    }
                    None => r.resolve(&work.request.question, Length::Standard),
                };
                complete_from(&mut self.session, &work, resolution, now)
            });
            let semantic = resolver.filter(|r| r.semantic_model().is_some());
            if let Some(hit) = local {
                self.remember(key, hit);
            } else if self.provider.is_some() || semantic.is_some() {
                generation = Some(Generation {
                    route: self.provider.clone(),
                    work,
                    semantic,
                });
            } else {
                self.session
                    .complete(key, now, Err(AnswerFailure::NoPreparedMatch));
            }
        }
        (
            (self.session.sequence() != before || self.answer_dirty).then(|| self.changed()),
            generation,
        )
    }

    /// Appends streamed answer text. The 50 ms tick publishes it, so a fast stream costs at most
    /// 20 snapshots a second, not one per token.
    fn answer_delta(&mut self, key: AnswerKey, delta: &str) -> bool {
        let accepted = self.session.append_answer(key, delta);
        self.answer_dirty |= accepted;
        accepted
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
                if utterance.you {
                    self.session.ingest_side(utterance);
                    Ok(())
                } else {
                    self.next_utterance = self.next_utterance.max(utterance.id.saturating_add(1));
                    self.audio_metrics.decode_ms = Some(decode_ms);
                    self.audio_metrics.queue_ms = Some(queue_ms);
                    self.session.ingest(utterance)
                }
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
                    "Audio was interrupted for a moment. Review the affected transcript.",
                );
                Ok(())
            }
            AudioEvent::MicrophoneStopped { message } => {
                self.session.note_audio_gap(
                    Span::at(now),
                    0,
                    format!(
                        "Your microphone stopped ({message}) The call is still being transcribed."
                    ),
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

    fn finish_audio(
        &mut self,
        result: Result<(), AudioError>,
        model: Option<Recognizer>,
    ) -> Option<View> {
        let run = self.audio.take()?;
        // A clean pause parks the loaded model. A failed run reloads it: the model may be why.
        let paused = result.is_ok() && !run.failed && run.stopping == Some(CaptureState::Paused);
        self.warm = model
            .filter(|_| paused)
            .map(|model| (model, Instant::now()));
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
        if let Some(resolver) = &generation.semantic
            && let Some(view) =
                semantic_answer(&state, &providers, resolver, &generation.work).await
        {
            emit(&app, &view);
            return;
        }
        let Some(route) = generation.route.clone() else {
            let view = state
                .lock()
                .expect("desktop session lock")
                .finish_answer(key, Err(AnswerFailure::NoPreparedMatch));
            if let Some(view) = view {
                emit(&app, &view);
            }
            return;
        };
        let cancel = async {
            while !generation.work.is_cancelled() {
                tokio::time::sleep(Duration::from_millis(40)).await;
            }
        };
        let result = tokio::select! {
            biased;
            () = cancel => return,
            answer = providers.answer(&route, &generation.work.request, |delta| {
                let mut desktop = state.lock().expect("desktop session lock");
                !generation.work.is_cancelled() && desktop.answer_delta(key, delta)
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

/// Embeds the question with the pack's local model and retries the pack. Any failure or a
/// slow embedding falls through to the provider (or to "no prepared match").
async fn semantic_answer(
    state: &Shared,
    providers: &ProviderHub,
    resolver: &Resolver,
    work: &Work,
) -> Option<View> {
    let model = resolver.semantic_model()?;
    let question = &work.request.question;
    let norm = format!("{model}\n{}", words(question).join(" "));
    let cached = state
        .lock()
        .expect("desktop session lock")
        .query_vectors
        .get(&norm)
        .cloned();
    let vector = match cached {
        Some(v) => v,
        None => tokio::time::timeout(
            Duration::from_millis(1_500),
            providers.embed(model, std::slice::from_ref(question)),
        )
        .await
        .ok()?
        .ok()?
        .pop()?,
    };
    let resolution = resolver.resolve_with(question, Length::Standard, Some(&vector));
    let mut desktop = state.lock().expect("desktop session lock");
    if desktop.query_vectors.len() >= 64 {
        desktop.query_vectors.clear();
    }
    desktop.query_vectors.insert(norm, vector);
    let now = elapsed_ms(desktop.clock);
    let hit = complete_from(&mut desktop.session, work, resolution, now)?;
    desktop.remember(work.request.key, hit);
    Some(desktop.changed())
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
        let mut model = job.model;
        let result = (|| {
            if job.stop.requested() {
                return Ok(());
            }
            // A resumed session still holds its model. Anything else loads and verifies it.
            let loaded = match model.take() {
                Some(model) => Ok(model),
                None => copilot_audio::load(
                    job.config.engine,
                    Path::new(&job.config.model_path),
                    &job.config.vocabulary,
                    job.config.short_followups,
                ),
            };
            if job.stop.requested() {
                model = loaded.ok();
                return Ok(());
            }
            let recognizer = model.insert(loaded?);
            copilot_audio::capture(
                recognizer.as_mut(),
                job.config.source,
                job.config.microphone.map(|id| Source {
                    kind: SourceKind::Microphone,
                    endpoint_id: Some(id),
                }),
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
        })();
        let view = state
            .lock()
            .expect("desktop session lock")
            .finish_audio(result, model);
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
                microphone: None,
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
        let view = desktop.finish_audio(Ok(()), None).unwrap();
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
                you: false,
            },
            at_ms: 1_002,
            decode_ms: 2,
            queue_ms: 0,
        });
        let view = desktop
            .finish_audio(
                Err(AudioError {
                    message: "Device disconnected".into(),
                    dropped_ms: 30,
                    device: true,
                }),
                None,
            )
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
                    you: false,
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
    fn prepared_pack_answers_a_typed_question_without_a_provider() {
        use copilot_core::prep::{PackSettings, Workspace, community};
        let mut ws = Workspace::default();
        let pack = include_str!("../../../../community-packs/examples/java-backend.json");
        ws.install_community(community::parse(pack.as_bytes()).unwrap(), "java.json", 1)
            .unwrap();
        let settings = PackSettings {
            name: "Java".into(),
            community_ids: vec!["org.openkeet.examples.java-backend".into()],
            ..PackSettings::default()
        };
        let id = ws.compile_pack(None, settings, 2).unwrap();
        let shared = Desktop::shared();
        let mut desktop = shared.lock().unwrap();
        assert!(desktop.ask("When should I avoid Redis?").is_err());
        desktop.set_prepared(Resolver::build(&ws, &id).map(Arc::new));
        desktop.ask("When should I avoid Redis?").unwrap();
        let (view, generation) = desktop.tick();
        assert!(
            generation.is_none(),
            "no provider request for a prepared hit"
        );
        let view = view.unwrap();
        let answer = view.session.question.unwrap().answer.unwrap();
        assert!(answer.text.starts_with("Avoid a cache"));
        assert_eq!(view.prepared_answers[0].key, answer.key);
        assert_eq!(view.prepared_answers[0].label, "Community concept");
        // A heard question still taking shape: the user picks a listed interpretation.
        desktop.clear().unwrap();
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
                text: "Can you explain when to use Redis".into(),
                you: false,
            },
            at_ms: 300,
            decode_ms: 100,
            queue_ms: 0,
        });
        let pick = desktop
            .anticipation
            .as_ref()
            .expect("candidates while forming")
            .candidates[0]
            .family_id
            .clone();
        assert!(desktop.choose_candidate("not-listed").is_err());
        desktop.choose_candidate(&pick).unwrap();
        let (view, generation) = desktop.tick();
        assert!(generation.is_none());
        let question = view.unwrap().session.question.unwrap();
        assert_eq!(
            question.answer.unwrap().status,
            copilot_core::AnswerStatus::Checked
        );
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
                you: false,
            },
            at_ms: 300,
            decode_ms: 100,
            queue_ms: 0,
        });
        let error = AudioError {
            message: "Device disconnected".into(),
            dropped_ms: 30,
            device: true,
        };
        assert!(desktop.finish_audio(Err(error), None).unwrap().retryable);
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
        // A pause parks the loaded model for the next resume. Stop releases it.
        struct Probe;
        impl SpeechRecognizer for Probe {
            fn decode(&mut self, _: &[i16], _: bool) -> Result<String, AudioError> {
                Ok(String::new())
            }
        }
        desktop.stop(true).unwrap();
        desktop.finish_audio(Ok(()), Some(Box::new(Probe)));
        assert!(desktop.begin_audio(None).unwrap().model.is_some());
        desktop.stop(false).unwrap();
        desktop.finish_audio(Ok(()), Some(Box::new(Probe)));
        assert!(desktop.warm.is_none());
    }
}
