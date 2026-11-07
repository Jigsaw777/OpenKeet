use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use copilot_audio::MODELS;
use sha2::{Digest, Sha256};
use tauri::State;
use tokio::{fs, io::AsyncWriteExt, sync::Mutex};

const MODEL_REVISION: &str = "5359861c739e955e79d9a303bcbc70fb988958b1";

pub struct SpeechSetup {
    directory: PathBuf,
    downloading: Mutex<()>,
}

impl SpeechSetup {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory: directory.join("models"),
            downloading: Mutex::new(()),
        }
    }
}

fn model_available(path: &Path) -> bool {
    MODELS.iter().any(|model| {
        path.file_name().is_some_and(|name| name == model.filename)
            && path
                .metadata()
                .is_ok_and(|file| file.is_file() && file.len() == model.bytes)
    })
}

#[tauri::command]
pub async fn check_speech_model(path: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || model_available(Path::new(&path)))
        .await
        .map_err(|_| "Could not check the speech model.".into())
}

#[tauri::command]
pub async fn find_speech_model(setup: State<'_, SpeechSetup>) -> Result<Option<String>, String> {
    let cache = setup.directory.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let installed = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|path| path.join("models")));
        let project = std::env::current_dir().ok().map(|path| path.join("models"));
        [Some(cache), installed, project]
            .into_iter()
            .flatten()
            .map(|folder| folder.join(MODELS[0].filename))
            .find(|path| model_available(path))
            .and_then(|path| path.into_os_string().into_string().ok())
    })
    .await
    .map_err(|_| "Could not find local speech models.".into())
}

#[tauri::command]
pub async fn download_speech_model(setup: State<'_, SpeechSetup>) -> Result<String, String> {
    let _download = setup
        .downloading
        .try_lock()
        .map_err(|_| "A speech model download is already running.")?;
    let model = MODELS[0];
    let url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/{MODEL_REVISION}/{}",
        model.filename
    );
    let client = reqwest::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(900))
        .build()
        .map_err(|_| "Could not prepare the model download.")?;
    let mut response = client.get(url).send().await.and_then(reqwest::Response::error_for_status)
        .map_err(|_| "Could not download the speech model. Check the network or choose an existing file.")?;
    fs::create_dir_all(&setup.directory)
        .await
        .map_err(|_| "Could not create the speech model folder.")?;
    let staged = tempfile::NamedTempFile::new_in(&setup.directory)
        .map_err(|_| "Could not stage the speech model.")?;
    let mut file = fs::File::from_std(
        staged
            .reopen()
            .map_err(|_| "Could not open the staged speech model.")?,
    );
    let mut hash = Sha256::new();
    let mut length = 0;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Model download was interrupted. Try again.")?
    {
        length += chunk.len() as u64;
        if length > model.bytes {
            return Err("The downloaded speech model has an unexpected size.".into());
        }
        hash.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|_| "Could not write the speech model. Check free disk space.")?;
    }
    if length != model.bytes || format!("{:x}", hash.finalize()) != model.sha256 {
        return Err("The speech model failed verification. Try downloading again.".into());
    }
    file.flush()
        .await
        .map_err(|_| "Could not finish writing the speech model.")?;
    file.sync_all()
        .await
        .map_err(|_| "Could not finish writing the speech model.")?;
    drop(file);
    let path = setup.directory.join(model.filename);
    staged
        .persist(&path)
        .map_err(|_| "Could not save the verified speech model.")?;
    path.into_os_string()
        .into_string()
        .map_err(|_| "The speech model path cannot be displayed.".into())
}
