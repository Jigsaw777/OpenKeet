use std::sync::Arc;

use tauri::{AppHandle, State, async_runtime::spawn_blocking};

use crate::{
    platform::open_browser,
    providers::{Connections, Model, ProviderHub, ProviderKind, Selection},
    workspace::{Shared, View, update},
};

#[tauri::command]
pub fn provider_connections(hub: State<'_, Arc<ProviderHub>>) -> Connections {
    hub.connections()
}

#[tauri::command]
pub async fn provider_models(
    hub: State<'_, Arc<ProviderHub>>,
    kind: ProviderKind,
) -> Result<Vec<Model>, String> {
    hub.models(kind).await
}

#[tauri::command]
pub async fn select_answer_provider(
    app: AppHandle,
    state: State<'_, Shared>,
    hub: State<'_, Arc<ProviderHub>>,
    route: Selection,
    consent: bool,
) -> Result<View, String> {
    let state = Arc::clone(state.inner());
    let hub = Arc::clone(hub.inner());
    spawn_blocking(move || {
        update(&app, &state, |desktop| {
            desktop.select_provider(&hub, route, consent)
        })
    })
    .await
    .map_err(|_| "Could not save the answer provider.")?
}

#[tauri::command]
pub fn open_claude_console() -> Result<(), String> {
    open_browser("https://platform.claude.com/settings/keys")
}

#[tauri::command]
pub async fn disable_answer_provider(
    app: AppHandle,
    state: State<'_, Shared>,
    hub: State<'_, Arc<ProviderHub>>,
) -> Result<View, String> {
    let state = Arc::clone(state.inner());
    let hub = Arc::clone(hub.inner());
    spawn_blocking(move || {
        hub.disable()?;
        update(&app, &state, |desktop| {
            desktop.sync_provider(hub.selection());
            Ok(())
        })
    })
    .await
    .map_err(|_| "Could not disable the answer provider.")?
}

#[tauri::command]
pub async fn save_claude_key(
    app: AppHandle,
    state: State<'_, Shared>,
    hub: State<'_, Arc<ProviderHub>>,
    key: String,
) -> Result<Connections, String> {
    let state = Arc::clone(state.inner());
    let hub = Arc::clone(hub.inner());
    spawn_blocking(move || {
        hub.save_claude(key)?;
        synchronized(&app, &state, &hub)
    })
    .await
    .map_err(|_| "Could not save the Claude connection.")?
}

#[tauri::command]
pub async fn clear_claude_key(
    app: AppHandle,
    state: State<'_, Shared>,
    hub: State<'_, Arc<ProviderHub>>,
) -> Result<Connections, String> {
    let state = Arc::clone(state.inner());
    let hub = Arc::clone(hub.inner());
    spawn_blocking(move || {
        hub.clear_claude()?;
        synchronized(&app, &state, &hub)
    })
    .await
    .map_err(|_| "Could not remove the Claude connection.")?
}

#[tauri::command]
pub async fn sign_in_chatgpt(
    app: AppHandle,
    state: State<'_, Shared>,
    hub: State<'_, Arc<ProviderHub>>,
    different_account: bool,
) -> Result<Connections, String> {
    hub.sign_in_chatgpt(different_account).await?;
    synchronized(&app, &state, &hub)
}

#[tauri::command]
pub fn cancel_chatgpt_sign_in(hub: State<'_, Arc<ProviderHub>>) {
    hub.cancel_sign_in();
}

#[tauri::command]
pub async fn sign_out_chatgpt(
    app: AppHandle,
    state: State<'_, Shared>,
    hub: State<'_, Arc<ProviderHub>>,
) -> Result<Connections, String> {
    let old = hub.clear_chatgpt().await?;
    let state = synchronized(&app, &state, &hub)?;
    hub.revoke_chatgpt(old).await?;
    Ok(state)
}

fn synchronized(app: &AppHandle, state: &Shared, hub: &ProviderHub) -> Result<Connections, String> {
    update(app, state, |desktop| {
        desktop.sync_provider(hub.selection());
        Ok(())
    })?;
    Ok(hub.connections())
}
