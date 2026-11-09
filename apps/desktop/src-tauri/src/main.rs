#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod bridge;
mod history;
mod platform;
mod prep;
mod provider_commands;
mod providers;
mod speech_setup;
mod workspace;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use copilot_audio::Endpoint;
use tauri::{
    AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use workspace::{CaptureConfig, Desktop, Shared, View, emit, update};

#[derive(Default)]
struct LastSurface(AtomicBool);

#[derive(Default)]
struct CapturePrivacy(AtomicBool);

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
fn start_capture(
    app: AppHandle,
    state: State<'_, Shared>,
    config: CaptureConfig,
) -> Result<View, String> {
    workspace::start_audio(app, Arc::clone(&state), Some(config))
}

#[tauri::command]
fn resume_capture(app: AppHandle, state: State<'_, Shared>) -> Result<View, String> {
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

/// Answers a forming question with one of the pack interpretations the user picked.
#[tauri::command]
fn use_prepared(
    app: AppHandle,
    state: State<'_, Shared>,
    family_id: String,
) -> Result<View, String> {
    update(&app, &state, |desktop| desktop.choose_candidate(&family_id))
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
fn screen_share_privacy(app: AppHandle, enabled: Option<bool>) -> Result<Option<bool>, String> {
    if !cfg!(windows) {
        return Ok(None);
    }
    let privacy = app.state::<CapturePrivacy>();
    let previous = privacy.0.load(Ordering::Relaxed);
    let Some(enabled) = enabled else {
        return Ok(Some(previous));
    };
    let main = app
        .get_webview_window("main")
        .ok_or("Could not find the OpenKeet workspace.")?;
    platform::set_capture_exclusion(&main, enabled)?;
    if let Some(focus) = app.get_webview_window("focus")
        && let Err(error) = platform::set_capture_exclusion(&focus, enabled)
    {
        platform::set_capture_exclusion(&main, previous).map_err(|restore| {
            format!("{error} Could not restore the workspace setting. {restore}")
        })?;
        return Err(error);
    }
    privacy.0.store(enabled, Ordering::Relaxed);
    Ok(Some(enabled))
}

#[tauri::command]
async fn open_focus(app: AppHandle) -> Result<(), String> {
    if app.get_webview_window("focus").is_none() {
        let window =
            WebviewWindowBuilder::new(&app, "focus", WebviewUrl::App("index.html?focus=1".into()))
                .title("OpenKeet · Focus")
                .theme(Some(tauri::Theme::Dark))
                .inner_size(420.0, 510.0)
                .min_inner_size(340.0, 420.0)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .build()
                .map_err(|err| err.to_string())?;
        let (configured, ready) = tokio::sync::oneshot::channel();
        let target = window.clone();
        window
            .run_on_main_thread(move || {
                let result = platform::configure_tool_window(&target).and_then(|()| {
                    if target
                        .app_handle()
                        .state::<CapturePrivacy>()
                        .0
                        .load(Ordering::Relaxed)
                    {
                        platform::set_capture_exclusion(&target, true)?;
                    }
                    Ok(())
                });
                let _ = configured.send(result);
            })
            .map_err(|error| error.to_string())?;
        if let Err(error) = ready.await.map_err(|_| "Could not prepare Focus window.")? {
            let _ = window.destroy();
            return Err(error);
        }
    }
    show_surface(&app, "focus")
}

#[tauri::command]
fn show_workspace(app: AppHandle) -> Result<(), String> {
    show_surface(&app, "main")
}

#[tauri::command]
fn hide_workspace(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|error| error.to_string())?;
    platform::memory_target(&window, true);
    let app = window.app_handle();
    let state = app.state::<Shared>();
    update(app, &state, Desktop::pause_when_hidden)?;
    Ok(())
}

fn restore_last_surface(app: &AppHandle) {
    let overlay = app
        .try_state::<LastSurface>()
        .is_some_and(|state| state.0.load(Ordering::Relaxed));
    if !overlay || show_surface(app, "focus").is_err() {
        let _ = show_surface(app, "main");
    }
}

fn show_surface(app: &AppHandle, label: &str) -> Result<(), String> {
    let window = app
        .get_webview_window(label)
        .ok_or("Could not find this OpenKeet window.")?;
    reveal(&window)?;
    let overlay = label == "focus";
    if let Some(other) = app.get_webview_window(if overlay { "main" } else { "focus" }) {
        other.hide().map_err(|err| err.to_string())?;
        platform::memory_target(&other, true);
    }
    if let Some(state) = app.try_state::<LastSurface>() {
        state.0.store(overlay, Ordering::Relaxed);
    }
    Ok(())
}

fn reveal(window: &WebviewWindow) -> Result<(), String> {
    platform::memory_target(window, false);
    window
        .show()
        .and_then(|_| window.unminimize())
        .and_then(|_| window.set_focus())
        .map_err(|err| err.to_string())
}

fn install_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show OpenKeet", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit OpenKeet", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    // The window icon is the 256 px ICO entry; the tray gets the simplified small-size mark.
    TrayIconBuilder::with_id("copilot")
        .icon(tauri::include_image!("icons/tray.png"))
        .tooltip("OpenKeet")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                restore_last_surface(app);
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
                restore_last_surface(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            restore_last_surface(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        restore_last_surface(app);
                    }
                })
                .build(),
        )
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && let Some(surface) = window.app_handle().get_webview_window(window.label())
            {
                api.prevent_close();
                let _ = hide_workspace(surface);
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
            bridge::start(std::env::args_os().skip(1), Arc::clone(&providers))?;
            app.manage(speech_setup::SpeechSetup::new(directory.clone()));
            app.manage(prep::PrepState::new(directory.clone()));
            app.manage(history::HistoryStore::new(directory));
            app.manage(Arc::clone(&providers));
            install_tray(app)?;
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyI);
            let shortcut_available = app.global_shortcut().register(shortcut).is_ok();
            let state = Desktop::shared();
            {
                let mut desktop = state.lock().expect("desktop session lock");
                desktop.sync_provider(providers.selection());
                desktop.set_shortcut_available(shortcut_available);
            }
            app.manage(Arc::clone(&state));
            app.manage(LastSurface::default());
            app.manage(CapturePrivacy::default());
            let main = app
                .get_webview_window("main")
                .ok_or("Missing workspace window.")?;
            platform::configure_tool_window(&main).map_err(std::io::Error::other)?;
            reveal(&main).map_err(std::io::Error::other)?;
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
            screen_share_privacy,
            list_endpoints,
            speech_setup::check_speech_model,
            speech_setup::find_speech_model,
            speech_setup::download_speech_model,
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
            prep::prep,
            prep::prep_draft_with_ai,
            prep::prep_ai,
            prep::prep_paraphrase,
            use_prepared,
            use_transcript,
            clear_session,
            open_focus,
            hide_workspace,
            show_workspace
        ])
        .run(tauri::generate_context!())
        .expect("run OpenKeet desktop");
}
