use std::{
    error::Error,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use copilot_audio::{LocalWhisper, Stop, StreamOptions, elapsed_ms};
use copilot_core::{Source, SourceKind};
use serde_json::json;

type Result = std::result::Result<(), Box<dyn Error>>;

pub fn help() {
    println!(
        "       copilot devices\n       copilot transcribe MODEL [--vocabulary WORDS] [--short-followups] WAV...\n       copilot capture MODEL system|mic default|ENDPOINT_ID SECONDS"
    );
}

fn emit(event: serde_json::Value) {
    println!("{event}");
}

fn model(path: &str, vocabulary: &str) -> std::result::Result<LocalWhisper, Box<dyn Error>> {
    let start = Instant::now();
    let model = LocalWhisper::load(Path::new(path), vocabulary)?;
    emit(json!({"event":"model_ready", "load_ms":elapsed_ms(start)}));
    Ok(model)
}

fn transcribe(path: &str, args: &[String]) -> Result {
    let (vocabulary, clips) = match args {
        [flag, vocabulary, clips @ ..] if flag == "--vocabulary" => (vocabulary.as_str(), clips),
        _ => ("", args),
    };
    let (short_followups, clips) = match clips {
        [flag, clips @ ..] if flag == "--short-followups" => (true, clips),
        _ => (false, clips),
    };
    if clips.is_empty() {
        return Err("Provide at least one mono 16 kHz PCM WAV.".into());
    }
    let mut model = model(path, vocabulary)?;
    if short_followups {
        model.enable_short_followups();
    }
    for (index, clip) in clips.iter().enumerate() {
        emit(json!({"event":"clip_start","index":index}));
        copilot_audio::recognize_wav(&mut model, Path::new(clip), |event| {
            println!(
                "{}",
                serde_json::to_string(&event).expect("serializable audio event")
            );
        })?;
        emit(json!({"event":"clip_end","index":index}));
    }
    Ok(())
}

fn capture(path: &str, route: &str, endpoint: &str, seconds: &str) -> Result {
    let kind = match route {
        "system" => SourceKind::SystemOutput,
        "mic" => SourceKind::Microphone,
        _ => return Err("Choose the system or mic route explicitly.".into()),
    };
    let seconds: u64 = seconds.parse()?;
    if !(1..=3600).contains(&seconds) {
        return Err("Capture length must be between 1 and 3600 seconds.".into());
    }
    let source = Source {
        kind,
        endpoint_id: (endpoint != "default").then(|| endpoint.to_owned()),
    };
    let mut model = model(path, "")?;
    let stop = Stop::default();
    copilot_audio::capture(
        &mut model,
        source,
        Instant::now(),
        StreamOptions::default(),
        &stop,
        |event| {
            if matches!(event, copilot_audio::AudioEvent::Ready { .. }) {
                let timer = stop.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_secs(seconds));
                    timer.request();
                });
            }
            println!(
                "{}",
                serde_json::to_string(&event).expect("serializable audio event")
            );
        },
    )?;
    Ok(())
}

pub fn run(args: &[String]) -> Option<Result> {
    match args {
        [cmd] if cmd == "devices" => Some(
            copilot_audio::endpoints()
                .map(|devices| emit(json!(devices)))
                .map_err(Into::into),
        ),
        [cmd, path, rest @ ..] if cmd == "transcribe" => Some(transcribe(path, rest)),
        [cmd, path, route, endpoint, seconds] if cmd == "capture" => {
            Some(capture(path, route, endpoint, seconds))
        }
        _ => None,
    }
}
