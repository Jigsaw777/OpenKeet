#![cfg(windows)]

use std::{
    env,
    path::PathBuf,
    sync::{Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use copilot_audio::{AudioEvent, Engine, Stop, StreamOptions};
use copilot_core::{Source, SourceKind, Stability};
use wasapi::{DeviceEnumerator, Direction, SessionState};
use windows::{
    Win32::Media::Audio::{PlaySoundW, SND_FILENAME, SND_NODEFAULT, SND_SYNC},
    core::HSTRING,
};

fn quiet_default_output() -> String {
    wasapi::initialize_mta().ok().unwrap();
    let (id, active) = {
        let device = DeviceEnumerator::new()
            .unwrap()
            .get_default_device(&Direction::Render)
            .unwrap();
        let manager = device.get_iaudiosessionmanager().unwrap();
        let sessions = manager.get_audiosessionenumerator().unwrap();
        let active = (0..sessions.get_count().unwrap())
            .filter(|index| {
                sessions.get_session(*index).unwrap().get_state().unwrap() == SessionState::Active
            })
            .count();
        (device.get_id().unwrap(), active)
    };
    wasapi::deinitialize();
    assert_eq!(
        active, 0,
        "Stop unrelated playback before the synthetic WASAPI test."
    );
    id
}

#[test]
#[ignore = "plays a synthetic fixture on the default Windows output"]
fn default_and_explicit_loopback_transcribe_synthetic_playback() {
    let model = PathBuf::from(
        env::var_os("COPILOT_TEST_MODEL").expect("Set COPILOT_TEST_MODEL to a verified model file"),
    );
    let wav = PathBuf::from(
        env::var_os("COPILOT_TEST_WAV").expect("Set COPILOT_TEST_WAV to generated cache-0.wav"),
    );
    let duration = hound::WavReader::open(&wav).unwrap();
    assert!(
        duration.duration() < duration.spec().sample_rate * 10,
        "Use a short synthetic fixture."
    );
    // A model folder selects Parakeet. A .bin file selects Whisper.
    let engine = if model.is_dir() {
        Engine::Parakeet
    } else {
        Engine::Whisper
    };
    let mut model = copilot_audio::load(engine, &model, "LRU cache, Java", false).unwrap();
    for explicit in [false, true] {
        let endpoint = quiet_default_output();
        let source = Source {
            kind: SourceKind::SystemOutput,
            endpoint_id: explicit.then_some(endpoint),
        };
        let (ready, started) = mpsc::sync_channel(1);
        let stop = Stop::default();
        let report = Mutex::new(Vec::new());
        let path = HSTRING::from(wav.as_os_str());
        thread::scope(|scope| {
            let finish = stop.clone();
            let playback = scope.spawn(move || {
                let played = started.recv_timeout(Duration::from_secs(5)).is_ok() && {
                    thread::sleep(Duration::from_millis(300));
                    // SAFETY: the path is a live UTF-16 string. Synchronous playback uses a local fixture.
                    unsafe { PlaySoundW(&path, None, SND_FILENAME | SND_NODEFAULT | SND_SYNC) }
                        .as_bool()
                };
                thread::sleep(Duration::from_millis(900));
                finish.request();
                played
            });
            let result = copilot_audio::capture(
                model.as_mut(),
                source,
                None,
                Instant::now(),
                StreamOptions::default(),
                &stop,
                |event| {
                    if matches!(event, AudioEvent::Ready { .. }) {
                        let _ = ready.try_send(());
                    }
                    report.lock().unwrap().push(event);
                },
            );
            assert!(
                playback.join().unwrap(),
                "Synthetic fixture playback did not start."
            );
            result.unwrap();
        });
        let events = report.into_inner().unwrap();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, AudioEvent::Meter { peak, .. } if *peak > 0))
        );
        let finals: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                AudioEvent::Transcript {
                    utterance, at_ms, ..
                } if utterance.stability == Stability::Final => Some((
                    utterance.text.to_lowercase(),
                    at_ms.saturating_sub(utterance.span.end_ms),
                )),
                _ => None,
            })
            .collect();
        let final_text = finals
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            final_text.contains("cache") && final_text.contains("java"),
            "Expected the fixture terms in a final transcript."
        );
        println!(
            "{} system route ({engine:?}): nonzero meter and both fixture terms recognized in {final_text:?}, final {} ms after the last voiced frame",
            if explicit { "Explicit" } else { "Default" },
            finals.last().map_or(0, |(_, ms)| *ms)
        );
        thread::sleep(Duration::from_secs(1));
    }
}
