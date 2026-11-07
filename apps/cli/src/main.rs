use std::{env, error::Error, process::ExitCode};

#[cfg(feature = "audio")]
mod audio;
mod prep;

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(result) = prep::run(&args) {
        return result;
    }
    #[cfg(feature = "audio")]
    if let Some(result) = audio::run(&args) {
        return result;
    }
    let help = matches!(args.as_slice(), [cmd] if cmd == "--help" || cmd == "-h");
    println!("Usage:");
    prep::help();
    #[cfg(feature = "audio")]
    audio::help();
    #[cfg(not(feature = "audio"))]
    println!(
        "       Build with --features audio for the devices, transcribe and capture commands."
    );
    if help {
        Ok(())
    } else {
        Err("Choose a command.".into())
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
