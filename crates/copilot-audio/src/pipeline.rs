use std::{
    collections::VecDeque,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};

use copilot_core::{Source, Span, Stability, Utterance};
use webrtc_vad::{SampleRate, Vad, VadMode};

use crate::{
    AudioError, AudioEvent, FRAME_MS, SAMPLE_RATE, SAMPLES, SpeechRecognizer, Stop, elapsed_ms,
};

const FRAME_QUEUE: usize = 64;
const DECODE_QUEUE: usize = 8;
const PRE_ROLL: usize = 8;
const POST_ROLL: usize = 18;
const MAX_SEGMENT: usize = 800;
/// Past 15 s of unbroken speech, end the utterance at the first short pause, not mid-word at the cap.
/// The detector holds "voice" for about 270 ms after speech, so two quiet frames are a 330 ms pause.
const SOFT_SEGMENT: usize = 500;
const SOFT_PAUSE: usize = 2;
const FIRST_PREVIEW_MS: u64 = 1_400;
/// A shorter microphone sound is a click or a cough, not a line worth keeping.
const MIN_SIDE_MS: u64 = 600;

#[derive(Clone, Copy)]
pub struct StreamOptions {
    pub first_id: u64,
    pub interim: bool,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            first_id: 1,
            interim: true,
        }
    }
}

#[derive(Clone, Copy)]
struct Frame {
    samples: [i16; SAMPLES],
    span: Span,
    dropped_ms: Option<u64>,
}

pub(crate) struct Producer(SyncSender<Frame>);

impl Producer {
    pub(crate) fn push(
        &self,
        samples: [i16; SAMPLES],
        span: Span,
        dropped_ms: Option<u64>,
    ) -> Result<(), AudioError> {
        self.0.try_send(Frame { samples, span, dropped_ms }).map_err(|error| match error {
            TrySendError::Full(_) => AudioError { message: "Capture backlog filled. Audio was interrupted. Restart and try a faster model.".into(), dropped_ms: FRAME_MS + dropped_ms.unwrap_or(0), device: false },
            TrySendError::Disconnected(_) => AudioError::new("Audio processing stopped."),
        })
    }
}

struct Segment {
    samples: Vec<i16>,
    span: Span,
    quiet_frames: usize,
    /// Loudest frame so far.
    peak: u16,
    /// Frames heard while the call itself was speaking. Microphone lane only.
    over_call: usize,
    next_preview_ms: u64,
    previews: u64,
}

/// Decoder load the segmenter reads: previews go only to an idle decoder and yield to a final.
#[derive(Default)]
struct Load {
    jobs: AtomicUsize,
    final_waiting: Arc<AtomicBool>,
    /// The session's main source has voice right now.
    call_voice: AtomicBool,
}

struct Job {
    id: u64,
    revision: u64,
    samples: Vec<i16>,
    span: Span,
    queued_ms: u64,
    final_result: bool,
    /// From the user's microphone lane, not the session's main source.
    you: bool,
    retried: bool,
}

/// Feeds the optional microphone lane until the given flag asks it to stop.
pub(crate) type SideFeed<'a> =
    Box<dyn FnOnce(&Producer, &Stop) -> Result<(), AudioError> + Send + 'a>;

fn enqueue(
    sender: &SyncSender<Job>,
    load: &Load,
    id: u64,
    segment: Segment,
    clock: Instant,
    you: bool,
) -> Result<(), AudioError> {
    let span = segment.span;
    // ponytail: without echo cancellation, a microphone sound that mostly coincides with the
    // call's speech is taken for the speakers leaking in and dropped, as are short blips. Talking
    // over the interviewer is lost with it. Real echo cancellation if that matters.
    if you
        && (span.end_ms.saturating_sub(span.start_ms) < MIN_SIDE_MS
            || segment.over_call * 2 > segment.samples.len() / SAMPLES)
    {
        return Ok(());
    }
    // Only the call's finished speech interrupts previews.
    if !you {
        load.final_waiting.store(true, Ordering::Release);
    }
    load.jobs.fetch_add(1, Ordering::AcqRel);
    let job = Job {
        id,
        revision: segment.previews + 1,
        samples: segment.samples,
        span,
        queued_ms: elapsed_ms(clock),
        final_result: true,
        you,
        retried: false,
    };
    let Err(error) = sender.try_send(job) else {
        return Ok(());
    };
    load.jobs.fetch_sub(1, Ordering::AcqRel);
    match error {
        // A lost microphone line is not worth ending the session.
        TrySendError::Full(_) if you => Ok(()),
        TrySendError::Full(_) => Err(AudioError { message: "Recognition backlog filled. A completed utterance was lost. Restart and try a faster model.".into(), dropped_ms: span.end_ms.saturating_sub(span.start_ms), device: false }),
        TrySendError::Disconnected(_) => Err(AudioError::new("Local speech decoding stopped.")),
    }
}

fn segment(
    frames: Receiver<Frame>,
    jobs: SyncSender<Job>,
    load: &Load,
    clock: Instant,
    options: StreamOptions,
    you: bool,
    report: &impl Fn(AudioEvent),
) -> Result<(), AudioError> {
    // The microphone lane reports only its finished lines: one meter, one speech state.
    let report = |event| {
        if !you {
            report(event)
        }
    };
    let mut vad = Vad::new_with_rate_and_mode(SampleRate::Rate16kHz, VadMode::Quality);
    let mut preroll = VecDeque::with_capacity(PRE_ROLL);
    let mut active: Option<Segment> = None;
    let mut id = options.first_id;
    let mut consecutive_voice = 0;
    let mut peak = 0;
    let mut count = 0;
    for frame in frames {
        if let Some(dropped_ms) = frame.dropped_ms {
            report(AudioEvent::Discontinuity {
                span: frame.span,
                dropped_ms,
            });
        }
        let frame_peak = frame
            .samples
            .iter()
            .map(|sample| sample.unsigned_abs())
            .max()
            .unwrap_or(0);
        peak = peak.max(frame_peak);
        count += 1;
        if count % 5 == 0 {
            report(AudioEvent::Meter {
                peak,
                frames: count,
            });
            peak = 0;
        }
        // WebRTC adapts its voice/noise model. Raise quiet PCM within a fixed gain ceiling.
        let boosted = frame.samples.map(|value| value.saturating_mul(4));
        let voice = vad
            .is_voice_segment(&boosted)
            .map_err(|_| AudioError::new("Voice detector rejected the audio format."))?;
        if !you {
            load.call_voice.store(voice, Ordering::Release);
        }
        if let Some(current) = &mut active {
            current.samples.extend_from_slice(&frame.samples);
            current.peak = current.peak.max(frame_peak);
            current.over_call += usize::from(you && load.call_voice.load(Ordering::Acquire));
            if voice {
                current.span.end_ms = frame.span.end_ms;
                current.quiet_frames = 0;
            } else {
                current.quiet_frames += 1;
            }
            let frames = current.samples.len() / SAMPLES;
            let pause = if frames >= SOFT_SEGMENT {
                SOFT_PAUSE
            } else {
                POST_ROLL
            };
            if current.quiet_frames >= pause || frames >= MAX_SEGMENT {
                let mut ended = active.take().expect("active segment");
                ended.samples.truncate(
                    ended
                        .samples
                        .len()
                        .saturating_sub(ended.quiet_frames.saturating_sub(4) * SAMPLES),
                );
                enqueue(&jobs, load, id, ended, clock, you)?;
                id += 1;
                consecutive_voice = 0;
            } else if options.interim
                && voice
                && frame_peak >= current.peak / 4
                && current.span.end_ms.saturating_sub(current.span.start_ms)
                    >= current.next_preview_ms
                && load.jobs.load(Ordering::Acquire) == 0
            {
                // Rolling previews for an idle decoder. Each waits for the speech to double, so
                // all of an utterance's previews cost about two greedy passes over it. A preview
                // starts on a loud frame only: the detector still says "voice" for a moment after
                // speech stops, and a preview begun there would hold up the final.
                let spoken = current.span.end_ms - current.span.start_ms;
                current.next_preview_ms = (spoken * 2).max(spoken + 2_500);
                current.previews += 1;
                load.jobs.fetch_add(1, Ordering::AcqRel);
                let preview = Job {
                    id,
                    revision: current.previews,
                    samples: current.samples.clone(),
                    span: current.span,
                    queued_ms: elapsed_ms(clock),
                    final_result: false,
                    you,
                    retried: false,
                };
                if jobs.try_send(preview).is_err() {
                    load.jobs.fetch_sub(1, Ordering::AcqRel);
                }
            }
        } else {
            if preroll.len() == PRE_ROLL {
                preroll.pop_front();
            }
            preroll.push_back(frame);
            consecutive_voice = if voice { consecutive_voice + 1 } else { 0 };
            if consecutive_voice >= 3 {
                let span = Span {
                    start_ms: preroll[preroll.len() - consecutive_voice].span.start_ms,
                    end_ms: frame.span.end_ms,
                };
                active = Some(Segment {
                    span,
                    samples: preroll.drain(..).flat_map(|frame| frame.samples).collect(),
                    quiet_frames: 0,
                    peak: frame_peak,
                    over_call: 0,
                    next_preview_ms: FIRST_PREVIEW_MS,
                    previews: 0,
                });
                report(AudioEvent::Speech {
                    utterance_id: id,
                    span,
                });
            }
        }
    }
    if !you {
        load.call_voice.store(false, Ordering::Release);
    }
    // The producer closes at the first capture error. PCM already in the queue is usable.
    if let Some(ended) = active {
        enqueue(&jobs, load, id, ended, clock, you)?;
    }
    if count % 5 != 0 {
        report(AudioEvent::Meter {
            peak,
            frames: count,
        });
    }
    Ok(())
}

pub(crate) fn run(
    model: &mut dyn SpeechRecognizer,
    clock: Instant,
    options: StreamOptions,
    stop: &Stop,
    report: &(impl Fn(AudioEvent) + Sync),
    feed: impl FnOnce(&Producer) -> Result<(), AudioError> + Send,
    side: Option<SideFeed<'_>>,
) -> Result<(), AudioError> {
    let (frame_tx, frame_rx) = mpsc::sync_channel(FRAME_QUEUE);
    let (job_tx, job_rx) = mpsc::sync_channel(DECODE_QUEUE);
    let load = &Load::default();
    model.yield_previews_to(Arc::clone(&load.final_waiting));
    // Set once the main source ends, for any reason: the microphone lane then ends too.
    let halt = &Stop::default();
    thread::scope(|scope| {
        if let Some(side) = side {
            let (side_tx, side_rx) = mpsc::sync_channel(FRAME_QUEUE);
            let jobs = job_tx.clone();
            // The microphone lane never ends the session. A failure there becomes a notice.
            scope.spawn(move || {
                if let Err(error) = side(&Producer(side_tx), halt) {
                    report(AudioEvent::MicrophoneStopped {
                        message: error.message,
                    });
                }
            });
            scope.spawn(move || {
                let quiet = StreamOptions {
                    interim: false,
                    ..options
                };
                let _ = segment(side_rx, jobs, load, clock, quiet, true, report);
            });
        }
        let capture = scope.spawn(move || {
            let result = feed(&Producer(frame_tx));
            halt.request();
            result
        });
        let segment = scope.spawn(move || {
            let result = segment(frame_rx, job_tx, load, clock, options, false, report);
            if result.is_err() {
                stop.request();
            }
            result
        });
        let decode = decode(model, job_rx, load, clock, report);
        if decode.is_err() {
            stop.request();
        }
        let segment = segment
            .join()
            .map_err(|_| AudioError::new("Voice detector worker failed."))?;
        let capture = capture
            .join()
            .map_err(|_| AudioError::new("Capture worker failed."))?;
        decode.and(segment).and(capture)
    })
}

fn decode(
    model: &mut dyn SpeechRecognizer,
    jobs: Receiver<Job>,
    load: &Load,
    clock: Instant,
    report: &impl Fn(AudioEvent),
) -> Result<(), AudioError> {
    let mut pending = VecDeque::new();
    loop {
        pending.extend(jobs.try_iter());
        if pending.is_empty() {
            let Ok(job) = jobs.recv() else {
                return Ok(());
            };
            pending.push_back(job);
        }
        // The call's speech is decoded before the user's own microphone.
        let next = pending.iter().position(|job: &Job| !job.you).unwrap_or(0);
        let mut job = pending.remove(next).expect("a pending job");
        let call_final = job.final_result && !job.you;
        if call_final {
            load.final_waiting.store(false, Ordering::Release);
        }
        let started_ms = elapsed_ms(clock);
        // A microphone line decodes like a preview: one bounded pass that gives way to the call.
        let text = model.decode(&job.samples, call_final);
        if text.as_ref().is_ok_and(String::is_empty)
            && job.you
            && !job.retried
            && load.final_waiting.load(Ordering::Acquire)
        {
            // It gave way to the call's finished speech. Decode it once more afterwards.
            job.retried = true;
            pending.push_back(job);
            continue;
        }
        load.jobs.fetch_sub(1, Ordering::AcqRel);
        let text = text?;
        let at_ms = elapsed_ms(clock);
        if call_final || !text.is_empty() {
            let utterance = Utterance {
                id: job.id,
                revision: job.revision,
                span: job.span,
                stability: if job.final_result {
                    Stability::Final
                } else {
                    Stability::Interim
                },
                text,
                you: job.you,
            };
            report(AudioEvent::Transcript {
                utterance,
                at_ms,
                decode_ms: at_ms.saturating_sub(started_ms),
                queue_ms: started_ms.saturating_sub(job.queued_ms),
            });
        }
    }
}

pub fn recognize_wav(
    model: &mut dyn SpeechRecognizer,
    path: &Path,
    report: impl Fn(AudioEvent) + Sync,
) -> Result<(), AudioError> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|_| AudioError::new("Could not open a PCM WAV file."))?;
    let spec = reader.spec();
    if spec.sample_rate != SAMPLE_RATE
        || spec.channels != 1
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return Err(AudioError::new("Choose a mono, 16-bit, 16 kHz PCM WAV."));
    }
    let stop = Stop::default();
    let clock = Instant::now();
    run(
        model,
        clock,
        StreamOptions::default(),
        &stop,
        &report,
        |producer| {
            report(AudioEvent::Ready {
                source: Source::REPLAY,
            });
            let mut samples = reader.samples::<i16>();
            let mut start_ms = 0;
            while !stop.requested() {
                let mut block = [0; SAMPLES];
                let mut count = 0;
                for (dst, sample) in block.iter_mut().zip(&mut samples) {
                    *dst = sample
                        .map_err(|_| AudioError::new("WAV data is invalid or incomplete."))?;
                    count += 1;
                }
                if count == 0 {
                    break;
                }
                let end_ms = start_ms + FRAME_MS;
                thread::sleep(
                    (clock + Duration::from_millis(end_ms))
                        .saturating_duration_since(Instant::now()),
                );
                producer.push(block, Span { start_ms, end_ms }, None)?;
                start_ms = end_ms;
            }
            Ok(())
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_producer_reports_saturation_without_waiting_for_reader() {
        let (tx, _rx) = mpsc::sync_channel(1);
        let producer = Producer(tx);
        producer.push([0; SAMPLES], Span::at(0), None).unwrap();
        let error = producer.push([0; SAMPLES], Span::at(30), None).unwrap_err();
        assert_eq!(error.dropped_ms, 30);
    }

    /// A voiced 30 ms frame with syllable-rate loudness changes, so the detector keeps hearing speech.
    fn voiced(n: u64) -> [i16; SAMPLES] {
        std::array::from_fn(|i| {
            let t = (n as usize * SAMPLES + i) as f32;
            let phase = t * 0.081;
            let loudness = 1_800.0 * (0.7 + 0.3 * (t * 0.0016).sin());
            (loudness * (phase.sin() + 0.4 * (2.0 * phase).sin())) as i16
        })
    }

    #[test]
    fn long_speech_gets_rolling_previews_and_ends_at_a_short_pause() {
        struct Probe;
        impl SpeechRecognizer for Probe {
            fn decode(&mut self, samples: &[i16], _: bool) -> Result<String, AudioError> {
                Ok((samples.len() / SAMPLES).to_string())
            }
        }

        let seen = std::sync::Mutex::new(Vec::new());
        let notices = std::sync::Mutex::new(Vec::new());
        run(
            &mut Probe,
            Instant::now(),
            StreamOptions::default(),
            &Stop::default(),
            &|event| match event {
                AudioEvent::Transcript { utterance, .. } => seen.lock().unwrap().push(utterance),
                AudioEvent::MicrophoneStopped { message } => notices.lock().unwrap().push(message),
                _ => {}
            },
            |producer| {
                // 15.3 s of speech, a 360 ms pause, then more speech.
                for n in 0..560 {
                    let samples = if (510..522).contains(&n) {
                        [0; SAMPLES]
                    } else {
                        voiced(n)
                    };
                    let span = Span {
                        start_ms: n * FRAME_MS,
                        end_ms: (n + 1) * FRAME_MS,
                    };
                    producer.push(samples, span, None)?;
                    if n % 32 == 31 {
                        thread::sleep(Duration::from_millis(5));
                    }
                }
                Ok(())
            },
            // The microphone lane speaks once the call is quiet, then its device fails.
            Some(Box::new(|producer, call_ended| {
                while !call_ended.requested() {
                    thread::sleep(Duration::from_millis(5));
                }
                thread::sleep(Duration::from_millis(100));
                for n in 0..30 {
                    let span = Span {
                        start_ms: n * FRAME_MS,
                        end_ms: (n + 1) * FRAME_MS,
                    };
                    producer.push(voiced(n), span, None)?;
                }
                Err(AudioError::new("Microphone unplugged"))
            })),
        )
        .unwrap();
        // The microphone's line arrived labelled, and its failure ended nothing.
        let (mine, seen): (Vec<_>, Vec<_>) =
            seen.into_inner().unwrap().into_iter().partition(|u| u.you);
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].stability, copilot_core::Stability::Final);
        assert_eq!(notices.into_inner().unwrap(), ["Microphone unplugged"]);
        let first: Vec<_> = seen.iter().filter(|u| u.id == 1).collect();
        assert!(first.len() >= 3, "at least two previews before the final");
        assert!(first.iter().map(|u| u.revision).eq(1..=first.len() as u64));
        let last = first.last().unwrap();
        assert_eq!(last.stability, copilot_core::Stability::Final);
        // Ended at the pause, well before the 24 s cap.
        assert!((510 * FRAME_MS..525 * FRAME_MS).contains(&last.span.end_ms));
        assert!(seen.iter().any(|u| u.id == 2 && u.revision == 1));
    }

    #[test]
    fn a_device_fault_reopens_the_endpoint_then_falls_back_to_the_default() {
        let connected = AtomicBool::new(false);
        let fault = || Err(AudioError::device("Device disconnected"));
        let source = Source {
            kind: copilot_core::SourceKind::SystemOutput,
            endpoint_id: Some("headset".into()),
        };
        // A device that never delivered audio is not retried.
        let mut calls = 0;
        let never = crate::reconnecting(source.clone(), &Stop::default(), &connected, |_, _| {
            calls += 1;
            fault()
        });
        assert!(never.is_err());
        assert_eq!(calls, 1);
        // One that worked is reopened three times, then the default device takes over.
        let mut opened = Vec::new();
        crate::reconnecting(source, &Stop::default(), &connected, |source, lost_ms| {
            opened.push((source.endpoint_id.clone(), lost_ms.is_some()));
            match opened.len() {
                1 => {
                    connected.store(true, Ordering::Release);
                    fault()
                }
                2..=4 => fault(),
                _ => Ok(()),
            }
        })
        .unwrap();
        assert_eq!(opened.len(), 5);
        assert_eq!(opened[0], (Some("headset".into()), false));
        assert_eq!(opened[3], (Some("headset".into()), true));
        assert_eq!(opened[4], (None, true));
    }

    #[test]
    fn failed_input_drains_queued_speech_before_returning_the_error() {
        struct Probe;
        impl SpeechRecognizer for Probe {
            fn decode(
                &mut self,
                samples: &[i16],
                final_result: bool,
            ) -> Result<String, AudioError> {
                assert!(final_result);
                assert_eq!(samples.len(), 12 * SAMPLES);
                Ok("Tail preserved".into())
            }
        }

        let finals = std::sync::Mutex::new(Vec::new());
        let error = run(
            &mut Probe,
            Instant::now(),
            StreamOptions::default(),
            &Stop::default(),
            &|event| {
                if let AudioEvent::Transcript { utterance, .. } = event {
                    finals.lock().unwrap().push(utterance);
                }
            },
            |producer| {
                for n in 0..12 {
                    let samples = std::array::from_fn(|i| {
                        let phase = (n as usize * SAMPLES + i) as f32 * 0.081;
                        (1_800.0 * (phase.sin() + 0.4 * (2.0 * phase).sin())) as i16
                    });
                    producer.push(
                        samples,
                        Span {
                            start_ms: n * FRAME_MS,
                            end_ms: (n + 1) * FRAME_MS,
                        },
                        None,
                    )?;
                }
                Err(AudioError {
                    message: "Device disconnected".into(),
                    dropped_ms: 30,
                    device: true,
                })
            },
            None,
        )
        .unwrap_err();
        assert_eq!(error.dropped_ms, 30);
        assert_eq!(error.message, "Device disconnected");
        let finals = finals.into_inner().unwrap();
        assert_eq!(finals.len(), 1);
        assert_eq!(finals[0].text, "Tail preserved");
        assert_eq!(finals[0].span.end_ms, 12 * FRAME_MS);
    }
}
