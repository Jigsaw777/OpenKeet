use std::{
    ffi::c_void,
    io::{self, Read},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use sha2::{Digest, Sha256};

use crate::open_locked;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperSegment,
    WhisperState,
};

use crate::{AudioError, SAMPLE_RATE, SpeechRecognizer};

#[derive(Clone, Copy)]
pub struct Model {
    pub filename: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
}

pub const MODELS: &[Model] = &[
    Model {
        filename: "ggml-small.en.bin",
        bytes: 487_614_201,
        sha256: "c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d",
    },
    Model {
        filename: "ggml-large-v3-turbo-q5_0.bin",
        bytes: 574_041_195,
        sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
    },
];

pub struct LocalWhisper {
    state: WhisperState,
    threads: i32,
    vocabulary: String,
    short_followups: bool,
    final_waiting: Option<Arc<AtomicBool>>,
}

/// Whisper polls this after the encoder pass and after every decoder step, and stops on true.
unsafe extern "C" fn final_waiting(flag: *mut c_void) -> bool {
    // SAFETY: `flag` is the AtomicBool that `LocalWhisper::final_waiting` keeps alive during the decode.
    unsafe { &*flag.cast::<AtomicBool>() }.load(Ordering::Acquire)
}

impl LocalWhisper {
    pub fn load(path: &Path, vocabulary: &str) -> Result<Self, AudioError> {
        let spec = MODELS
            .iter()
            .find(|m| path.file_name().is_some_and(|name| name == m.filename))
            .ok_or_else(|| AudioError::new("Choose a supported local Whisper model file."))?;
        if vocabulary.len() > 300 || vocabulary.contains('\0') {
            return Err(AudioError::new(
                "Enter up to 300 UTF-8 bytes of speech vocabulary.",
            ));
        }
        let invalid = || {
            AudioError::new(
                "Model file is missing or failed verification. Check its download and checksum.",
            )
        };
        let file = open_locked(path).map_err(|_| invalid())?;
        if file.metadata().map_err(|_| invalid())?.len() != spec.bytes {
            return Err(invalid());
        }
        whisper_rs::install_logging_hooks();
        let mut config = WhisperContextParameters::default();
        config.use_gpu(false);
        // flash_attn was measured: 92 MiB less commit but CER 5.0% -> 6.9% on the profile pack.
        let failed = |_| AudioError::new("Local Whisper could not load the verified model.");
        // The C loader opens ASCII paths only on Windows. For those, hash through the locked
        // handle and let Whisper stream the same, unchangeable file: the model is never held
        // twice in memory. Other paths parse a verified in-memory copy, as before.
        let context = if path.to_str().is_some_and(str::is_ascii) {
            let mut hash = Sha256::new();
            let read =
                io::copy(&mut (&file).take(spec.bytes + 1), &mut hash).map_err(|_| invalid())?;
            if read != spec.bytes || format!("{:x}", hash.finalize()) != spec.sha256 {
                return Err(invalid());
            }
            let context = WhisperContext::new_with_params(path, config).map_err(failed)?;
            drop(file);
            context
        } else {
            let mut bytes = Vec::with_capacity(spec.bytes as usize);
            file.take(spec.bytes + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| invalid())?;
            if bytes.len() as u64 != spec.bytes
                || format!("{:x}", Sha256::digest(&bytes)) != spec.sha256
            {
                return Err(invalid());
            }
            WhisperContext::new_from_buffer_with_params(&bytes, config).map_err(failed)?
        };
        let state = context
            .create_state()
            .map_err(|_| AudioError::new("Local Whisper could not allocate decoder state."))?;
        let threads = std::thread::available_parallelism().map_or(2, |n| n.get().min(6)) as i32;
        Ok(Self {
            state,
            threads,
            vocabulary: vocabulary.to_owned(),
            short_followups: false,
            final_waiting: None,
        })
    }

    pub fn enable_short_followups(&mut self) {
        self.short_followups = true;
    }
}

fn words(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_alphanumeric() && ch != '\'')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Whisper turns music, a hum or a noise into "♪", "...", "GONG!" or "The End". A segment with
/// no word, one word said five times or more, or a few words Whisper itself doubts is not speech.
/// On the speech test clips no real segment averaged below -1.3 log-probability a token; these
/// fell between -1.7 and -2.8.
fn invented(segment: &WhisperSegment) -> bool {
    let text = segment.to_str().unwrap_or_default();
    let said = words(text);
    let tokens = segment.n_tokens();
    let logp = (0..tokens)
        .filter_map(|i| segment.get_token(i))
        .map(|token| token.token_probability().max(1e-9).ln())
        .sum::<f32>()
        / tokens.max(1) as f32;
    said.is_empty()
        || (said.len() >= 5 && said.iter().all(|word| *word == said[0]))
        || (tokens <= 6 && logp < -1.5)
}

/// Joins Whisper's segments, leaving out a segment that repeats the end of the one before it.
/// On a short clip Whisper can decode the last sentence twice, the second time as a new segment.
// ponytail: a speaker who really says the same sentence twice in a row loses the repeat.
fn without_echoes(segments: Vec<String>) -> String {
    let mut kept: Vec<String> = Vec::new();
    for segment in segments {
        let new = words(&segment);
        if let Some(last) = kept.last() {
            let before = words(last);
            let tail = &before[before.len().saturating_sub(new.len() + 2)..];
            if new.len() >= 3 && in_order(&new, tail) * 5 >= new.len() * 4 {
                continue;
            }
        }
        kept.push(segment);
    }
    kept.join(" ")
}

/// How many words of `new` appear in `old` in the same order (longest common subsequence).
fn in_order(new: &[String], old: &[String]) -> usize {
    let mut row = vec![0; old.len() + 1];
    for word in new {
        let mut diagonal = 0;
        for (j, other) in old.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if word == other {
                diagonal + 1
            } else {
                above.max(row[j])
            };
            diagonal = above;
        }
    }
    row[old.len()]
}

impl SpeechRecognizer for LocalWhisper {
    fn decode(&mut self, samples: &[i16], final_result: bool) -> Result<String, AudioError> {
        let peak = samples
            .iter()
            .map(|sample| sample.unsigned_abs())
            .max()
            .unwrap_or(0)
            .max(1);
        let gain = (7_000.0 / f32::from(peak)).clamp(1.0, 16.0);
        let signal: Vec<f32> = samples
            .iter()
            .map(|sample| f32::from(*sample) * gain / 32_768.0)
            .collect();
        let short_followup = self.short_followups && samples.len() <= SAMPLE_RATE as usize * 5 / 4;
        let strategy = if final_result && !short_followup {
            SamplingStrategy::BeamSearch {
                beam_size: 3,
                patience: -1.0,
            }
        } else {
            SamplingStrategy::Greedy { best_of: 1 }
        };
        let mut params = FullParams::new(strategy);
        params.set_n_threads(self.threads);
        params.set_language(Some("en"));
        params.set_no_context(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_nst(true);
        params.set_temperature_inc(0.0);
        // 320 samples per encoder step. Keep headroom after the last captured frame.
        params.set_audio_ctx((samples.len().div_ceil(320) + 32).clamp(192, 1500) as i32);
        if !self.vocabulary.is_empty() {
            params.set_initial_prompt(&self.vocabulary);
        }
        let seconds = samples.len() as f32 / SAMPLE_RATE as f32;
        if !final_result {
            // A greedy pass over a sentence cut mid-way can loop on one phrase for seconds.
            // A preview gets one pass with a token budget of about twice normal speech.
            params.set_single_segment(true);
            params.set_max_tokens((seconds * 8.0) as i32 + 16);
        }
        // A preview gives way at Whisper's next check once a finished utterance needs the decoder.
        let yields = self.final_waiting.as_ref().filter(|_| !final_result);
        if let Some(waiting) = yields {
            // SAFETY: the callback only reads the flag, which `self` owns for the whole call.
            unsafe {
                params.set_abort_callback(Some(final_waiting));
                params.set_abort_callback_user_data(Arc::as_ptr(waiting).cast_mut().cast());
            }
        }
        if self.state.full(params, &signal).is_err() {
            if yields.is_some_and(|waiting| waiting.load(Ordering::Acquire)) {
                return Ok(String::new());
            }
            return Err(AudioError::new(
                "Local speech decoding failed. Restart capture or choose another model.",
            ));
        }
        let segments: Vec<String> = self
            .state
            .as_iter()
            .filter(|segment| !invented(segment))
            .filter_map(|segment| segment.to_str().ok().map(str::trim).map(str::to_owned))
            .filter(|text| !text.is_empty())
            .collect();
        let text = without_echoes(segments);
        // More than five words a second is a decoder loop, not speech. The last preview stays.
        if !final_result && text.split_whitespace().count() as f32 > seconds * 5.0 + 5.0 {
            return Ok(String::new());
        }
        Ok(text)
    }

    fn yield_previews_to(&mut self, final_waiting: Arc<AtomicBool>) {
        self.final_waiting = Some(final_waiting);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_repeated_last_sentence_is_dropped() {
        let segments = |list: &[&str]| list.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            super::without_echoes(segments(&[
                "Imagine a table in PostgreSQL with 200 million rows",
                "and a table in PostgreSQL with 200 million rows.",
                "Which index would you create?",
                "Which tools did you use?",
            ])),
            "Imagine a table in PostgreSQL with 200 million rows Which index would you create? Which tools did you use?"
        );
        // The same words in a new order say something new.
        assert_eq!(
            super::without_echoes(segments(&[
                "We need a cache for the users.",
                "For the users, cache it."
            ])),
            "We need a cache for the users. For the users, cache it."
        );
    }
}
