//! Speech benchmark harness (formerly `benchmarks/*.py`): synthetic fixtures, word-error
//! scoring, CLI runs with Windows process metrics, and before/after comparisons.
//! Report files keep the JSON shapes the Python scripts wrote.

use std::{
    error::Error,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

const RATE: usize = 16_000;
const VOICES: [&str; 3] = [
    "Microsoft David Desktop",
    "Microsoft Hazel Desktop",
    "Microsoft Zira Desktop",
];
const STRESS_VOICES: [(&str, &str); 8] = [
    ("Microsoft David", "en-US"),
    ("Microsoft Mark", "en-US"),
    ("Microsoft Zira", "en-US"),
    ("Microsoft George", "en-GB"),
    ("Microsoft Hazel", "en-GB"),
    ("Microsoft Susan", "en-GB"),
    ("Microsoft Heera", "en-IN"),
    ("Microsoft Ravi", "en-IN"),
];
const PROFILE_VOICES: [&str; 3] = ["Microsoft Mark", "Microsoft Hazel", "Microsoft Heera"];
const PROFILE_CASES: [&str; 7] = [
    "seashells",
    "seashells-fast",
    "sixth-quiet",
    "cash-cache-noise",
    "tech-telephone",
    "why-fast",
    "correction-pause",
];

pub fn help() {
    println!(
        "       copilot bench-asr generate [--output DIR] [--stress] [--corpus FILE]\n       copilot bench-asr run --cli EXE --model MODEL [--output DIR] [--vocabulary WORDS] [--short-followups] [--profile]\n       copilot bench-asr profile --cli EXE --model MODEL [--stress DIR] [--output DIR]\n       copilot bench-asr compare --baseline DIR --cli EXE --model MODEL [--scratch DIR] [--output DIR]"
    );
    #[cfg(feature = "audio")]
    println!("       copilot bench-asr parakeet --input DIR --model PARAKEET_DIR");
}

pub fn run(args: &[String]) -> Option<Result> {
    let [first, command, rest @ ..] = args else {
        return None;
    };
    if first != "bench-asr" {
        return None;
    }
    let get = |name: &str| {
        rest.iter()
            .position(|a| a == name)
            .and_then(|i| rest.get(i + 1))
            .map(String::as_str)
    };
    let has = |name: &str| rest.iter().any(|a| a == name);
    let need = |name: &str| {
        get(name)
            .map(PathBuf::from)
            .ok_or_else(|| format!("{command} needs {name}."))
    };
    Some((|| -> Result {
        match command.as_str() {
            "generate" => {
                let stress = has("--stress");
                let out = get("--output")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| default_output(stress));
                generate(&out, stress, get("--corpus").map(Path::new))
            }
            "run" => {
                let out = get("--output")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| default_output(false));
                rust_run(
                    &out,
                    &need("--cli")?,
                    &need("--model")?,
                    get("--vocabulary").unwrap_or(""),
                    has("--profile"),
                    has("--short-followups"),
                )
                .map(|_| ())
            }
            "profile" => profile(
                &get("--stress")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| default_output(true)),
                &get("--output")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| Path::new("benchmarks/output/profile").into()),
                &need("--cli")?,
                &need("--model")?,
            ),
            "compare" => compare(
                &need("--baseline")?,
                &need("--cli")?,
                &need("--model")?,
                &get("--scratch")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| Path::new("benchmarks/output/optimized-3.0").into()),
                &get("--output")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| Path::new("benchmarks/results").into()),
            ),
            #[cfg(feature = "audio")]
            "parakeet" => parakeet(&need("--input")?, &need("--model")?),
            _ => Err("Unknown bench-asr command. Run copilot --help.".into()),
        }
    })())
}

fn default_output(stress: bool) -> PathBuf {
    Path::new("benchmarks/output").join(if stress { "stress" } else { "" })
}

// ---------- files ----------

fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_str(
        &fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?,
    )?)
}

fn write_json(path: &Path, value: &Value) -> Result {
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
    Ok(())
}

fn sha256(path: &Path) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

/// Mono 16 kHz 16-bit PCM, as the listener expects.
fn read_wav16(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if (
        spec.channels,
        spec.bits_per_sample,
        spec.sample_rate as usize,
    ) != (1, 16, RATE)
    {
        return Err(format!("Expected mono 16-bit {RATE} Hz PCM in {}", path.display()).into());
    }
    Ok(reader
        .samples::<i16>()
        .map(|s| s.map(|v| f32::from(v) / 32768.0))
        .collect::<std::result::Result<_, _>>()?)
}

/// Any PCM WAV (OneCore voices write their native rate) down-mixed and resampled to 16 kHz.
fn read_any_wav(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels.max(1));
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<std::result::Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / scale))
                .collect::<std::result::Result<_, _>>()?
        }
    };
    let mono: Vec<f32> = interleaved
        .chunks(channels)
        .map(|c| c.iter().sum::<f32>() / c.len() as f32)
        .collect();
    Ok(resample(&mono, spec.sample_rate as usize, RATE))
}

/// Windowed-sinc resampler (16 taps per side, Hann window).
fn resample(input: &[f32], from: usize, to: usize) -> Vec<f32> {
    if from == to {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let cutoff = 0.95 * (to as f64 / from as f64).min(1.0);
    let taps = 16.0 / cutoff;
    let len = (input.len() as f64 / ratio).floor() as usize;
    (0..len)
        .map(|i| {
            let t = i as f64 * ratio;
            let (lo, hi) = ((t - taps).ceil() as isize, (t + taps).floor() as isize);
            let mut acc = 0.0;
            for j in lo.max(0)..=hi.min(input.len() as isize - 1) {
                let x = t - j as f64;
                let window = 0.5 + 0.5 * (std::f64::consts::PI * x / taps).cos();
                acc += f64::from(input[j as usize]) * cutoff * sinc(cutoff * x) * window;
            }
            acc as f32
        })
        .collect()
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
    }
}

fn write_wav16(path: &Path, samples: &[f32]) -> Result {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: RATE as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for s in samples {
        writer.write_sample((s.clamp(-1.0, 0.9999) * 32768.0).round() as i16)?;
    }
    writer.finalize()?;
    Ok(())
}

// ---------- fixture generation ----------

/// Deterministic noise source (SplitMix64); fixtures differ from the Python generator's.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn uniform(&mut self) -> f32 {
        (self.next() * 2.0 - 1.0) as f32
    }
    fn normal(&mut self) -> f32 {
        let (a, b) = (self.next().max(f64::MIN_POSITIVE), self.next());
        ((-2.0 * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()) as f32
    }
}

/// Low-pass 8 kHz path with G.711-style mu-law companding.
fn telephone_band(samples: &[f32]) -> Vec<f32> {
    let cutoff = 3_400.0 / RATE as f64;
    let kernel: Vec<f64> = (-31..=31)
        .map(|n| {
            let k = f64::from(n + 31);
            2.0 * cutoff
                * sinc(2.0 * cutoff * f64::from(n))
                * (0.54 - 0.46 * (std::f64::consts::TAU * k / 62.0).cos())
        })
        .collect();
    let sum: f64 = kernel.iter().sum();
    let mu = 255f64;
    let narrow: Vec<f64> = (0..samples.len())
        .step_by(2)
        .map(|i| {
            let mut acc = 0.0;
            for (k, w) in kernel.iter().enumerate() {
                let j = i as isize + 31 - k as isize;
                if j >= 0 && (j as usize) < samples.len() {
                    acc += w / sum * f64::from(samples[j as usize]);
                }
            }
            let x = acc.clamp(-1.0, 1.0);
            let encoded = x.signum() * (mu * x.abs()).ln_1p() / mu.ln_1p();
            let q = (127.0 * encoded).round() / 127.0;
            q.signum() * (mu.ln_1p() * q.abs()).exp_m1() / mu
        })
        .collect();
    (0..samples.len())
        .map(|i| {
            let (a, frac) = (i / 2, (i % 2) as f64 / 2.0);
            let left = narrow[a.min(narrow.len() - 1)];
            let right = narrow.get(a + 1).copied().unwrap_or(left);
            (left + (right - left) * frac) as f32
        })
        .collect()
}

fn control_audio(kind: &str, rng: &mut Rng) -> Vec<f32> {
    let mut samples = vec![0f32; RATE * 3];
    let tau = std::f32::consts::TAU;
    match kind {
        "tone" => (0..RATE * 2)
            .for_each(|i| samples[i] = 0.09 * (tau * 440.0 * i as f32 / RATE as f32).sin()),
        "keyboard" => {
            for step in 0..13 {
                let offset = (RATE as f64 * (0.2 + 2.1 * f64::from(step) / 12.0)) as usize;
                for i in 0..400 {
                    samples[offset + i] = 0.45 * rng.uniform() * (-(i as f32) / 60.0).exp();
                }
            }
        }
        "chords" => {
            let n = RATE * 3 / 4;
            for (index, hz) in [
                [220.0, 277.0, 330.0],
                [196.0, 247.0, 294.0],
                [262.0, 330.0, 392.0],
            ]
            .iter()
            .enumerate()
            {
                for i in 0..n {
                    let t = i as f32 / RATE as f32;
                    let chord: f32 = hz.iter().map(|f| (tau * f * t).sin()).sum();
                    samples[index * n + i] = 0.05 * (std::f32::consts::PI * t / 0.75).sin() * chord;
                }
            }
        }
        _ => {}
    }
    samples
}

fn generate(output: &Path, stress: bool, corpus: Option<&Path>) -> Result {
    fs::create_dir_all(output)?;
    let output = &std::path::absolute(output)?;
    let default = Path::new("benchmarks").join(if stress {
        "stress-corpus.json"
    } else {
        "corpus.json"
    });
    let fixtures = read_json(corpus.unwrap_or(&default))?;
    let fixtures = fixtures
        .as_array()
        .ok_or("The corpus must be a JSON array.")?;
    let voices: Vec<&str> = if stress {
        STRESS_VOICES.iter().map(|v| v.0).collect()
    } else {
        VOICES.to_vec()
    };
    let (mut jobs, mut entries) = (Vec::new(), Vec::new());
    for fixture in fixtures {
        let parts: Vec<&str> = fixture["parts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let id = fixture["id"].as_str().ok_or("Every fixture needs an id.")?;
        let options: Vec<Option<&str>> = if parts.is_empty() {
            vec![None]
        } else {
            voices.iter().map(|v| Some(*v)).collect()
        };
        for (index, voice) in options.into_iter().enumerate() {
            let name = if voice.is_some() {
                format!("{id}-{index}")
            } else {
                id.to_string()
            };
            let files: Vec<String> = (0..parts.len())
                .map(|i| format!("{name}-part{i}.wav"))
                .collect();
            let rate = if stress {
                fixture.get("speaking_rate").cloned().unwrap_or(json!(1))
            } else {
                fixture.get("rate").cloned().unwrap_or(json!(0))
            };
            for (file, text) in files.iter().zip(&parts) {
                jobs.push(json!({"voice": voice, "rate": rate, "file": file, "text": text}));
            }
            entries.push((fixture, name, voice, files));
        }
    }
    write_json(&output.join("synthesis.json"), &Value::Array(jobs))?;
    let script = Path::new("benchmarks").join(if stress {
        "synthesize-onecore.ps1"
    } else {
        "synthesize.ps1"
    });
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-File"])
        .arg(&script)
        .arg("-Jobs")
        .arg(output.join("synthesis.json"))
        .arg("-Output")
        .arg(output)
        .status()?;
    if !status.success() {
        return Err(format!("{} failed ({status}).", script.display()).into());
    }
    let mut rng = Rng(20_261_106);
    let mut manifest = Vec::new();
    for (fixture, name, voice, files) in entries {
        let (samples, speech_end) = if files.is_empty() {
            (
                control_audio(fixture["control"].as_str().unwrap_or(""), &mut rng),
                Value::Null,
            )
        } else {
            let mut joined = vec![0f32; RATE / 4];
            for (index, file) in files.iter().enumerate() {
                let path = output.join(file);
                let part = if stress {
                    read_any_wav(&path)?
                } else {
                    read_wav16(&path)?
                };
                let first = part
                    .iter()
                    .position(|s| s.abs() > 0.002)
                    .ok_or("A synthesized part was silent.")?;
                let last = part.iter().rposition(|s| s.abs() > 0.002).unwrap_or(first);
                let start = (first
                    + RATE * fixture["clip_start_ms"].as_u64().unwrap_or(0) as usize / 1000)
                    .min(last);
                if index > 0 {
                    joined.extend(std::iter::repeat_n(
                        0.0,
                        RATE * fixture["gap_ms"].as_u64().unwrap_or(0) as usize / 1000,
                    ));
                }
                joined.extend_from_slice(&part[start..=last]);
            }
            let end = json!(joined.len() as f64 / RATE as f64);
            if fixture["telephone"].as_bool().unwrap_or(false) {
                joined = telephone_band(&joined);
            }
            if let Some(peak) = fixture["peak"].as_f64() {
                let max = joined
                    .iter()
                    .fold(0f32, |m, s| m.max(s.abs()))
                    .max(f32::EPSILON);
                joined.iter_mut().for_each(|s| *s *= peak as f32 / max);
            }
            if let Some(snr) = fixture["snr_db"].as_f64() {
                let rms = (joined.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>()
                    / joined.len() as f64)
                    .sqrt();
                let noise = (rms / 10f64.powf(snr / 20.0)) as f32;
                joined.iter_mut().for_each(|s| *s += noise * rng.normal());
            }
            joined.extend(std::iter::repeat_n(0.0, RATE * 3 / 2));
            (joined, end)
        };
        let path = output.join(format!("{name}.wav"));
        write_wav16(&path, &samples)?;
        let mut entry = json!({
            "id": name, "case": fixture["id"], "voice": voice, "file": format!("{name}.wav"), "sha256": sha256(&path)?,
            "reference": fixture["parts"].as_array().into_iter().flatten().filter_map(Value::as_str).collect::<Vec<_>>().join(" "),
            "terms": fixture.get("terms").cloned().unwrap_or(json!([])),
            "duration_s": samples.len() as f64 / RATE as f64, "speech_end_s": speech_end,
        });
        if stress {
            entry["locale"] = json!(
                voice
                    .and_then(|v| STRESS_VOICES.iter().find(|s| s.0 == v))
                    .map(|s| s.1)
            );
            entry["condition"] = fixture["condition"].clone();
        }
        manifest.push(entry);
    }
    let seconds: f64 = manifest
        .iter()
        .filter_map(|e| e["duration_s"].as_f64())
        .sum();
    println!("Generated {} clips ({seconds:.1} s)", manifest.len());
    write_json(&output.join("manifest.json"), &Value::Array(manifest))
}

// ---------- scoring ----------

pub fn words(text: &str) -> Vec<String> {
    let text = text.to_lowercase().replace('\u{2019}', "'");
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_alphanumeric() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && chars[i].is_alphanumeric() {
            i += 1;
        }
        if i + 1 < chars.len() && chars[i] == '\'' && chars[i + 1].is_alphanumeric() {
            i += 1;
            while i < chars.len() && chars[i].is_alphanumeric() {
                i += 1;
            }
        }
        out.push(chars[start..i].iter().collect());
    }
    out
}

fn distance<T: PartialEq>(reference: &[T], actual: &[T]) -> usize {
    let mut row: Vec<usize> = (0..=actual.len()).collect();
    for (index, expected) in reference.iter().enumerate() {
        let mut next = vec![index + 1];
        for (column, received) in actual.iter().enumerate() {
            let cost = row[column] + usize::from(expected != received);
            next.push((next[column] + 1).min(row[column + 1] + 1).min(cost));
        }
        row = next;
    }
    row[actual.len()]
}

pub fn score(reference: &str, actual: &str, terms: &[&str]) -> Value {
    let (expected, received) = (words(reference), words(actual));
    let (ec, rc): (Vec<char>, Vec<char>) = (
        expected.concat().chars().collect(),
        received.concat().chars().collect(),
    );
    let phrase = format!(" {} ", received.join(" "));
    json!({
        "word_errors": distance(&expected, &received), "words": expected.len(),
        "char_errors": distance(&ec, &rc), "chars": ec.len(),
        "term_hits": terms.iter().filter(|t| phrase.contains(&format!(" {} ", words(t).join(" ")))).count(),
        "terms": terms.len(), "missed": !expected.is_empty() && received.is_empty(),
        "false_transcript": expected.is_empty() && !received.is_empty(),
    })
}

/// One transcript event: (final?, seconds, utterance id, words).
type Event = (bool, f64, u64, Vec<String>);

/// Earliest partial whose first two words already match its utterance's final text.
pub fn stable_partial(events: &[Event]) -> Option<f64> {
    let mut ids: Vec<u64> = events.iter().map(|e| e.2).collect();
    ids.dedup();
    let mut best: Option<f64> = None;
    for id in ids {
        let stream: Vec<&Event> = events.iter().filter(|e| e.2 == id).collect();
        let Some(last) = stream.last().filter(|l| l.0 && !l.3.is_empty()) else {
            continue;
        };
        let prefix = &last.3[..last.3.len().min(2)];
        let head = |w: &[String]| w[..w.len().min(prefix.len())].to_vec();
        if let Some(found) = stream
            .windows(2)
            .find(|p| !p[0].0 && head(&p[0].3) == prefix && head(&p[1].3) == prefix)
        {
            best = Some(best.map_or(found[0].1, |b| b.min(found[0].1)));
        }
    }
    best
}

fn round(x: f64, places: i32) -> f64 {
    let f = 10f64.powi(places);
    (x * f).round() / f
}

fn num(row: &Value, key: &str) -> f64 {
    match &row[key] {
        Value::Bool(b) => f64::from(u8::from(*b)),
        v => v.as_f64().unwrap_or(0.0),
    }
}

fn summarize(rows: &[Value]) -> Value {
    let stats = |key: &str| {
        let mut data: Vec<f64> = rows.iter().filter_map(|r| r[key].as_f64()).collect();
        if data.is_empty() {
            return json!({"n": 0});
        }
        data.sort_by(f64::total_cmp);
        let n = data.len();
        let median = if n % 2 == 1 {
            data[n / 2]
        } else {
            (data[n / 2 - 1] + data[n / 2]) / 2.0
        };
        json!({"n": n, "p50_s": round(median, 3), "p95_s": round(data[(0.95 * n as f64).ceil() as usize - 1], 3)})
    };
    let mut out = Map::new();
    for key in [
        "word_errors",
        "words",
        "char_errors",
        "chars",
        "term_hits",
        "terms",
        "missed",
        "false_transcript",
        "dropped_finals",
        "dropped_frames",
        "discontinuities",
    ] {
        out.insert(
            key.into(),
            json!(rows.iter().map(|r| num(r, key)).sum::<f64>() as u64),
        );
    }
    let total = |key: &str| rows.iter().map(|r| num(r, key)).sum::<f64>();
    out.insert(
        "wer".into(),
        json!(round(total("word_errors") / total("words").max(1.0), 4)),
    );
    out.insert(
        "cer".into(),
        json!(round(total("char_errors") / total("chars").max(1.0), 4)),
    );
    out.insert("final_latency".into(), stats("final_latency_s"));
    out.insert("stable_partial".into(), stats("stable_partial_s"));
    out.insert(
        "reported_decode_s".into(),
        json!(round(total("reported_decode_s"), 3)),
    );
    out.insert("audio_s".into(), json!(round(total("duration_s"), 3)));
    Value::Object(out)
}

// ---------- running the CLI ----------

struct Finished {
    success: bool,
    stdout: String,
    stderr: String,
    metrics: Option<Value>,
}

/// Runs a command with a timeout; with `profile`, samples its Windows memory every 100 ms.
fn execute(command: &[String], timeout: f64, profile: bool) -> Result<Finished> {
    let mut child = Command::new(&command[0])
        .args(&command[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let drain = |mut pipe: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut s = String::new();
            let _ = pipe.read_to_string(&mut s);
            s
        })
    };
    let out = drain(Box::new(child.stdout.take().expect("piped stdout")));
    let err = drain(Box::new(child.stderr.take().expect("piped stderr")));
    let started = Instant::now();
    let mut samples = Vec::new();
    let (mut peak_ws, mut peak_commit) = (0u64, 0u64);
    let status = loop {
        if profile && let Some((ws, peak, commit)) = memory(&child) {
            samples.push(ws);
            peak_ws = peak_ws.max(peak);
            peak_commit = peak_commit.max(commit);
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed().as_secs_f64() >= timeout {
            let _ = child.kill();
            return Err(format!("{} timed out after {timeout:.0} s.", command[0]).into());
        }
        thread::sleep(Duration::from_millis(100));
    };
    let wall = started.elapsed().as_secs_f64();
    let metrics = if profile {
        if samples.is_empty() {
            return Err("No Windows process memory samples were captured.".into());
        }
        samples.sort_unstable();
        let mib = |b: u64| round(b as f64 / 1_048_576.0, 1);
        Some(json!({
            "wall_s": round(wall, 3), "cpu_s": round(cpu_seconds(&child).unwrap_or(0.0), 3),
            "peak_working_set_mib": mib(peak_ws), "median_working_set_mib": mib(samples[samples.len() / 2]),
            "peak_private_commit_mib": mib(peak_commit), "samples": samples.len(),
        }))
    } else {
        None
    };
    Ok(Finished {
        success: status.success(),
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
        metrics,
    })
}

#[cfg(windows)]
fn memory(child: &std::process::Child) -> Option<(u64, u64, u64)> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
    };
    let mut c = PROCESS_MEMORY_COUNTERS {
        cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..Default::default()
    };
    // SAFETY: the child's handle stays open while `child` is alive; `c` is a valid out pointer.
    unsafe { GetProcessMemoryInfo(HANDLE(child.as_raw_handle()), &mut c, c.cb) }.ok()?;
    Some((
        c.WorkingSetSize as u64,
        c.PeakWorkingSetSize as u64,
        c.PeakPagefileUsage as u64,
    ))
}

#[cfg(windows)]
fn cpu_seconds(child: &std::process::Child) -> Option<f64> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::{FILETIME, HANDLE},
        System::Threading::GetProcessTimes,
    };
    let (mut a, mut b, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY: as above; all four out pointers are valid.
    unsafe {
        GetProcessTimes(
            HANDLE(child.as_raw_handle()),
            &mut a,
            &mut b,
            &mut kernel,
            &mut user,
        )
    }
    .ok()?;
    let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
    Some((ticks(kernel) + ticks(user)) as f64 / 10_000_000.0)
}

#[cfg(not(windows))]
fn memory(_: &std::process::Child) -> Option<(u64, u64, u64)> {
    None
}

#[cfg(not(windows))]
fn cpu_seconds(_: &std::process::Child) -> Option<f64> {
    None
}

fn check_hashes(output: &Path, manifest: &[Value]) -> Result {
    for entry in manifest {
        let path = output.join(entry["file"].as_str().unwrap_or(""));
        if sha256(&path)? != entry["sha256"].as_str().unwrap_or("") {
            return Err(format!("Fixture checksum changed: {}", path.display()).into());
        }
    }
    Ok(())
}

fn model_label(model: &Path) -> String {
    let name = if model.is_dir() {
        model.file_name()
    } else {
        model.file_stem()
    };
    name.map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn rust_run(
    output: &Path,
    cli: &Path,
    model: &Path,
    vocabulary: &str,
    profile: bool,
    short_followups: bool,
) -> Result<Value> {
    let manifest = read_json(&output.join("manifest.json"))?;
    let manifest = manifest
        .as_array()
        .ok_or("manifest.json must be an array.")?;
    check_hashes(output, manifest)?;
    println!(
        "Running Rust {} on {} clips",
        model.file_name().unwrap_or_default().to_string_lossy(),
        manifest.len()
    );
    let started = Instant::now();
    let mut command = vec![
        std::path::absolute(cli)?.display().to_string(),
        "transcribe".into(),
        std::path::absolute(model)?.display().to_string(),
    ];
    if !vocabulary.is_empty() {
        command.extend(["--vocabulary".into(), vocabulary.into()]);
    }
    if short_followups {
        command.push("--short-followups".into());
    }
    command.extend(manifest.iter().map(|e| {
        output
            .join(e["file"].as_str().unwrap_or(""))
            .display()
            .to_string()
    }));
    let timeout = manifest.iter().map(|e| num(e, "duration_s")).sum::<f64>() * 4.0 + 180.0;
    let done = execute(&command, timeout, profile)?;
    if !done.success {
        return Err(format!("Rust comparator failed: {}", done.stderr.trim()).into());
    }
    let (mut events, mut rows, mut segments, mut load_s) = (
        Vec::<(Event, f64, f64, String)>::new(),
        Vec::new(),
        0u64,
        Value::Null,
    );
    for line in done.stdout.lines().filter(|l| !l.trim().is_empty()) {
        let item: Value = serde_json::from_str(line)?;
        match item["event"].as_str().unwrap_or("") {
            "model_ready" => load_s = json!(num(&item, "load_ms") / 1000.0),
            "clip_start" => {
                events.clear();
                segments = 0;
            }
            "speech" => segments += 1,
            "transcript" => {
                let u = &item["utterance"];
                let text = u["text"].as_str().unwrap_or("");
                events.push((
                    (
                        u["stability"] == "final",
                        num(&item, "at_ms") / 1000.0,
                        u["id"].as_u64().unwrap_or(0),
                        words(text),
                    ),
                    num(&item, "decode_ms") / 1000.0,
                    num(&item, "queue_ms") / 1000.0,
                    text.to_string(),
                ));
            }
            "clip_end" => {
                let index = item["index"].as_u64().unwrap_or(0) as usize;
                let entry = manifest.get(index).ok_or("clip_end for an unknown clip")?;
                let finals: Vec<&(Event, f64, f64, String)> =
                    events.iter().filter(|e| e.0.0 && !e.3.is_empty()).collect();
                let actual = finals
                    .iter()
                    .map(|e| e.3.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                let terms: Vec<&str> = entry["terms"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                let mut row = entry.as_object().cloned().unwrap_or_default();
                if let Value::Object(s) =
                    score(entry["reference"].as_str().unwrap_or(""), &actual, &terms)
                {
                    row.extend(s);
                }
                let plain: Vec<Event> = events.iter().map(|e| e.0.clone()).collect();
                row.insert("actual".into(), json!(actual));
                row.insert("speech_segments".into(), json!(segments));
                row.insert(
                    "empty_final_segments".into(),
                    json!(events.iter().filter(|e| e.0.0 && e.3.is_empty()).count()),
                );
                row.insert("stable_partial_s".into(), json!(stable_partial(&plain)));
                row.insert(
                    "final_latency_s".into(),
                    match (finals.last(), entry["speech_end_s"].as_f64()) {
                        (Some(last), Some(end)) => json!(last.0.1 - end),
                        _ => Value::Null,
                    },
                );
                row.insert(
                    "reported_decode_s".into(),
                    json!(
                        events
                            .iter()
                            .filter(|e| !e.3.is_empty())
                            .map(|e| e.1)
                            .sum::<f64>()
                    ),
                );
                row.insert(
                    "max_decode_queue_s".into(),
                    json!(events.iter().map(|e| e.2).fold(0f64, f64::max)),
                );
                for key in ["dropped_finals", "dropped_frames", "discontinuities"] {
                    row.insert(key.into(), json!(0));
                }
                rows.push(Value::Object(row));
            }
            _ => {}
        }
    }
    if rows
        .iter()
        .map(|r| &r["id"])
        .ne(manifest.iter().map(|e| &e["id"]))
    {
        return Err("Rust comparator did not finish every fixture in manifest order".into());
    }
    let mut summary = summarize(&rows);
    summary["load_s"] = load_s;
    summary["wall_s_with_load"] = json!(round(started.elapsed().as_secs_f64(), 3));
    summary["max_decode_queue_s"] = json!(
        rows.iter()
            .map(|r| num(r, "max_decode_queue_s"))
            .fold(0f64, f64::max)
    );
    let label = format!(
        "rust-{}{}{}",
        model_label(model),
        if vocabulary.is_empty() { "" } else { "-hints" },
        if short_followups {
            "-short-followups"
        } else {
            ""
        }
    );
    let mut report =
        json!({"backend": label, "vocabulary": vocabulary, "summary": summary, "clips": rows});
    if let Some(metrics) = done.metrics {
        report["process"] = metrics;
    }
    write_json(&output.join(format!("{label}.json")), &report)?;
    println!("{label} {}", report["summary"]);
    Ok(report)
}

// ---------- profiles and comparisons ----------

fn profile(stress: &Path, output: &Path, cli: &Path, model: &Path) -> Result {
    fs::create_dir_all(output)?;
    let stress = std::path::absolute(stress)?;
    let rows: Vec<Value> = read_json(&stress.join("manifest.json"))?
        .as_array()
        .ok_or("manifest.json must be an array.")?
        .iter()
        .filter(|e| {
            e["condition"] == "control"
                || (PROFILE_VOICES.iter().any(|v| e["voice"] == *v)
                    && PROFILE_CASES.iter().any(|c| e["case"] == *c))
        })
        .map(|e| {
            let mut e = e.clone();
            e["file"] = json!(
                stress
                    .join(e["file"].as_str().unwrap_or(""))
                    .display()
                    .to_string()
            );
            e
        })
        .collect();
    if rows.len() != PROFILE_VOICES.len() * PROFILE_CASES.len() + 4 {
        return Err("Generate the complete Windows stress pack before profiling.".into());
    }
    let manifest = output.join("manifest.json");
    write_json(&manifest, &Value::Array(rows.clone()))?;
    println!("Profiling the Rust engine on {} clips", rows.len());
    let report = rust_run(output, cli, model, "", true, false)?;
    // The Python prototype is no longer profiled; its frozen numbers stay in benchmarks/results.
    write_json(
        &output.join("comparison.json"),
        &json!({
            "manifest_sha256": sha256(&manifest)?, "clips": rows.len(),
            "rust": {"process": report["process"], "asr": report["summary"]},
        }),
    )?;
    println!("rust process {}", report["process"]);
    Ok(())
}

fn compare(baseline: &Path, cli: &Path, model: &Path, scratch: &Path, output: &Path) -> Result {
    let baseline = std::path::absolute(baseline)?;
    fs::create_dir_all(scratch)?;
    let manifest = baseline.join("manifest.json");
    let manifest_sha = sha256(&manifest)?;
    let summary = read_json(&baseline.join("comparison.json"))?;
    if summary["manifest_sha256"] != manifest_sha.as_str() {
        return Err("The baseline manifest changed after its measurements.".into());
    }
    let entries: Vec<Value> = read_json(&manifest)?
        .as_array()
        .ok_or("manifest.json must be an array.")?
        .iter()
        .map(|c| {
            let mut c = c.clone();
            c["file"] = json!(
                baseline
                    .join(c["file"].as_str().unwrap_or(""))
                    .display()
                    .to_string()
            );
            c
        })
        .collect();
    write_json(
        &scratch.join("manifest.json"),
        &Value::Array(entries.clone()),
    )?;
    let mut reports = Vec::new();
    let mut names = Vec::new();
    if let (Ok(mut python), Some(process)) = (
        read_json(&baseline.join("legacy-small.en.json")),
        summary.get("python").map(|p| p["process"].clone()),
    ) {
        python["process"] = process;
        reports.push(python);
        names.push(("python-baseline", "Python A"));
    }
    reports.push(read_json(
        &baseline.join(format!("rust-{}.json", model_label(model))),
    )?);
    names.push(("current-before", "Current B"));
    reports.push(rust_run(scratch, cli, model, "", true, false)?);
    names.push(("optimized-after", "Optimized C"));
    let ids: Vec<Vec<(Value, Value)>> = reports
        .iter()
        .map(|r| {
            r["clips"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|c| (c["id"].clone(), c["sha256"].clone()))
                .collect()
        })
        .collect();
    if ids[0].is_empty() || ids.iter().any(|i| *i != ids[0]) {
        return Err("Baseline and candidate must use the same ordered IDs and WAV hashes.".into());
    }
    let clips: Vec<&Vec<Value>> = reports
        .iter()
        .map(|r| r["clips"].as_array().expect("clips"))
        .collect();
    let mut data: Vec<(String, &str, Vec<f64>)> = Vec::new();
    for (key, label) in [
        ("final_latency_s", "Final latency"),
        ("stable_partial_s", "Stable preview latency"),
    ] {
        let common: Vec<usize> = (0..clips[0].len())
            .filter(|&i| clips.iter().all(|c| !c[i][key].is_null()))
            .collect();
        if common.is_empty() {
            continue;
        }
        let stats: Vec<Value> = clips
            .iter()
            .map(|c| {
                summarize(&common.iter().map(|&i| c[i].clone()).collect::<Vec<_>>())
                    [key.trim_end_matches("_s")]
                .clone()
            })
            .collect();
        for q in ["p50", "p95"] {
            data.push((
                format!("{label} {q} ({} common clips)", common.len()),
                "s",
                stats.iter().map(|s| num(s, &format!("{q}_s"))).collect(),
            ));
        }
    }
    for (key, label, unit) in [
        ("cpu_s", "Process CPU", "s"),
        ("wall_s", "Process wall time", "s"),
        ("peak_working_set_mib", "Peak resident working set", "MiB"),
        (
            "median_working_set_mib",
            "Median resident working set",
            "MiB",
        ),
        ("peak_private_commit_mib", "Peak private commit", "MiB"),
    ] {
        data.push((
            label.into(),
            unit,
            reports.iter().map(|r| num(&r["process"], key)).collect(),
        ));
    }
    for (key, label, unit, factor) in [
        ("load_s", "Model load", "s", 1.0),
        ("wer", "Word error rate", "%", 100.0),
        ("cer", "Character error rate", "%", 100.0),
        ("term_hits", "Technical term matches", "matches", 1.0),
        ("missed", "Missed speech clips", "clips", 1.0),
        (
            "false_transcript",
            "False transcripts in controls",
            "clips",
            1.0,
        ),
        (
            "dropped_frames",
            "Reported frame queue drops",
            "events",
            1.0,
        ),
        (
            "dropped_finals",
            "Reported final queue drops",
            "events",
            1.0,
        ),
    ] {
        data.push((
            label.into(),
            unit,
            reports
                .iter()
                .map(|r| round(num(&r["summary"], key) * factor, 3))
                .collect(),
        ));
    }
    fs::create_dir_all(output)?;
    for ((file, _), report) in names.iter().zip(&reports) {
        let mut report = report.clone();
        if let Some(clips) = report["clips"].as_array_mut() {
            clips.iter_mut().for_each(|c| {
                c.as_object_mut().map(|o| o.remove("file"));
            });
        }
        report["manifest_sha256"] = json!(manifest_sha);
        write_json(&output.join(format!("{file}.json")), &report)?;
    }
    let mut header: Vec<String> = ["Metric", "Unit"].map(String::from).to_vec();
    header.extend(names.iter().map(|n| n.1.to_string()));
    let last = reports.len() - 1;
    if last == 2 {
        header.push("C − A".into());
    }
    header.push("C − B".into());
    let rows: Vec<Vec<String>> = data
        .iter()
        .map(|(label, unit, v)| {
            let mut row = vec![label.clone(), unit.to_string()];
            row.extend(v.iter().map(|x| fmt(*x)));
            if last == 2 {
                row.push(fmt(round(v[2] - v[0], 3)));
            }
            row.push(fmt(round(v[last] - v[last - 1], 3)));
            row
        })
        .collect();
    let csv: String = std::iter::once(&header)
        .chain(&rows)
        .map(|r| {
            r.iter()
                .map(|c| {
                    if c.contains(',') {
                        format!("\"{c}\"")
                    } else {
                        c.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
                + "\r\n"
        })
        .collect();
    fs::write(output.join("comparison.csv"), csv)?;
    let align: Vec<&str> = (0..header.len())
        .map(|i| if i < 2 { "---" } else { "---:" })
        .collect();
    let table: Vec<String> = std::iter::once(header.clone())
        .chain(std::iter::once(
            align.iter().map(|s| s.to_string()).collect(),
        ))
        .chain(rows)
        .map(|r| format!("| {} |", r.join(" | ")))
        .collect();
    fs::write(
        output.join("comparison.md"),
        format!(
            "# Paired ASR process comparison\n\nOne sequential run per engine, {} clips. Manifest SHA-256 `{manifest_sha}`. WER/CER differences are percentage points. The table excludes desktop UI and LLM costs.\n\n{}\n",
            entries.len(),
            table.join("\n")
        ),
    )?;
    Ok(())
}

fn fmt(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{x:.1}")
    } else {
        format!("{x}")
    }
}

// ---------- Parakeet offline reference ----------

#[cfg(feature = "audio")]
fn parakeet(input: &Path, model: &Path) -> Result {
    let started = Instant::now();
    let mut recognizer = copilot_audio::load(copilot_audio::Engine::Parakeet, model, "", false)?;
    let load_s = started.elapsed().as_secs_f64();
    let manifest = read_json(&input.join("manifest.json"))?;
    let manifest = manifest
        .as_array()
        .ok_or("manifest.json must be an array.")?;
    check_hashes(input, manifest)?;
    let mut rows = Vec::new();
    for entry in manifest {
        let samples: Vec<i16> = read_wav16(&input.join(entry["file"].as_str().unwrap_or("")))?
            .iter()
            .map(|s| (s * 32768.0) as i16)
            .collect();
        let started = Instant::now();
        let text = recognizer.decode(&samples, true)?.trim().to_string();
        let terms: Vec<&str> = entry["terms"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let mut row = entry.as_object().cloned().unwrap_or_default();
        if let Value::Object(s) = score(entry["reference"].as_str().unwrap_or(""), &text, &terms) {
            row.extend(s);
        }
        row.insert("actual".into(), json!(text));
        row.insert(
            "offline_decode_s".into(),
            json!(started.elapsed().as_secs_f64()),
        );
        rows.push(Value::Object(row));
    }
    let mut summary = Map::new();
    for key in [
        "word_errors",
        "words",
        "char_errors",
        "chars",
        "term_hits",
        "terms",
        "missed",
        "false_transcript",
        "duration_s",
        "offline_decode_s",
    ] {
        summary.insert(
            key.into(),
            json!(rows.iter().map(|r| num(r, key)).sum::<f64>()),
        );
    }
    let get = |k: &str| summary[k].as_f64().unwrap_or(0.0);
    let extra = json!({"wer": get("word_errors") / get("words").max(1.0), "cer": get("char_errors") / get("chars").max(1.0),
        "load_s": load_s, "offline_rtf": get("offline_decode_s") / get("duration_s").max(f64::EPSILON)});
    summary.extend(extra.as_object().cloned().unwrap_or_default());
    let report = json!({"summary": summary, "clips": rows});
    write_json(&input.join("nvidia-parakeet-v3-offline.json"), &report)?;
    println!("{}", report["summary"]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoring_matches_the_former_python_harness() {
        let result = score(
            "LRU cache in Java!",
            "LRU cash in JAVA",
            &["LRU cache", "Java"],
        );
        assert_eq!(
            (
                result["word_errors"].as_u64(),
                result["term_hits"].as_u64(),
                result["words"].as_u64()
            ),
            (Some(1), Some(1), Some(4))
        );
        assert_eq!(score("", "spurious speech", &[])["false_transcript"], true);
        assert_eq!(score("why?", "", &[])["missed"], true);
        assert_eq!(
            words("Don’t stop, it's p95_latency"),
            ["don't", "stop", "it's", "p95", "latency"]
        );
        let event = |fin: bool, at: f64, text: &str| (fin, at, 1, words(text));
        let events = [
            event(false, 1.0, "Do not"),
            event(false, 2.0, "Do you"),
            event(true, 3.0, "Do you agree?"),
        ];
        assert_eq!(stable_partial(&events), Some(2.0));
        assert_eq!(stable_partial(&events[..2]), None);
        let tone = control_audio("tone", &mut Rng(1));
        assert_eq!(
            (
                telephone_band(&tone).len(),
                resample(&tone, 16_000, 8_000).len()
            ),
            (tone.len(), tone.len() / 2)
        );
    }
}
