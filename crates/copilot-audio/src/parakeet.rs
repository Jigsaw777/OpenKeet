use std::{
    fs::{self, File},
    io,
    path::Path,
};

use sha2::{Digest, Sha256};
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig};

use crate::{AudioError, Model, SAMPLE_RATE, SpeechRecognizer};

/// sherpa-onnx INT8 export of NVIDIA Parakeet TDT 0.6B v3, Hugging Face revision `PARAKEET_REVISION`.
pub const PARAKEET_FOLDER: &str = "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
pub const PARAKEET_REVISION: &str = "2bda32ec70b097a55adaa07d9a7173915b43cc78";
pub const PARAKEET_FILES: &[Model] = &[
    Model {
        filename: "encoder.int8.onnx",
        bytes: 652_184_281,
        sha256: "acfc2b4456377e15d04f0243af540b7fe7c992f8d898d751cf134c3a55fd2247",
    },
    Model {
        filename: "decoder.int8.onnx",
        bytes: 11_845_275,
        sha256: "179e50c43d1a9de79c8a24149a2f9bac6eb5981823f2a2ed88d655b24248db4e",
    },
    Model {
        filename: "joiner.int8.onnx",
        bytes: 6_355_277,
        sha256: "3164c13fc2821009440d20fcb5fdc78bff28b4db2f8d0f0b329101719c0948b3",
    },
    Model {
        filename: "tokens.txt",
        bytes: 93_939,
        sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
    },
];

pub struct LocalParakeet(OfflineRecognizer);

fn verified(path: &Path, spec: &Model) -> Option<String> {
    let file = File::open(path).ok()?;
    if file.metadata().ok()?.len() != spec.bytes {
        return None;
    }
    let mut hash = Sha256::new();
    let read = io::copy(&mut io::Read::take(file, spec.bytes + 1), &mut hash).ok()?;
    if read != spec.bytes || format!("{:x}", hash.finalize()) != spec.sha256 {
        return None;
    }
    path.to_str().map(str::to_owned)
}

impl LocalParakeet {
    /// Loads the model folder after checking every file's size and SHA-256.
    pub fn load(folder: &Path) -> Result<Self, AudioError> {
        // ponytail: sherpa-onnx opens files by path, so a file swapped after hashing is not caught.
        let paths = PARAKEET_FILES
            .iter()
            .map(|spec| verified(&folder.join(spec.filename), spec))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                AudioError::new(
                    "Parakeet model folder is missing files or failed verification. Download it again.",
                )
            })?;
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.transducer = OfflineTransducerModelConfig {
            encoder: Some(paths[0].clone()),
            decoder: Some(paths[1].clone()),
            joiner: Some(paths[2].clone()),
        };
        config.model_config.tokens = Some(paths[3].clone());
        config.model_config.model_type = Some("nemo_transducer".into());
        config.model_config.num_threads =
            std::thread::available_parallelism().map_or(2, |n| n.get().min(6)) as i32;
        // ONNX Runtime threads otherwise spin between decodes. On the i5-1335U that used
        // 5-8x the CPU time for a 1% faster decode. sherpa-onnx reads options from a file.
        let options =
            std::env::temp_dir().join(format!("copilot-parakeet-{}.cfg", std::process::id()));
        let unwritable = || AudioError::new("Parakeet could not write its runtime options.");
        config.model_config.provider =
            Some(format!("cpu:{}", options.to_str().ok_or_else(unwritable)?));
        fs::write(
            &options,
            "SessionConfig.session.intra_op.allow_spinning=0\nSessionConfig.session.inter_op.allow_spinning=0\n",
        )
        .map_err(|_| unwritable())?;
        let recognizer = OfflineRecognizer::create(&config);
        let _ = fs::remove_file(&options);
        recognizer
            .map(Self)
            .ok_or_else(|| AudioError::new("Parakeet could not load the verified model."))
    }
}

impl SpeechRecognizer for LocalParakeet {
    fn decode(&mut self, samples: &[i16], _final_result: bool) -> Result<String, AudioError> {
        let signal: Vec<f32> = samples
            .iter()
            .map(|sample| f32::from(*sample) / 32_768.0)
            .collect();
        let stream = self.0.create_stream();
        stream.accept_waveform(SAMPLE_RATE as i32, &signal);
        self.0.decode(&stream);
        stream
            .get_result()
            .map(|result| result.text.trim().to_owned())
            .ok_or_else(|| {
                AudioError::new(
                    "Local speech decoding failed. Restart capture or choose another model.",
                )
            })
    }
}
