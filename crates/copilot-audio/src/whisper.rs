use std::{fs::File, io::Read, path::Path};

use sha2::{Digest, Sha256};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
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
        let file = File::open(path).map_err(|_| invalid())?;
        if file.metadata().map_err(|_| invalid())?.len() != spec.bytes {
            return Err(invalid());
        }
        // Parse the exact bytes checked here, including on a replaced path.
        let mut bytes = Vec::with_capacity(spec.bytes as usize);
        file.take(spec.bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid())?;
        if bytes.len() as u64 != spec.bytes
            || format!("{:x}", Sha256::digest(&bytes)) != spec.sha256
        {
            return Err(invalid());
        }
        whisper_rs::install_logging_hooks();
        let mut config = WhisperContextParameters::default();
        config.use_gpu(false);
        let context = WhisperContext::new_from_buffer_with_params(&bytes, config)
            .map_err(|_| AudioError::new("Local Whisper could not load the verified model."))?;
        drop(bytes);
        let state = context
            .create_state()
            .map_err(|_| AudioError::new("Local Whisper could not allocate decoder state."))?;
        let threads = std::thread::available_parallelism().map_or(2, |n| n.get().min(6)) as i32;
        Ok(Self {
            state,
            threads,
            vocabulary: vocabulary.to_owned(),
            short_followups: false,
        })
    }

    pub fn enable_short_followups(&mut self) {
        self.short_followups = true;
    }
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
        self.state.full(params, &signal).map_err(|_| {
            AudioError::new(
                "Local speech decoding failed. Restart capture or choose another model.",
            )
        })?;
        let text = self
            .state
            .as_iter()
            .filter_map(|segment| segment.to_str().ok().map(str::trim).map(str::to_owned))
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        Ok(text)
    }
}
