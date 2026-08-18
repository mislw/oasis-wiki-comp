//! IPC commands exposed to the frontend.

use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::config::{self, Settings};
use crate::skill::MultiTargetStatus;
use crate::state::{AppState, BallState};
use crate::ui_delivery_preflight::{DeliveryPreflightState, WidgetBlueprintSearchResult};
use crate::ui_workbench_catalog::{self, LoadedWorkbenchPageView, WorkbenchCatalogView};
use crate::ui_workflow::{self, UiWorkflowStore};

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

/// Return the latest validated external UI Workbench session URL.
#[tauri::command]
pub fn get_pending_ui_workbench_url(app: AppHandle) -> Option<String> {
    app.state::<AppState>()
        .pending_ui_workbench_url
        .lock()
        .unwrap()
        .clone()
}

/// Return generated UI pages for the left navigation rail.
#[tauri::command]
pub fn list_ui_workbench_pages(app: AppHandle) -> WorkbenchCatalogView {
    let catalog = app
        .state::<AppState>()
        .ui_workbench_catalog
        .lock()
        .unwrap()
        .clone();
    ui_workbench_catalog::catalog_view(&catalog)
}

/// Persist and return the selected generated UI page.
#[tauri::command]
pub fn select_ui_workbench_page(
    app: AppHandle,
    page_id: String,
) -> Result<WorkbenchCatalogView, String> {
    let state = app.state::<AppState>();
    let mut next = state.ui_workbench_catalog.lock().unwrap().clone();
    ui_workbench_catalog::select_page(&mut next, &page_id)?;
    ui_workbench_catalog::save_catalog(&next)?;
    let view = ui_workbench_catalog::catalog_view(&next);
    *state.ui_workbench_catalog.lock().unwrap() = next;
    Ok(view)
}

/// Load one generated page's current session JSON.
#[tauri::command]
pub fn load_ui_workbench_page(
    app: AppHandle,
    page_id: String,
) -> Result<LoadedWorkbenchPageView, String> {
    let catalog = app
        .state::<AppState>()
        .ui_workbench_catalog
        .lock()
        .unwrap()
        .clone();
    ui_workbench_catalog::load_page_view(&catalog, &page_id)
}

/// Read one asset contained by a registered generated page.
#[tauri::command]
pub fn read_ui_workbench_asset(
    app: AppHandle,
    page_id: String,
    asset_path: String,
) -> Result<String, String> {
    let catalog = app
        .state::<AppState>()
        .ui_workbench_catalog
        .lock()
        .unwrap()
        .clone();
    ui_workbench_catalog::read_page_asset(&catalog, &page_id, &asset_path)
}

/// Open the UI Workbench without requiring a newly generated session.
#[tauri::command]
pub fn open_ui_workbench(app: AppHandle) -> Result<(), String> {
    crate::ui_workbench::show_ui_workbench(&app).map(|_| ())
}

/// Return the persistent eight-stage UI workflow store.
#[tauri::command]
pub fn list_ui_workflow_tasks(app: AppHandle) -> UiWorkflowStore {
    app.state::<AppState>()
        .ui_workflow_store
        .lock()
        .unwrap()
        .clone()
}

/// Persist the selected UI workflow task.
#[tauri::command]
pub fn select_ui_workflow_task(app: AppHandle, task_id: String) -> Result<UiWorkflowStore, String> {
    let state = app.state::<AppState>();
    let mut next = state.ui_workflow_store.lock().unwrap().clone();
    if !next.tasks.iter().any(|task| task.task_id == task_id) {
        return Err(format!("unknown UI workflow task: {task_id}"));
    }
    next.selected_task_id = Some(task_id);
    ui_workflow::save_store(&next)?;
    *state.ui_workflow_store.lock().unwrap() = next.clone();
    Ok(next)
}

/// Update the editor target for one UI workflow task.
#[tauri::command]
pub fn update_ui_workflow_target(
    app: AppHandle,
    task_id: String,
    project_workspace: String,
    widget_blueprint: String,
) -> Result<UiWorkflowStore, String> {
    let state = app.state::<AppState>();
    let mut next = state.ui_workflow_store.lock().unwrap().clone();
    let task = next
        .tasks
        .iter_mut()
        .find(|task| task.task_id == task_id)
        .ok_or_else(|| format!("unknown UI workflow task: {task_id}"))?;
    task.target.project_workspace = project_workspace.trim().to_owned();
    task.target.widget_blueprint = widget_blueprint.trim().to_owned();
    task.target.widget_blueprint_name.clear();
    task.target.widget_blueprint_class.clear();
    task.target.preflight = None;
    ui_workflow::save_store(&next)?;
    *state.ui_workflow_store.lock().unwrap() = next.clone();
    Ok(next)
}

#[tauri::command]
pub async fn search_widget_blueprints(
    app: AppHandle,
    task_id: String,
    project_workspace: String,
    query: String,
) -> Result<WidgetBlueprintSearchResult, String> {
    if !app
        .state::<AppState>()
        .ui_workflow_store
        .lock()
        .unwrap()
        .tasks
        .iter()
        .any(|task| task.task_id == task_id)
    {
        return Err(format!("unknown UI workflow task: {task_id}"));
    }
    crate::ui_delivery_preflight::search_widget_blueprints(&app, &project_workspace, &query)
        .await
        .map(|(result, _)| result)
}

#[tauri::command]
pub async fn preflight_ui_delivery(
    app: AppHandle,
    task_id: String,
    project_workspace: String,
    selected_load_path: String,
) -> Result<UiWorkflowStore, String> {
    let now = current_unix_ms();
    let (candidate, evidence) = crate::ui_delivery_preflight::validate_exact_widget_blueprint(
        &app,
        &task_id,
        &project_workspace,
        &selected_load_path,
        now,
    )
    .await?;
    let state = app.state::<AppState>();
    let mut next = state.ui_workflow_store.lock().unwrap().clone();
    let task = next
        .tasks
        .iter_mut()
        .find(|task| task.task_id == task_id)
        .ok_or_else(|| format!("unknown UI workflow task: {task_id}"))?;
    task.target.project_workspace = project_workspace.trim().to_owned();
    task.target.widget_blueprint = candidate.load_path;
    task.target.widget_blueprint_name = candidate.display_name;
    task.target.widget_blueprint_class = candidate.class_name;
    task.target.preflight = Some(evidence);
    task.updated_at_unix_ms = now;
    ui_workflow::save_store(&next)?;
    *state.ui_workflow_store.lock().unwrap() = next.clone();
    app.emit("ui-workflow://progress", next.clone())
        .map_err(|error| format!("could not emit UI workflow progress: {error}"))?;
    Ok(next)
}

/// Open the native eight-stage UI workflow window.
#[tauri::command]
pub fn open_ui_workflow(app: AppHandle) -> Result<(), String> {
    crate::ui_workflow::show_ui_workflow(&app).map(|_| ())
}

#[derive(Debug, Clone, Serialize)]
pub struct DeliveryDispatchResult {
    pub delivery_id: String,
    pub request_path: String,
    pub new_task_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexPromptSubmissionResult {
    pub submitted: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UiSourceDispatchResult {
    pub new_task_url: String,
}

/// Prepare a new Codex task for a fresh generated or imported UI source.
#[tauri::command]
pub fn start_ui_source_task(
    project_workspace: String,
    source_mode: String,
) -> Result<UiSourceDispatchResult, String> {
    let workspace = PathBuf::from(project_workspace.trim());
    if !workspace.is_dir() {
        return Err("project workspace is unavailable".into());
    }
    let mode = crate::codex_delivery::UiSourceMode::parse(&source_mode)?;
    let prompt = crate::codex_delivery::build_ui_source_prompt(mode);
    let new_task_url = crate::codex_delivery::build_codex_new_task_url(&workspace, &prompt)?;
    Ok(UiSourceDispatchResult { new_task_url })
}

/// Submit a prefilled UI source task only while packaged Codex Desktop owns the foreground.
#[tauri::command]
pub async fn submit_codex_ui_source_prompt() -> Result<CodexPromptSubmissionResult, String> {
    let submitted = tauri::async_runtime::spawn_blocking(|| {
        crate::codex_delivery::submit_foreground_codex_prompt(Duration::from_secs(6))
    })
    .await
    .map_err(|error| format!("Codex foreground verification task failed: {error}"))??;
    let message = if submitted {
        "已新建 Codex 任务并开始 UI 流程"
    } else {
        "新 Codex 任务已打开并预填 UI 指令，请按 Enter 开始"
    };
    Ok(CodexPromptSubmissionResult {
        submitted,
        message: message.into(),
    })
}

/// Freeze the current Workbench tree and prepare a new Desktop-owned Codex task.
#[tauri::command]
pub async fn confirm_and_deliver_ui(
    app: AppHandle,
    page_id: String,
    tree: serde_json::Value,
    evidence_id: String,
) -> Result<DeliveryDispatchResult, String> {
    let state = app.state::<AppState>();
    let mut next = state.ui_workflow_store.lock().unwrap().clone();
    let snapshot = next
        .tasks
        .iter()
        .find(|task| task.page_id == page_id)
        .cloned()
        .ok_or_else(|| format!("unknown UI workflow page: {page_id}"))?;
    if snapshot.stages.umg.status == crate::ui_workflow::WorkflowStatus::InProgress {
        return Err("this UI task already has an active editor delivery".into());
    }
    let saved_evidence = snapshot
        .target
        .preflight
        .as_ref()
        .filter(|evidence| {
            evidence.status == DeliveryPreflightState::Ready && evidence.evidence_id == evidence_id
        })
        .ok_or_else(|| "delivery preflight is missing or stale".to_string())?;
    let now = current_unix_ms();
    let (candidate, fresh_evidence) =
        crate::ui_delivery_preflight::validate_exact_widget_blueprint(
            &app,
            &snapshot.task_id,
            &snapshot.target.project_workspace,
            &snapshot.target.widget_blueprint,
            now,
        )
        .await?;
    if candidate.load_path != saved_evidence.selected_load_path {
        return Err("target WidgetBlueprint changed after preflight".into());
    }
    let task = next
        .tasks
        .iter_mut()
        .find(|task| task.page_id == page_id)
        .ok_or_else(|| format!("unknown UI workflow page: {page_id}"))?;
    task.target.widget_blueprint_name = candidate.display_name;
    task.target.widget_blueprint_class = candidate.class_name;
    task.target.preflight = Some(fresh_evidence);
    let agent = task
        .agent_context
        .clone()
        .ok_or_else(|| "workflow task has no originating Agent".to_string())?;
    let workspace_text = if task.target.project_workspace.trim().is_empty() {
        agent.workspace.clone()
    } else {
        task.target.project_workspace.clone()
    };
    let workspace = PathBuf::from(workspace_text);
    if !workspace.is_dir() {
        return Err("registered project workspace is unavailable".into());
    }
    let prepared = crate::ui_workflow::prepare_delivery(task, &tree, now)?;
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| format!("could not resolve Companion resources: {error}"))?;
    let reporter = crate::codex_delivery::bundled_reporter_path(&resource_dir);
    let prompt = crate::codex_delivery::build_delivery_prompt(
        &prepared.request_path,
        &reporter,
        &task.session_dir,
        &task.task_id,
    );
    let new_task_url = crate::codex_delivery::build_codex_new_task_url(&workspace, &prompt)?;
    task.stages.umg.status = crate::ui_workflow::WorkflowStatus::AwaitingConfirmation;
    task.stages.umg.message = "交付请求已冻结，正在新建 Codex 任务".into();
    task.latest_message = task.stages.umg.message.clone();
    crate::ui_workflow::save_store(&next)?;
    *state.ui_workflow_store.lock().unwrap() = next.clone();
    app.emit("ui-workflow://progress", next)
        .map_err(|error| format!("could not emit UI workflow progress: {error}"))?;
    Ok(DeliveryDispatchResult {
        delivery_id: prepared.delivery_id,
        request_path: prepared.request_path.to_string_lossy().into_owned(),
        new_task_url,
    })
}

/// Submit a prefilled new task only while packaged Codex Desktop owns the foreground.
#[tauri::command]
pub async fn submit_codex_new_task_prompt(
    app: AppHandle,
    page_id: String,
) -> Result<CodexPromptSubmissionResult, String> {
    let submitted = tauri::async_runtime::spawn_blocking(|| {
        crate::codex_delivery::submit_foreground_codex_prompt(Duration::from_secs(6))
    })
    .await
    .map_err(|error| format!("Codex foreground verification task failed: {error}"))??;
    let state = app.state::<AppState>();
    let mut next = state.ui_workflow_store.lock().unwrap().clone();
    let message = {
        let task = next
            .tasks
            .iter_mut()
            .find(|task| task.page_id == page_id)
            .ok_or_else(|| format!("unknown UI workflow page: {page_id}"))?;
        let now = current_unix_ms();
        task.stages.umg.updated_at_unix_ms = now;
        task.updated_at_unix_ms = now;
        if submitted {
            task.stages.umg.status = crate::ui_workflow::WorkflowStatus::InProgress;
            task.stages.umg.message = "已由新 Codex 任务开始实现".into();
        } else {
            task.stages.umg.status = crate::ui_workflow::WorkflowStatus::AwaitingConfirmation;
            task.stages.umg.message = "新 Codex 任务已打开并预填交付指令，请按 Enter 开始".into();
        }
        task.latest_message = task.stages.umg.message.clone();
        task.latest_message.clone()
    };
    crate::ui_workflow::save_store(&next)?;
    *state.ui_workflow_store.lock().unwrap() = next.clone();
    app.emit("ui-workflow://progress", next)
        .map_err(|error| format!("could not emit UI workflow progress: {error}"))?;
    Ok(CodexPromptSubmissionResult { submitted, message })
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

/// Toggle the settings popover (used by the floating ball click).
#[tauri::command]
pub fn open_settings(app: AppHandle) {
    crate::tray::toggle_settings(&app);
}

#[tauri::command]
pub fn open_agent_settings(app: AppHandle, target_id: String) -> Result<(), String> {
    crate::tray::show_agent_settings_inline(&app, &target_id)
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

fn reinstall_skill_targets(
    app: &AppHandle,
    targets: Vec<String>,
) -> Result<MultiTargetStatus, String> {
    let _installed = crate::skill::install_skill(app, &targets)?;

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
        crate::ball::apply_state(app, ns);
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
