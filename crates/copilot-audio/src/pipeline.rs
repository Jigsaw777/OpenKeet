use std::{
    collections::VecDeque,
    path::Path,
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
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
            TrySendError::Full(_) => AudioError { message: "Capture backlog filled. Audio was interrupted. Restart and try a faster model.".into(), dropped_ms: FRAME_MS + dropped_ms.unwrap_or(0) },
            TrySendError::Disconnected(_) => AudioError::new("Audio processing stopped."),
        })
    }
}

struct Segment {
    samples: Vec<i16>,
    span: Span,
    quiet_frames: usize,
    interim_sent: bool,
}

struct Job {
    id: u64,
    samples: Vec<i16>,
    span: Span,
    queued_ms: u64,
    final_result: bool,
}

fn enqueue(
    sender: &SyncSender<Job>,
    id: u64,
    segment: Segment,
    clock: Instant,
) -> Result<(), AudioError> {
    let span = segment.span;
    sender.try_send(Job { id, samples: segment.samples, span, queued_ms: elapsed_ms(clock), final_result: true })
        .map_err(|error| match error {
            TrySendError::Full(_) => AudioError { message: "Recognition backlog filled. A completed utterance was lost. Restart and try a faster model.".into(), dropped_ms: span.end_ms.saturating_sub(span.start_ms) },
            TrySendError::Disconnected(_) => AudioError::new("Local speech decoding stopped."),
        })
}

fn segment(
    frames: Receiver<Frame>,
    jobs: SyncSender<Job>,
    clock: Instant,
    options: StreamOptions,
    report: &impl Fn(AudioEvent),
) -> Result<(), AudioError> {
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
        if let Some(current) = &mut active {
            current.samples.extend_from_slice(&frame.samples);
            if voice {
                current.span.end_ms = frame.span.end_ms;
                current.quiet_frames = 0;
            } else {
                current.quiet_frames += 1;
            }
            if current.quiet_frames >= POST_ROLL || current.samples.len() >= MAX_SEGMENT * SAMPLES {
                let mut ended = active.take().expect("active segment");
                ended.samples.truncate(
                    ended
                        .samples
                        .len()
                        .saturating_sub(ended.quiet_frames.saturating_sub(4) * SAMPLES),
                );
                enqueue(&jobs, id, ended, clock)?;
                id += 1;
                consecutive_voice = 0;
            } else if options.interim
                && !current.interim_sent
                && voice
                && current.span.end_ms.saturating_sub(current.span.start_ms) >= 1_400
            {
                current.interim_sent = true;
                // One optional preview per utterance. A full decoder queue only rejects the preview.
                let _ = jobs.try_send(Job {
                    id,
                    samples: current.samples.clone(),
                    span: current.span,
                    queued_ms: elapsed_ms(clock),
                    final_result: false,
                });
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
                    interim_sent: false,
                });
                report(AudioEvent::Speech {
                    utterance_id: id,
                    span,
                });
            }
        }
    }
    // The producer closes at the first capture error. PCM already in the queue is usable.
    if let Some(ended) = active {
        enqueue(&jobs, id, ended, clock)?;
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
) -> Result<(), AudioError> {
    let (frame_tx, frame_rx) = mpsc::sync_channel(FRAME_QUEUE);
    let (job_tx, job_rx) = mpsc::sync_channel(DECODE_QUEUE);
    thread::scope(|scope| {
        let capture = scope.spawn(move || feed(&Producer(frame_tx)));
        let segment = scope.spawn(move || {
            let result = segment(frame_rx, job_tx, clock, options, report);
            if result.is_err() {
                stop.request();
            }
            result
        });
        let decode = decode(model, job_rx, clock, report);
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
    clock: Instant,
    report: &impl Fn(AudioEvent),
) -> Result<(), AudioError> {
    for job in jobs {
        let started_ms = elapsed_ms(clock);
        let text = model.decode(&job.samples, job.final_result)?;
        let at_ms = elapsed_ms(clock);
        if job.final_result || !text.is_empty() {
            let utterance = Utterance {
                id: job.id,
                revision: if job.final_result { 2 } else { 1 },
                span: job.span,
                stability: if job.final_result {
                    Stability::Final
                } else {
                    Stability::Interim
                },
                text,
            };
            report(AudioEvent::Transcript {
                utterance,
                at_ms,
                decode_ms: at_ms.saturating_sub(started_ms),
                queue_ms: started_ms.saturating_sub(job.queued_ms),
            });
        }
    }
    Ok(())
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
                })
            },
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
