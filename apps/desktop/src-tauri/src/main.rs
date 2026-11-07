#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod history;
mod platform;
mod provider_commands;
mod providers;
mod workspace;

use std::{sync::Arc, thread, time::Duration};

use copilot_audio::Endpoint;
use tauri::{
    AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
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
    let window = if let Some(window) = app.get_webview_window("focus") {
        window
    } else {
        let window =
            WebviewWindowBuilder::new(&app, "focus", WebviewUrl::App("index.html?focus=1".into()))
                .title("Personal Copilot · Focus")
                .theme(Some(tauri::Theme::Dark))
                .inner_size(420.0, 550.0)
                .min_inner_size(340.0, 420.0)
                .skip_taskbar(true)
                .visible(false)
                .build()
                .map_err(|err| err.to_string())?;
        let (configured, ready) = tokio::sync::oneshot::channel();
        let target = window.clone();
        window
            .run_on_main_thread(move || {
                let _ = configured.send(platform::configure_tool_window(&target));
            })
            .map_err(|error| error.to_string())?;
        if let Err(error) = ready.await.map_err(|_| "Could not prepare Focus window.")? {
            let _ = window.close();
            return Err(error);
        }
        window
    };
    reveal(window)
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
    reveal_workspace(&app)
}

#[tauri::command]
fn hide_workspace(window: WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Hide the workspace from its own window.".into());
    }
    window.hide().map_err(|error| error.to_string())
}

fn reveal_workspace(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("The main workspace has been closed.")?;
    reveal(window)
}

fn reveal(window: WebviewWindow) -> Result<(), String> {
    window
        .show()
        .and_then(|_| window.unminimize())
        .and_then(|_| window.set_skip_taskbar(true))
        .and_then(|_| window.set_focus())
        .map_err(|err| err.to_string())
}

fn install_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Personal Copilot", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Personal Copilot", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let mut tray = TrayIconBuilder::with_id("copilot")
        .tooltip("Personal Copilot")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                let _ = reveal_workspace(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = reveal_workspace(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        let _ = reveal_workspace(app);
                    }
                })
                .build(),
        )
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, WindowEvent::CloseRequested { .. }) {
                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            let directory = if let Some(local) =
                std::env::var_os("COPILOT_DATA_DIR").filter(|path| !path.is_empty())
            {
                std::path::PathBuf::from(local)
            } else {
                app.path().app_local_data_dir()?
            };
            let providers = Arc::new(
                providers::ProviderHub::new(directory.clone()).map_err(std::io::Error::other)?,
            );
            app.manage(history::HistoryStore::new(directory));
            app.manage(Arc::clone(&providers));
            install_tray(app)?;
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyI);
            let shortcut_available = app.global_shortcut().register(shortcut).is_ok();
            let state = Desktop::shared();
            {
                let mut desktop = state.lock().expect("desktop session lock");
                desktop.sync_provider(providers.selection());
                desktop.set_shortcut_available(shortcut_available);
            }
            app.manage(Arc::clone(&state));
            let main = app
                .get_webview_window("main")
                .ok_or("Missing workspace window.")?;
            platform::configure_tool_window(&main).map_err(std::io::Error::other)?;
            reveal(main).map_err(std::io::Error::other)?;
            let handle = app.handle().clone();
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_millis(50));
                    let (view, generation) = state.lock().expect("desktop session lock").tick();
                    if let Some(view) = view {
                        emit(&handle, &view);
                    }
                    if let Some(generation) = generation {
                        workspace::run_generation(
                            handle.clone(),
                            Arc::clone(&state),
                            Arc::clone(&providers),
                            generation,
                        );
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
            provider_commands::provider_connections,
            provider_commands::provider_models,
            provider_commands::select_answer_provider,
            provider_commands::disable_answer_provider,
            provider_commands::open_claude_console,
            provider_commands::save_claude_key,
            provider_commands::clear_claude_key,
            provider_commands::sign_in_chatgpt,
            provider_commands::cancel_chatgpt_sign_in,
            provider_commands::sign_out_chatgpt,
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
            hide_workspace,
            show_workspace
        ])
        .run(tauri::generate_context!())
        .expect("run Personal Copilot desktop");
}
