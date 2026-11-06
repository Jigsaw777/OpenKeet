#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod history;
mod workspace;

use std::{sync::Arc, thread, time::Duration};

use copilot_audio::Endpoint;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use workspace::{CaptureConfig, Desktop, Shared, View, emit, update};

#[tauri::command]
fn get_snapshot(state: State<'_, Shared>) -> View {
    state.lock().expect("desktop session lock").view()
}

#[tauri::command]
async fn list_endpoints() -> Result<Vec<Endpoint>, String> {
    tauri::async_runtime::spawn_blocking(copilot_audio::endpoints)
        .await
        .map_err(|_| "Could not enumerate audio devices.")?
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn start_sample(app: AppHandle, state: State<'_, Shared>) -> Result<View, String> {
    update(&app, &state, Desktop::start_sample)
}

#[tauri::command]
fn start_capture(
    app: AppHandle,
    state: State<'_, Shared>,
    config: CaptureConfig,
) -> Result<View, String> {
    workspace::start_audio(app, Arc::clone(&state), Some(config))
}

#[tauri::command]
fn resume_capture(app: AppHandle, state: State<'_, Shared>) -> Result<View, String> {
    if let Some(view) = workspace::resume_sample(&app, &state) {
        return Ok(view);
    }
    workspace::start_audio(app, Arc::clone(&state), None)
}

#[tauri::command]
fn pause_capture(app: AppHandle, state: State<'_, Shared>) -> Result<View, String> {
    update(&app, &state, |desktop| desktop.stop(true))
}

#[tauri::command]
fn stop_capture(app: AppHandle, state: State<'_, Shared>) -> Result<View, String> {
    update(&app, &state, |desktop| desktop.stop(false))
}

#[tauri::command]
fn ask(app: AppHandle, state: State<'_, Shared>, question: String) -> Result<View, String> {
    update(&app, &state, |desktop| desktop.ask(&question))
}

#[tauri::command]
fn request_detected(app: AppHandle, state: State<'_, Shared>) -> Result<View, String> {
    update(&app, &state, Desktop::request_detected)
}

#[tauri::command]
fn cancel_answer(app: AppHandle, state: State<'_, Shared>) -> Result<View, String> {
    update(&app, &state, |desktop| {
        desktop.cancel_answer();
        Ok(())
    })
}

#[tauri::command]
fn use_transcript(state: State<'_, Shared>) -> Result<String, String> {
    state
        .lock()
        .expect("desktop session lock")
        .full_transcript()
}

#[tauri::command]
fn clear_session(app: AppHandle, state: State<'_, Shared>) -> Result<View, String> {
    update(&app, &state, Desktop::clear)
}

#[tauri::command]
async fn open_focus(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("focus") {
        window
            .show()
            .and_then(|_| window.set_focus())
            .map_err(|err| err.to_string())?;
    } else {
        WebviewWindowBuilder::new(&app, "focus", WebviewUrl::App("index.html?focus=1".into()))
            .title("Personal Copilot · Focus")
            .theme(Some(tauri::Theme::Dark))
            .inner_size(420.0, 550.0)
            .min_inner_size(340.0, 420.0)
            .build()
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn pin_focus(window: WebviewWindow, pinned: bool) -> Result<(), String> {
    if window.label() != "focus" {
        return Err("Open the focus window first.".into());
    }
    window
        .set_always_on_top(pinned)
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn show_workspace(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("The main workspace has been closed.")?;
    window
        .show()
        .and_then(|_| window.set_focus())
        .map_err(|err| err.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = if let Some(local) =
                std::env::var_os("COPILOT_DATA_DIR").filter(|path| !path.is_empty())
            {
                std::path::PathBuf::from(local)
            } else {
                app.path().app_local_data_dir()?
            };
            app.manage(history::HistoryStore::new(directory));
            let state = Desktop::shared();
            app.manage(Arc::clone(&state));
            let handle = app.handle().clone();
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_millis(50));
                    let view = state.lock().expect("desktop session lock").tick();
                    if let Some(view) = view {
                        emit(&handle, &view);
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            list_endpoints,
            start_sample,
            start_capture,
            resume_capture,
            pause_capture,
            stop_capture,
            ask,
            request_detected,
            cancel_answer,
            history::save_session,
            history::list_saved_sessions,
            history::read_saved_session,
            history::delete_saved_session,
            history::save_session_notes,
            history::export_saved_session,
            use_transcript,
            clear_session,
            open_focus,
            pin_focus,
            show_workspace
        ])
        .run(tauri::generate_context!())
        .expect("run Personal Copilot desktop");
}
