//! Persistent per-page state for the Oasis UI production workflow.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const STORE_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStage {
    Source,
    UiTree,
    Visual,
    Layering,
    Workbench,
    Umg,
    Logic,
    Review,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    NotStarted,
    InProgress,
    AwaitingConfirmation,
    Completed,
    Blocked,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StageState {
    pub status: WorkflowStatus,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub updated_at_unix_ms: u64,
}

impl StageState {
    fn new(status: WorkflowStatus) -> Self {
        Self {
            status,
            message: String::new(),
            updated_at_unix_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowStages {
    pub source: StageState,
    pub ui_tree: StageState,
    pub visual: StageState,
    pub layering: StageState,
    pub workbench: StageState,
    pub umg: StageState,
    pub logic: StageState,
    pub review: StageState,
}

impl Default for WorkflowStages {
    fn default() -> Self {
        Self {
            source: StageState::new(WorkflowStatus::NotStarted),
            ui_tree: StageState::new(WorkflowStatus::NotStarted),
            visual: StageState::new(WorkflowStatus::NotStarted),
            layering: StageState::new(WorkflowStatus::NotStarted),
            workbench: StageState::new(WorkflowStatus::NotStarted),
            umg: StageState::new(WorkflowStatus::NotStarted),
            logic: StageState::new(WorkflowStatus::NotStarted),
            review: StageState::new(WorkflowStatus::NotStarted),
        }
    }
}

impl WorkflowStages {
    #[cfg(test)]
    pub fn ordered(&self) -> [(WorkflowStage, &StageState); 8] {
        [
            (WorkflowStage::Source, &self.source),
            (WorkflowStage::UiTree, &self.ui_tree),
            (WorkflowStage::Visual, &self.visual),
            (WorkflowStage::Layering, &self.layering),
            (WorkflowStage::Workbench, &self.workbench),
            (WorkflowStage::Umg, &self.umg),
            (WorkflowStage::Logic, &self.logic),
            (WorkflowStage::Review, &self.review),
        ]
    }

    pub fn get_mut(&mut self, stage: WorkflowStage) -> &mut StageState {
        match stage {
            WorkflowStage::Source => &mut self.source,
            WorkflowStage::UiTree => &mut self.ui_tree,
            WorkflowStage::Visual => &mut self.visual,
            WorkflowStage::Layering => &mut self.layering,
            WorkflowStage::Workbench => &mut self.workbench,
            WorkflowStage::Umg => &mut self.umg,
            WorkflowStage::Logic => &mut self.logic,
            WorkflowStage::Review => &mut self.review,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentContext {
    pub provider: String,
    pub thread_id: String,
    pub session_id: String,
    pub workspace: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowTarget {
    pub project_workspace: String,
    pub widget_blueprint: String,
    #[serde(default)]
    pub widget_blueprint_name: String,
    #[serde(default)]
    pub widget_blueprint_class: String,
    #[serde(default)]
    pub preflight: Option<crate::ui_delivery_preflight::DeliveryPreflightEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiWorkflowTask {
    pub task_id: String,
    pub page_id: String,
    pub title: String,
    pub session_dir: PathBuf,
    pub control_count: usize,
    pub agent_context: Option<AgentContext>,
    pub target: WorkflowTarget,
    pub stages: WorkflowStages,
    #[serde(default)]
    pub latest_message: String,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PreparedDelivery {
    pub delivery_id: String,
    pub request_path: PathBuf,
    pub tree_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiWorkflowStore {
    pub schema_version: u32,
    pub selected_task_id: Option<String>,
    pub tasks: Vec<UiWorkflowTask>,
}

impl Default for UiWorkflowStore {
    fn default() -> Self {
        Self {
            schema_version: STORE_SCHEMA_VERSION,
            selected_task_id: None,
            tasks: Vec::new(),
        }
    }
}

pub fn store_path() -> PathBuf {
    crate::config::settings_dir().join("ui-workflow-tasks.json")
}

pub fn load_store() -> UiWorkflowStore {
    let path = store_path();
    load_store_with_recovery(&path, unix_ms())
}

pub fn load_store_with_recovery(path: &Path, stamp: u64) -> UiWorkflowStore {
    match load_store_from_path(path) {
        Ok(store) => store,
        Err(error) => {
            log::warn!("UI workflow store recovery: {error}");
            if path.exists() {
                let file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("ui-workflow-tasks.json");
                let backup = path.with_file_name(format!("{file_name}.invalid-{stamp}"));
                if let Err(rename_error) = fs::rename(path, &backup) {
                    log::warn!("UI workflow store backup failed: {rename_error}");
                }
            }
            UiWorkflowStore::default()
        }
    }
}

pub fn load_store_from_path(path: &Path) -> Result<UiWorkflowStore, String> {
    if !path.exists() {
        return Ok(UiWorkflowStore::default());
    }
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("could not read workflow store: {error}"))?;
    let mut store: UiWorkflowStore = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid workflow store JSON: {error}"))?;
    if store.schema_version == 1 {
        store.schema_version = STORE_SCHEMA_VERSION;
        for task in &mut store.tasks {
            task.target.preflight = None;
        }
    } else if store.schema_version != STORE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported workflow store schema: {}",
            store.schema_version
        ));
    }
    reopen_legacy_cli_deliveries(&mut store);
    Ok(store)
}

fn reopen_legacy_cli_deliveries(store: &mut UiWorkflowStore) {
    const LEGACY_MESSAGE: &str = "已投递到原 Codex 对话，等待 MCP 实现";
    const RETRY_MESSAGE: &str = "旧版交付未确认，等待重新投递";
    for task in &mut store.tasks {
        if task.stages.umg.status == WorkflowStatus::InProgress
            && task.stages.umg.message == LEGACY_MESSAGE
        {
            task.stages.umg.status = WorkflowStatus::AwaitingConfirmation;
            task.stages.umg.message = RETRY_MESSAGE.into();
            task.latest_message = RETRY_MESSAGE.into();
        }
    }
}

pub fn save_store(store: &UiWorkflowStore) -> Result<(), String> {
    save_store_to_path(store, &store_path())
}

pub fn save_store_to_path(store: &UiWorkflowStore, path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "workflow store has no parent".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create workflow store directory: {error}"))?;
    let temporary = parent.join(".ui-workflow-tasks.json.tmp");
    let bytes = serde_json::to_vec_pretty(store)
        .map_err(|error| format!("could not serialize workflow store: {error}"))?;
    {
        let mut file = fs::File::create(&temporary)
            .map_err(|error| format!("could not create temporary workflow store: {error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("could not write workflow store: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("could not sync workflow store: {error}"))?;
    }
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("could not replace workflow store: {error}"))?;
    }
    fs::rename(&temporary, path)
        .map_err(|error| format!("could not install workflow store: {error}"))
}

pub fn register_session(
    store: &mut UiWorkflowStore,
    session_dir: &Path,
    updated_at_unix_ms: u64,
) -> Result<(), String> {
    let session_dir = session_dir
        .canonicalize()
        .map_err(|error| format!("workflow session directory is unavailable: {error}"))?;
    let raw = fs::read_to_string(session_dir.join("session.json"))
        .map_err(|error| format!("could not read workflow session: {error}"))?;
    let session: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid workflow session JSON: {error}"))?;
    let page_id = required_string(&session, "page_id")?;
    let title = required_string(&session, "title")?;
    let control_count = session
        .get("controls")
        .or_else(|| session.get("nodes"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let agent_context = session
        .get("agent_context")
        .filter(|value| value.is_object())
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| format!("invalid agent context: {error}"))?;
    if let Some(existing) = store.tasks.iter_mut().find(|task| task.task_id == page_id) {
        existing.title = title;
        existing.session_dir = session_dir;
        existing.control_count = control_count;
        if agent_context.is_some() {
            existing.agent_context = agent_context;
        }
        existing.updated_at_unix_ms = updated_at_unix_ms;
    } else {
        let mut stages = WorkflowStages::default();
        stages.source.status = WorkflowStatus::Completed;
        stages.ui_tree.status = if control_count > 0 {
            WorkflowStatus::Completed
        } else {
            WorkflowStatus::Blocked
        };
        stages.visual.status = WorkflowStatus::Completed;
        stages.layering.status = WorkflowStatus::Completed;
        stages.workbench.status = WorkflowStatus::AwaitingConfirmation;
        store.tasks.insert(
            0,
            UiWorkflowTask {
                task_id: page_id.clone(),
                page_id: page_id.clone(),
                title,
                session_dir,
                control_count,
                agent_context,
                target: WorkflowTarget::default(),
                stages,
                latest_message: "等待确认控件".into(),
                updated_at_unix_ms,
            },
        );
    }
    store.selected_task_id = Some(page_id);
    Ok(())
}

pub fn mark_downstream_stale(stages: &mut WorkflowStages, changed: WorkflowStage) {
    let downstream = match changed {
        WorkflowStage::Source => &[
            WorkflowStage::UiTree,
            WorkflowStage::Visual,
            WorkflowStage::Layering,
            WorkflowStage::Workbench,
            WorkflowStage::Umg,
            WorkflowStage::Logic,
            WorkflowStage::Review,
        ][..],
        WorkflowStage::UiTree | WorkflowStage::Visual => &[
            WorkflowStage::Layering,
            WorkflowStage::Workbench,
            WorkflowStage::Umg,
            WorkflowStage::Logic,
            WorkflowStage::Review,
        ][..],
        WorkflowStage::Layering => &[
            WorkflowStage::Workbench,
            WorkflowStage::Umg,
            WorkflowStage::Logic,
            WorkflowStage::Review,
        ][..],
        WorkflowStage::Workbench => &[
            WorkflowStage::Umg,
            WorkflowStage::Logic,
            WorkflowStage::Review,
        ][..],
        WorkflowStage::Umg => &[WorkflowStage::Logic, WorkflowStage::Review][..],
        WorkflowStage::Logic => &[WorkflowStage::Review][..],
        WorkflowStage::Review => &[][..],
    };
    for stage in downstream {
        let state = stages.get_mut(*stage);
        if matches!(
            state.status,
            WorkflowStatus::Completed | WorkflowStatus::InProgress
        ) {
            state.status = WorkflowStatus::Stale;
        }
    }
}

pub fn prepare_delivery(
    task: &mut UiWorkflowTask,
    tree: &Value,
    created_at_unix_ms: u64,
) -> Result<PreparedDelivery, String> {
    let agent = task
        .agent_context
        .as_ref()
        .ok_or_else(|| "workflow task has no originating Agent context".to_string())?;
    if agent.provider != "codex" || agent.thread_id.trim().is_empty() {
        return Err("automatic delivery requires a captured Codex thread".into());
    }
    if task.target.widget_blueprint.trim().is_empty() {
        return Err("target WidgetBlueprint is required".into());
    }
    let preflight = task
        .target
        .preflight
        .as_ref()
        .filter(|evidence| {
            evidence.status == crate::ui_delivery_preflight::DeliveryPreflightState::Ready
                && evidence.selected_load_path == task.target.widget_blueprint
        })
        .ok_or_else(|| "target WidgetBlueprint has no valid editor preflight".to_string())?;
    let nodes = tree
        .get("nodes")
        .or_else(|| tree.get("controls"))
        .and_then(Value::as_array)
        .ok_or_else(|| "confirmed UI Tree must contain nodes or controls".to_string())?;
    if nodes.is_empty() {
        return Err("confirmed UI Tree must contain at least one control".into());
    }
    let delivery_id = format!("delivery-{created_at_unix_ms}");
    let deliveries = task.session_dir.join("deliveries");
    fs::create_dir_all(&deliveries)
        .map_err(|error| format!("could not create delivery root: {error}"))?;
    let directory = deliveries.join(&delivery_id);
    fs::create_dir(&directory)
        .map_err(|error| format!("could not create immutable delivery directory: {error}"))?;
    let tree_path = directory.join("confirmed-ui-tree.json");
    let request_path = directory.join("delivery-request.json");
    fs::write(
        &tree_path,
        serde_json::to_vec_pretty(tree)
            .map_err(|error| format!("could not serialize confirmed UI Tree: {error}"))?,
    )
    .map_err(|error| format!("could not write confirmed UI Tree: {error}"))?;
    let request = serde_json::json!({
        "schema_version": 2,
        "artifact_type": "oasis_ui_editor_delivery",
        "delivery_id": delivery_id,
        "task_id": task.task_id,
        "page_id": task.page_id,
        "title": task.title,
        "control_count": nodes.len(),
        "agent_context": agent,
        "target": task.target,
        "preflight": preflight,
        "confirmed_ui_tree": "confirmed-ui-tree.json",
        "authorized_scope": "widget_blueprint",
        "created_at_unix_ms": created_at_unix_ms,
    });
    fs::write(
        &request_path,
        serde_json::to_vec_pretty(&request)
            .map_err(|error| format!("could not serialize delivery request: {error}"))?,
    )
    .map_err(|error| format!("could not write delivery request: {error}"))?;
    mark_downstream_stale(&mut task.stages, WorkflowStage::Workbench);
    task.control_count = nodes.len();
    task.stages.workbench.status = WorkflowStatus::Completed;
    task.stages.workbench.updated_at_unix_ms = created_at_unix_ms;
    task.latest_message = "Workbench 已确认，等待创建新 Codex 任务".into();
    task.updated_at_unix_ms = created_at_unix_ms;
    Ok(PreparedDelivery {
        delivery_id,
        request_path,
        tree_path,
    })
}

#[derive(Debug, Clone, Deserialize)]
struct WorkflowUpdateFile {
    schema_version: u32,
    task_id: String,
    stage: WorkflowStage,
    status: WorkflowStatus,
    #[serde(default)]
    message: String,
    #[serde(default)]
    artifacts: Vec<String>,
}

pub fn parse_update_arg(args: &[String]) -> Result<Option<PathBuf>, String> {
    let Some(index) = args.iter().position(|arg| arg == "--ui-workflow-update") else {
        return Ok(None);
    };
    let value = args
        .get(index + 1)
        .ok_or_else(|| "--ui-workflow-update requires a path".to_string())?;
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err("UI workflow update path must be absolute".into());
    }
    Ok(Some(path))
}

pub fn is_open_requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--ui-workflow")
}

pub fn apply_update_file(
    store: &mut UiWorkflowStore,
    update_path: &Path,
    updated_at_unix_ms: u64,
) -> Result<(), String> {
    let raw = fs::read_to_string(update_path)
        .map_err(|error| format!("could not read UI workflow update: {error}"))?;
    let update: WorkflowUpdateFile = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid UI workflow update: {error}"))?;
    if update.schema_version != 1 {
        return Err("unsupported UI workflow update schema".into());
    }
    let task = store
        .tasks
        .iter_mut()
        .find(|task| task.task_id == update.task_id)
        .ok_or_else(|| format!("unknown UI workflow task: {}", update.task_id))?;
    let session = task
        .session_dir
        .canonicalize()
        .map_err(|error| format!("workflow session directory is unavailable: {error}"))?;
    let update_path = update_path
        .canonicalize()
        .map_err(|error| format!("workflow update file is unavailable: {error}"))?;
    if !update_path.starts_with(&session) {
        return Err("workflow update file is outside the registered session".into());
    }
    for artifact in &update.artifacts {
        let relative = Path::new(artifact);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(format!("invalid workflow artifact path: {artifact}"));
        }
        let resolved = session.join(relative);
        if !resolved.is_file() {
            return Err(format!("workflow artifact is unavailable: {artifact}"));
        }
    }
    let prerequisite_ready = match update.stage {
        WorkflowStage::Umg => task.stages.workbench.status == WorkflowStatus::Completed,
        WorkflowStage::Logic => task.stages.umg.status == WorkflowStatus::Completed,
        WorkflowStage::Review => task.stages.logic.status == WorkflowStatus::Completed,
        _ => true,
    };
    if update.status == WorkflowStatus::Completed && !prerequisite_ready {
        return Err("workflow stage prerequisite is incomplete or stale".into());
    }
    let state = task.stages.get_mut(update.stage);
    state.status = update.status;
    state.message = update.message.clone();
    state.updated_at_unix_ms = updated_at_unix_ms;
    task.latest_message = update.message;
    task.updated_at_unix_ms = updated_at_unix_ms;
    store.selected_task_id = Some(task.task_id.clone());
    Ok(())
}

pub fn apply_update_handoff(app: &AppHandle, update_path: &Path) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut next = state.ui_workflow_store.lock().unwrap().clone();
    apply_update_file(&mut next, update_path, unix_ms())?;
    save_store(&next)?;
    *state.ui_workflow_store.lock().unwrap() = next.clone();
    app.emit("ui-workflow://progress", next)
        .map_err(|error| format!("could not emit UI workflow progress: {error}"))
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

fn required_string(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("workflow session must contain {field}"))
}

pub fn show_ui_workflow(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    let window = match app.get_webview_window("ui-workflow") {
        Some(window) => window,
        None => WebviewWindowBuilder::new(app, "ui-workflow", WebviewUrl::App("index.html".into()))
            .title("UI 生图工具链")
            .inner_size(980.0, 770.0)
            .min_inner_size(860.0, 660.0)
            .resizable(true)
            .decorations(true)
            .transparent(false)
            .build()
            .map_err(|error| format!("could not create UI workflow window: {error}"))?,
    };
    window
        .show()
        .map_err(|error| format!("could not show UI workflow window: {error}"))?;
    window
        .unminimize()
        .map_err(|error| format!("could not restore UI workflow window: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("could not focus UI workflow window: {error}"))?;
    Ok(window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("oasis-workflow-{name}-{nonce}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn detects_explicit_ui_workflow_open_request() {
        assert!(is_open_requested(&[
            "oasis-companion.exe".into(),
            "--ui-workflow".into(),
        ]));
        assert!(!is_open_requested(&[
            "oasis-companion.exe".into(),
            "--background".into(),
        ]));
    }

    fn ready_preflight(load_path: &str) -> crate::ui_delivery_preflight::DeliveryPreflightEvidence {
        crate::ui_delivery_preflight::DeliveryPreflightEvidence {
            status: crate::ui_delivery_preflight::DeliveryPreflightState::Ready,
            checked_at_unix_ms: 90,
            mcp_server_name: "UGCAskQ".into(),
            mcp_server_version: "1.0.0".into(),
            editor_project_root: "/RedCliff".into(),
            selected_load_path: load_path.into(),
            selected_class_name: "UGCWidgetBlueprint".into(),
            evidence_id: "sha256:test-evidence".into(),
            message: "目标 WidgetBlueprint 已通过编辑器只读预检".into(),
        }
    }

    #[test]
    fn registers_all_eight_stages_and_preserves_later_progress() {
        let root = temp_dir("register");
        let session = root.join("session");
        fs::create_dir_all(&session).unwrap();
        fs::write(
            session.join("session.json"),
            serde_json::to_vec_pretty(&json!({
                "page_id": "currency-exchange",
                "title": "货币兑换",
                "source_image": "source.png",
                "controls": [{"component_id": "button.exchange"}],
                "agent_context": {
                    "provider": "codex",
                    "thread_id": "thread-1",
                    "session_id": "thread-1",
                    "workspace": "E:\\RedCliff"
                }
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(session.join("source.png"), b"png").unwrap();
        let mut store = UiWorkflowStore::default();

        register_session(&mut store, &session, 10).unwrap();
        store.tasks[0].stages.umg.status = WorkflowStatus::Completed;
        register_session(&mut store, &session, 20).unwrap();

        assert_eq!(store.tasks[0].stages.ordered().len(), 8);
        assert_eq!(
            store.tasks[0].stages.workbench.status,
            WorkflowStatus::AwaitingConfirmation
        );
        assert_eq!(store.tasks[0].stages.umg.status, WorkflowStatus::Completed);
        assert_eq!(store.selected_task_id.as_deref(), Some("currency-exchange"));
    }

    #[test]
    fn backs_up_an_invalid_store_before_recovering() {
        let root = temp_dir("invalid-store");
        let path = root.join("ui-workflow-tasks.json");
        fs::write(&path, b"{invalid").unwrap();

        let store = load_store_with_recovery(&path, 123);

        assert_eq!(store, UiWorkflowStore::default());
        assert!(!path.exists());
        assert!(root.join("ui-workflow-tasks.json.invalid-123").is_file());
    }

    #[test]
    fn migrates_schema_one_without_trusting_legacy_widget_blueprint_text() {
        let root = temp_dir("schema-one");
        let path = root.join("ui-workflow-tasks.json");
        let load_path = "/RedCliff/Asset/UI/CurrencyExchange.CurrencyExchange";
        let store = UiWorkflowStore {
            schema_version: 1,
            selected_task_id: Some("currency-exchange".into()),
            tasks: vec![UiWorkflowTask {
                task_id: "currency-exchange".into(),
                page_id: "currency-exchange".into(),
                title: "货币兑换".into(),
                session_dir: root.clone(),
                control_count: 63,
                agent_context: None,
                target: WorkflowTarget {
                    project_workspace: r"E:\RedCliff".into(),
                    widget_blueprint: load_path.into(),
                    widget_blueprint_name: "CurrencyExchange".into(),
                    widget_blueprint_class: "UGCWidgetBlueprint".into(),
                    preflight: Some(ready_preflight(load_path)),
                },
                stages: WorkflowStages::default(),
                latest_message: String::new(),
                updated_at_unix_ms: 10,
            }],
        };
        save_store_to_path(&store, &path).unwrap();

        let loaded = load_store_from_path(&path).unwrap();

        assert_eq!(loaded.schema_version, STORE_SCHEMA_VERSION);
        assert_eq!(loaded.tasks[0].target.widget_blueprint, load_path);
        assert_eq!(loaded.tasks[0].target.preflight, None);
    }

    #[test]
    fn reopens_legacy_cli_deliveries_for_desktop_retry() {
        let root = temp_dir("legacy-delivery");
        let path = root.join("ui-workflow-tasks.json");
        let mut stages = WorkflowStages::default();
        stages.umg.status = WorkflowStatus::InProgress;
        stages.umg.message = "已投递到原 Codex 对话，等待 MCP 实现".into();
        let store = UiWorkflowStore {
            schema_version: STORE_SCHEMA_VERSION,
            selected_task_id: Some("currency-exchange".into()),
            tasks: vec![UiWorkflowTask {
                task_id: "currency-exchange".into(),
                page_id: "currency-exchange".into(),
                title: "货币兑换".into(),
                session_dir: root,
                control_count: 63,
                agent_context: Some(AgentContext {
                    provider: "codex".into(),
                    thread_id: "thread-1".into(),
                    session_id: "thread-1".into(),
                    workspace: "E:\\RedCliff".into(),
                }),
                target: WorkflowTarget {
                    project_workspace: "E:\\RedCliff".into(),
                    widget_blueprint: "/Game/UI/CurrencyExchange".into(),
                    ..WorkflowTarget::default()
                },
                stages,
                latest_message: "已投递到原 Codex 对话，等待 MCP 实现".into(),
                updated_at_unix_ms: 10,
            }],
        };
        save_store_to_path(&store, &path).unwrap();

        let loaded = load_store_from_path(&path).unwrap();

        assert_eq!(
            loaded.tasks[0].stages.umg.status,
            WorkflowStatus::AwaitingConfirmation
        );
        assert_eq!(
            loaded.tasks[0].latest_message,
            "旧版交付未确认，等待重新投递"
        );
    }

    #[test]
    fn changing_workbench_marks_editor_stages_stale() {
        let mut stages = WorkflowStages::default();
        stages.workbench.status = WorkflowStatus::Completed;
        stages.umg.status = WorkflowStatus::Completed;
        stages.logic.status = WorkflowStatus::Completed;
        stages.review.status = WorkflowStatus::Completed;

        mark_downstream_stale(&mut stages, WorkflowStage::Workbench);

        assert_eq!(stages.umg.status, WorkflowStatus::Stale);
        assert_eq!(stages.logic.status, WorkflowStatus::Stale);
        assert_eq!(stages.review.status, WorkflowStatus::Stale);
    }

    #[test]
    fn prepares_an_immutable_delivery_with_the_current_tree_and_target() {
        let root = temp_dir("delivery");
        let session = root.join("session");
        fs::create_dir_all(&session).unwrap();
        let load_path = "/RedCliff/Asset/UI/CurrencyExchange.CurrencyExchange";
        let mut task = UiWorkflowTask {
            task_id: "currency-exchange".into(),
            page_id: "currency-exchange".into(),
            title: "货币兑换".into(),
            session_dir: session.clone(),
            control_count: 2,
            agent_context: Some(AgentContext {
                provider: "codex".into(),
                thread_id: "thread-1".into(),
                session_id: "thread-1".into(),
                workspace: "E:\\RedCliff".into(),
            }),
            target: WorkflowTarget {
                project_workspace: "E:\\RedCliff".into(),
                widget_blueprint: load_path.into(),
                widget_blueprint_name: "CurrencyExchange".into(),
                widget_blueprint_class: "UGCWidgetBlueprint".into(),
                preflight: Some(ready_preflight(load_path)),
            },
            stages: WorkflowStages::default(),
            latest_message: String::new(),
            updated_at_unix_ms: 0,
        };
        task.stages.workbench.status = WorkflowStatus::AwaitingConfirmation;
        task.stages.umg.status = WorkflowStatus::Completed;
        task.stages.logic.status = WorkflowStatus::Completed;
        task.stages.review.status = WorkflowStatus::Completed;
        let tree = json!({
            "page_size": {"width": 1280, "height": 720},
            "nodes": [
                {"id": "text.title", "category": "text", "display_text": "货币兑换", "text_style": {"font_size": 32}},
                {"id": "button.exchange", "category": "button"}
            ]
        });

        let prepared = prepare_delivery(&mut task, &tree, 100).unwrap();

        assert!(prepared.request_path.is_file());
        assert!(prepared.tree_path.is_file());
        assert_eq!(task.stages.workbench.status, WorkflowStatus::Completed);
        assert_eq!(task.stages.umg.status, WorkflowStatus::Stale);
        assert_eq!(task.stages.logic.status, WorkflowStatus::Stale);
        assert_eq!(task.stages.review.status, WorkflowStatus::Stale);
        let request: Value =
            serde_json::from_str(&fs::read_to_string(prepared.request_path).unwrap()).unwrap();
        assert_eq!(request["agent_context"]["thread_id"], "thread-1");
        assert_eq!(request["target"]["widget_blueprint"], load_path);
        assert_eq!(request["schema_version"], 2);
        assert_eq!(request["preflight"]["evidence_id"], "sha256:test-evidence");
        assert_eq!(request["control_count"], 2);
    }

    #[test]
    fn rejects_delivery_without_preflight_before_creating_files() {
        let root = temp_dir("delivery-no-preflight");
        let session = root.join("session");
        fs::create_dir_all(&session).unwrap();
        let mut task = UiWorkflowTask {
            task_id: "currency-exchange".into(),
            page_id: "currency-exchange".into(),
            title: "货币兑换".into(),
            session_dir: session.clone(),
            control_count: 1,
            agent_context: Some(AgentContext {
                provider: "codex".into(),
                thread_id: "thread-1".into(),
                session_id: "thread-1".into(),
                workspace: r"E:\RedCliff".into(),
            }),
            target: WorkflowTarget {
                project_workspace: r"E:\RedCliff".into(),
                widget_blueprint: "/RedCliff/Asset/UI/CurrencyExchange.CurrencyExchange".into(),
                ..WorkflowTarget::default()
            },
            stages: WorkflowStages::default(),
            latest_message: String::new(),
            updated_at_unix_ms: 0,
        };

        let error = prepare_delivery(
            &mut task,
            &json!({"nodes": [{"id": "button.exchange"}]}),
            100,
        )
        .unwrap_err();

        assert_eq!(
            error,
            "target WidgetBlueprint has no valid editor preflight"
        );
        assert!(!session.join("deliveries").exists());
    }

    #[test]
    fn applies_a_scoped_agent_progress_update() {
        let root = temp_dir("progress");
        let session = root.join("session");
        fs::create_dir_all(session.join("workflow-updates")).unwrap();
        fs::write(session.join("artifact.json"), b"{}").unwrap();
        let mut store = UiWorkflowStore::default();
        store.tasks.push(UiWorkflowTask {
            task_id: "currency-exchange".into(),
            page_id: "currency-exchange".into(),
            title: "货币兑换".into(),
            session_dir: session.clone(),
            control_count: 2,
            agent_context: None,
            target: WorkflowTarget::default(),
            stages: WorkflowStages::default(),
            latest_message: String::new(),
            updated_at_unix_ms: 0,
        });
        store.tasks[0].stages.workbench.status = WorkflowStatus::Completed;
        let update_path = session.join("workflow-updates").join("umg.json");
        fs::write(
            &update_path,
            serde_json::to_vec_pretty(&json!({
                "schema_version": 1,
                "task_id": "currency-exchange",
                "stage": "umg",
                "status": "completed",
                "message": "WidgetBlueprint 已保存并回读",
                "artifacts": ["artifact.json"]
            }))
            .unwrap(),
        )
        .unwrap();

        apply_update_file(&mut store, &update_path, 200).unwrap();

        assert_eq!(store.tasks[0].stages.umg.status, WorkflowStatus::Completed);
        assert_eq!(
            store.tasks[0].latest_message,
            "WidgetBlueprint 已保存并回读"
        );
        assert!(apply_update_file(&mut store, &root.join("outside.json"), 300).is_err());
    }
}
