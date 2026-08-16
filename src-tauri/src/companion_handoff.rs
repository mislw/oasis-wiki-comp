//! File-backed handoffs from the bundled Skill to the running Companion.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::AppHandle;

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct HandoffEnvelope {
    schema_version: u32,
    #[serde(flatten)]
    action: HandoffAction,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum HandoffAction {
    OpenUiWorkflow,
    UiWorkbench {
        url: String,
        session_dir: Option<PathBuf>,
    },
    WorkflowUpdate {
        update_path: PathBuf,
    },
}

#[derive(Serialize)]
struct RuntimeRecord {
    schema_version: u32,
    pid: u32,
    executable: PathBuf,
}

pub fn initialize(app: &AppHandle) -> Result<bool, String> {
    write_runtime_record()?;
    let handled = drain_pending(app);
    spawn_loop(app.clone());
    Ok(handled)
}

fn state_dir() -> PathBuf {
    crate::config::settings_dir()
}

fn inbox_dir() -> PathBuf {
    state_dir().join("handoffs")
}

fn runtime_path() -> PathBuf {
    state_dir().join("runtime.json")
}

fn write_runtime_record() -> Result<(), String> {
    let path = runtime_path();
    let directory = path
        .parent()
        .ok_or_else(|| "Companion runtime path has no parent".to_string())?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("could not create Companion state directory: {error}"))?;
    let record = RuntimeRecord {
        schema_version: 1,
        pid: std::process::id(),
        executable: std::env::current_exe()
            .map_err(|error| format!("could not resolve Companion executable: {error}"))?,
    };
    let temporary = directory.join(".runtime.json.tmp");
    let body = serde_json::to_vec_pretty(&record)
        .map_err(|error| format!("could not serialize Companion runtime record: {error}"))?;
    fs::write(&temporary, body)
        .map_err(|error| format!("could not write Companion runtime record: {error}"))?;
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|error| format!("could not replace Companion runtime record: {error}"))?;
    }
    fs::rename(&temporary, &path)
        .map_err(|error| format!("could not publish Companion runtime record: {error}"))
}

fn parse_handoff(path: &Path) -> Result<HandoffAction, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("could not read Companion handoff: {error}"))?;
    let envelope: HandoffEnvelope = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid Companion handoff: {error}"))?;
    if envelope.schema_version != 1 {
        return Err("unsupported Companion handoff schema".into());
    }
    match &envelope.action {
        HandoffAction::UiWorkbench { url, session_dir } => {
            crate::ui_workbench::validate_ui_workbench_url(url)?;
            if session_dir.as_ref().is_some_and(|path| !path.is_absolute()) {
                return Err("UI Workbench session directory must be absolute".into());
            }
        }
        HandoffAction::WorkflowUpdate { update_path } if !update_path.is_absolute() => {
            return Err("UI workflow update path must be absolute".into());
        }
        _ => {}
    }
    Ok(envelope.action)
}

fn apply_handoff(app: &AppHandle, action: HandoffAction) -> Result<(), String> {
    match action {
        HandoffAction::OpenUiWorkflow => crate::ui_workflow::show_ui_workflow(app).map(|_| ()),
        HandoffAction::UiWorkbench { url, session_dir } => {
            crate::ui_workbench::apply_ui_workbench_handoff(
                app,
                crate::ui_workbench::WorkbenchHandoff {
                    url: crate::ui_workbench::validate_ui_workbench_url(&url)?,
                    session_dir,
                },
            )
        }
        HandoffAction::WorkflowUpdate { update_path } => {
            crate::ui_workflow::apply_update_handoff(app, &update_path)
        }
    }
}

fn pending_paths() -> Result<Vec<PathBuf>, String> {
    let directory = inbox_dir();
    fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create Companion handoff inbox: {error}"))?;
    let mut paths = fs::read_dir(&directory)
        .map_err(|error| format!("could not read Companion handoff inbox: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn failed_path(path: &Path) -> PathBuf {
    path.with_extension("failed")
}

fn drain_pending(app: &AppHandle) -> bool {
    let paths = match pending_paths() {
        Ok(paths) => paths,
        Err(error) => {
            log::error!("{error}");
            return false;
        }
    };
    let mut handled = false;
    for path in paths {
        match parse_handoff(&path).and_then(|action| apply_handoff(app, action)) {
            Ok(()) => {
                handled = true;
                if let Err(error) = fs::remove_file(&path) {
                    log::warn!("could not remove completed Companion handoff: {error}");
                }
            }
            Err(error) => {
                log::error!("Companion handoff failed for {}: {error}", path.display());
                if let Err(rename_error) = fs::rename(&path, failed_path(&path)) {
                    log::warn!("could not quarantine failed Companion handoff: {rename_error}");
                }
            }
        }
    }
    handled
}

fn spawn_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            drain_pending(&app);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(name: &str, body: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("oasis-handoff-{name}-{nonce}"));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("handoff.json");
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn parses_all_supported_handoff_actions() {
        let open = temp_file("open", r#"{"schema_version":1,"kind":"open_ui_workflow"}"#);
        assert_eq!(parse_handoff(&open).unwrap(), HandoffAction::OpenUiWorkflow);

        let workbench = temp_file(
            "workbench",
            r#"{"schema_version":1,"kind":"ui_workbench","url":"http://localhost:50691/","session_dir":"C:\\session"}"#,
        );
        assert_eq!(
            parse_handoff(&workbench).unwrap(),
            HandoffAction::UiWorkbench {
                url: "http://localhost:50691/".into(),
                session_dir: Some(PathBuf::from(r"C:\session")),
            }
        );

        let update = temp_file(
            "update",
            r#"{"schema_version":1,"kind":"workflow_update","update_path":"C:\\session\\update.json"}"#,
        );
        assert_eq!(
            parse_handoff(&update).unwrap(),
            HandoffAction::WorkflowUpdate {
                update_path: PathBuf::from(r"C:\session\update.json"),
            }
        );
    }

    #[test]
    fn rejects_unsafe_or_unsupported_handoffs() {
        for (name, body) in [
            (
                "schema",
                r#"{"schema_version":2,"kind":"open_ui_workflow"}"#,
            ),
            (
                "remote",
                r#"{"schema_version":1,"kind":"ui_workbench","url":"http://example.com:50691/","session_dir":null}"#,
            ),
            (
                "relative",
                r#"{"schema_version":1,"kind":"workflow_update","update_path":"update.json"}"#,
            ),
        ] {
            assert!(parse_handoff(&temp_file(name, body)).is_err(), "{name}");
        }
    }
}
