//! IPC commands exposed to the frontend.

use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::config::{self, Settings};
use crate::skill::MultiTargetStatus;
use crate::state::{AppState, BallState};

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    app.state::<AppState>().settings.lock().unwrap().clone()
}

/// Replace the full settings object, persist, and re-sync side effects
/// (autostart, skill status).
#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    // Persist first (atomic). If it fails, don't touch in-memory state.
    config::save(&settings).map_err(|e| e.to_string())?;

    {
        let st = app.state::<AppState>();
        *st.settings.lock().unwrap() = settings.clone();
        st.config_error.store(false, Ordering::Relaxed);
    }

    crate::autostart::sync_from_config(&app, settings.companion.autostart);
    crate::refresh_skill_status(&app);
    Ok(())
}

#[tauri::command]
pub fn get_ball_state(app: AppHandle) -> BallState {
    *app.state::<AppState>().ball_state.lock().unwrap()
}

#[tauri::command]
pub fn get_agent_present(app: AppHandle) -> bool {
    app.state::<AppState>()
        .agent_present
        .load(Ordering::Relaxed)
}

#[tauri::command]
pub fn get_active_targets(app: AppHandle) -> Vec<String> {
    app.state::<AppState>()
        .active_targets
        .lock()
        .unwrap()
        .clone()
}

/// Toggle the settings popover (used by the floating ball click).
#[tauri::command]
pub fn open_settings(app: AppHandle) {
    crate::tray::toggle_settings(&app);
}

#[tauri::command]
pub fn open_agent_settings(app: AppHandle, target_id: String) {
    crate::tray::show_agent_settings(&app, &target_id);
}

/// Hide the current settings page without stopping the background companion.
#[tauri::command]
pub fn close_settings(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_skill_status(app: AppHandle) -> MultiTargetStatus {
    app.state::<AppState>().skill_status.lock().unwrap().clone()
}

/// Re-detect Skill status and recompute the ball.
#[tauri::command]
pub fn refresh_skill_status_cmd(app: AppHandle) -> MultiTargetStatus {
    crate::refresh_skill_status(&app);
    app.state::<AppState>().skill_status.lock().unwrap().clone()
}

/// (Re)install the Skill from the bundled resource into all enabled targets.
#[tauri::command]
pub fn reinstall_skill(app: AppHandle) -> Result<MultiTargetStatus, String> {
    let targets = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .skill
        .targets
        .clone();
    let status = reinstall_skill_targets(&app, targets)?;

    Ok(status)
}

#[tauri::command]
pub fn reinstall_skill_for_target(
    app: AppHandle,
    target_id: String,
) -> Result<MultiTargetStatus, String> {
    reinstall_skill_targets(&app, vec![target_id])
}

fn reinstall_skill_targets(app: &AppHandle, targets: Vec<String>) -> Result<MultiTargetStatus, String> {
    let _installed = crate::skill::install_skill(&app, &targets)?;

    {
        let st = app.state::<AppState>();
        let full_targets = st.settings.lock().unwrap().skill.targets.clone();
        let status = crate::skill::detect_status(&full_targets);
        *st.skill_status.lock().unwrap() = status.clone();
        {
            let mut s = st.settings.lock().unwrap();
            s.skill.installed_version = status.aggregate().installed_version().map(String::from);
            config::save(&s).map_err(|e| e.to_string())?;
        }
    }

    let ns = app.state::<AppState>().compute_state();
    let changed = {
        let st = app.state::<AppState>();
        let mut bs = st.ball_state.lock().unwrap();
        let c = *bs != ns;
        *bs = ns;
        c
    };
    if changed {
        crate::ball::apply_state(&app, ns);
    }
    Ok(app.state::<AppState>().skill_status.lock().unwrap().clone())
}

#[tauri::command]
pub fn set_pause_detection(app: AppHandle, paused: bool) -> Result<(), String> {
    crate::apply_pause(&app, paused)
}

#[tauri::command]
pub async fn get_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    crate::autostart::is_enabled(&app)
}

#[tauri::command]
pub async fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        crate::autostart::enable(&app)
    } else {
        crate::autostart::disable(&app)
    }?;
    // Persist into settings too.
    let mut s = app.state::<AppState>().settings.lock().unwrap().clone();
    s.companion.autostart = enabled;
    config::save(&s).map_err(|e| e.to_string())?;
    *app.state::<AppState>().settings.lock().unwrap() = s;
    Ok(())
}

/// Launch the configured Agent (used by the "Oasis" shortcut).
#[tauri::command]
pub fn launch_agent(app: AppHandle) {
    crate::launch_agent(&app);
}

#[tauri::command]
pub async fn check_updates(app: AppHandle) -> crate::updater::UpdateStatus {
    crate::refresh_update_status(app).await
}

#[tauri::command]
pub async fn install_latest_update(
    app: AppHandle,
) -> Result<crate::updater::UpdateInstallResult, String> {
    crate::updater::install_latest(&app).await
}

#[tauri::command]
pub async fn check_mcp_status(app: AppHandle) -> crate::mcp::McpStatus {
    crate::refresh_mcp_status(app).await
}

/// One-click connect: force-enable + auto-discover + full handshake.
/// No need to manually toggle switches or fill in the address first.
#[tauri::command]
pub async fn connect_mcp_auto(app: AppHandle) -> crate::mcp::McpStatus {
    crate::mcp::connect_auto(&app).await
}

#[tauri::command]
pub fn disable_mcp(app: AppHandle) -> crate::mcp::McpStatus {
    crate::mcp::disable(&app)
}

/// Auto-discover the editor's MCP endpoint and persist it to settings.
#[tauri::command]
pub async fn discover_mcp(
    app: AppHandle,
) -> Result<Option<crate::mcp::DiscoveredEndpoint>, String> {
    let ep = crate::mcp::discover(&app).await?;
    if let Some(ref ep) = ep {
        let mut s = app.state::<AppState>().settings.lock().unwrap().clone();
        s.skill_runtime.mcp.host = ep.host.clone();
        s.skill_runtime.mcp.port = ep.port;
        s.skill_runtime.mcp.sse_path = ep.sse_path.clone();
        config::save(&s).map_err(|e| e.to_string())?;
        *app.state::<AppState>().settings.lock().unwrap() = s;
        log::info!(
            "mcp endpoint discovered via {}: {}:{}{}",
            ep.source,
            ep.host,
            ep.port,
            ep.sse_path
        );
    }
    Ok(ep)
}

/// Return the cached tool list from the last successful `check`.
#[tauri::command]
pub fn list_mcp_tools(app: AppHandle) -> Vec<crate::config::McpToolInfo> {
    app.state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .skill_runtime
        .mcp
        .cached_tools
        .clone()
}

/// Call an MCP tool (one-shot session). Use `ue_read` for safe diagnostics;
/// `ue_py`/`ue_plan_submit` mutate editor state.
#[tauri::command]
pub async fn call_mcp_tool(
    app: AppHandle,
    name: String,
    arguments: serde_json::Value,
) -> Result<crate::mcp::ToolCallResult, String> {
    crate::mcp::call_tool(&app, &name, arguments).await
}
