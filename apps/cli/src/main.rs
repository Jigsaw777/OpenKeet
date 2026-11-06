use std::{
    env,
    error::Error,
    fs,
    io::{self, Write},
    process::ExitCode,
};

use copilot_core::{
    CaptureState, Policy, Session, Source,
    rehearsal::{self, Step},
};

fn replay(path: Option<String>) -> Result<(), Box<dyn Error>> {
    let script = match path {
        Some(path) => fs::read_to_string(path)?,
        None => rehearsal::DEMO_JSON.to_owned(),
    };
    let steps: Vec<Step> = serde_json::from_str(&script)?;
    let policy = Policy {
        draft_after_ms: Some(600),
        ..Policy::default()
    };
    let mut session = Session::new(1, Source::REPLAY, policy);
    let mut last = 0;
    let mut out = io::BufWriter::new(io::stdout().lock());
    session.set_capture(CaptureState::Capturing, 0);
    for step in steps {
        let now = step.at_ms;
        if now < last {
            return Err("Replay must be ordered by at_ms.".into());
        }
        for tick in (last..now).step_by(100) {
            rehearsal::answer_available(&mut session, tick);
        }
        step.apply(&mut session)?;
        rehearsal::answer_available(&mut session, now);
        last = now;
        for event in session.drain_events() {
            writeln!(out, "{}", serde_json::to_string(&event)?)?;
        }
    }
    let end = last.saturating_add(policy.debounce_ms);
    rehearsal::answer_available(&mut session, end);
    session.set_capture(CaptureState::Stopped, end);
    for event in session.drain_events() {
        writeln!(out, "{}", serde_json::to_string(&event)?)?;
    }
    writeln!(out, "{}", serde_json::to_string(&session.snapshot())?)?;
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [] => replay(None),
        [cmd] if cmd == "replay" => replay(None),
        [cmd, path] if cmd == "replay" => replay(Some(path.clone())),
        [cmd] if cmd == "--help" || cmd == "-h" => {
            println!("Usage: copilot replay [fixture.json]");
            Ok(())
        }
        _ => Err("Usage: copilot replay [fixture.json]".into()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
