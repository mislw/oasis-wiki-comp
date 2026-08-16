//! External UI Workbench session handoff.

use reqwest::Url;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;
use crate::ui_workbench_catalog::{register_session_dir, save_catalog};
use crate::ui_workflow;

/// Validated arguments for opening a generated UI Workbench session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchHandoff {
    pub url: String,
    pub session_dir: Option<PathBuf>,
}

/// Validate and normalize a loopback HTTP workbench base URL.
///
/// The returned URL always has a trailing slash so relative session assets
/// resolve beneath the supplied directory.
pub fn validate_ui_workbench_url(value: &str) -> Result<String, String> {
    let mut url = Url::parse(value).map_err(|error| format!("invalid workbench URL: {error}"))?;
    if url.scheme() != "http" {
        return Err("workbench URL must use http".into());
    }
    if !matches!(url.host_str(), Some("localhost" | "127.0.0.1")) {
        return Err("workbench URL must use a loopback host".into());
    }
    if url.port().is_none() {
        return Err("workbench URL must include an explicit port".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("workbench URL must not include credentials".into());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("workbench URL must not include a query or fragment".into());
    }

    if !url.path().ends_with('/') {
        let normalized_path = format!("{}/", url.path());
        url.set_path(&normalized_path);
    }
    Ok(url.into())
}

/// Parse the optional `--ui-workbench-url <value>` process argument.
pub fn parse_ui_workbench_url(args: &[String]) -> Result<Option<String>, String> {
    let Some(index) = args.iter().position(|arg| arg == "--ui-workbench-url") else {
        return Ok(None);
    };
    let value = args
        .get(index + 1)
        .ok_or_else(|| "--ui-workbench-url requires a value".to_string())?;
    validate_ui_workbench_url(value).map(Some)
}

/// Select an external workbench handoff from process arguments.
pub fn handoff_action(args: &[String]) -> Result<Option<WorkbenchHandoff>, String> {
    let Some(url) = parse_ui_workbench_url(args)? else {
        return Ok(None);
    };
    let session_dir = match args
        .iter()
        .position(|arg| arg == "--ui-workbench-session-dir")
    {
        Some(index) => {
            let value = args
                .get(index + 1)
                .ok_or_else(|| "--ui-workbench-session-dir requires a value".to_string())?;
            Some(PathBuf::from(value))
        }
        None => None,
    };
    Ok(Some(WorkbenchHandoff { url, session_dir }))
}

/// Register an optional persistent page, retain its URL, and show the workbench.
pub fn apply_ui_workbench_handoff(
    app: &AppHandle,
    handoff: WorkbenchHandoff,
) -> Result<(), String> {
    if let Some(session_dir) = &handoff.session_dir {
        let state = app.state::<AppState>();
        let mut next = state.ui_workbench_catalog.lock().unwrap().clone();
        match register_session_dir(&mut next, session_dir, current_unix_ms()) {
            Ok(_) => {
                save_catalog(&next)?;
                *state.ui_workbench_catalog.lock().unwrap() = next;
            }
            Err(error) => {
                log::warn!("UI Workbench session was not added to the page catalog: {error}");
            }
        }
        let mut workflow = state.ui_workflow_store.lock().unwrap().clone();
        match ui_workflow::register_session(&mut workflow, session_dir, current_unix_ms()) {
            Ok(()) => {
                ui_workflow::save_store(&workflow)?;
                *state.ui_workflow_store.lock().unwrap() = workflow.clone();
                app.emit("ui-workflow://progress", workflow)
                    .map_err(|error| format!("could not emit UI workflow progress: {error}"))?;
            }
            Err(error) => log::warn!("UI workflow task was not registered: {error}"),
        }
    }
    *app.state::<AppState>()
        .pending_ui_workbench_url
        .lock()
        .unwrap() = Some(handoff.url.clone());

    let window = show_ui_workbench(app)?;
    window
        .emit("ui-workbench://session", handoff.url)
        .map_err(|error| format!("could not emit UI Workbench session: {error}"))
}

/// Show and focus the UI Workbench window.
pub fn show_ui_workbench(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    let window = match app.get_webview_window("ui-workbench") {
        Some(window) => window,
        None => {
            WebviewWindowBuilder::new(app, "ui-workbench", WebviewUrl::App("index.html".into()))
                .title("Oasis UI 工作台")
                .inner_size(1440.0, 880.0)
                .min_inner_size(1080.0, 700.0)
                .resizable(true)
                .decorations(true)
                .transparent(false)
                .shadow(true)
                .skip_taskbar(false)
                .build()
                .map_err(|error| format!("could not create UI Workbench window: {error}"))?
        }
    };

    window
        .show()
        .map_err(|error| format!("could not show UI Workbench window: {error}"))?;
    window
        .unminimize()
        .map_err(|error| format!("could not restore UI Workbench window: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("could not focus UI Workbench window: {error}"))?;
    Ok(window)
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_loopback_http_session_urls() {
        assert_eq!(
            validate_ui_workbench_url("http://localhost:50691").unwrap(),
            "http://localhost:50691/"
        );
        assert_eq!(
            validate_ui_workbench_url("http://127.0.0.1:50691/session/").unwrap(),
            "http://127.0.0.1:50691/session/"
        );
    }

    #[test]
    fn rejects_non_loopback_or_credential_bearing_urls() {
        for value in [
            "https://localhost:50691/",
            "http://example.com:50691/",
            "http://user:pass@localhost:50691/",
            "http://localhost/",
            "http://localhost:50691/?token=secret",
            "http://localhost:50691/#fragment",
        ] {
            assert!(validate_ui_workbench_url(value).is_err(), "{value}");
        }
    }

    #[test]
    fn parses_the_ui_workbench_cli_pair() {
        let session_dir = std::env::temp_dir().join("oasis-workbench-currency");
        let args = vec![
            "oasis-companion.exe".into(),
            "--ui-workbench-url".into(),
            "http://localhost:50691".into(),
            "--ui-workbench-session-dir".into(),
            session_dir.to_string_lossy().into_owned(),
        ];
        assert_eq!(
            handoff_action(&args).unwrap(),
            Some(WorkbenchHandoff {
                url: "http://localhost:50691/".into(),
                session_dir: Some(session_dir),
            })
        );
    }

    #[test]
    fn selects_workbench_handoff() {
        let valid = vec![
            "oasis-companion.exe".into(),
            "--ui-workbench-url".into(),
            "http://127.0.0.1:50691".into(),
        ];
        assert_eq!(
            handoff_action(&valid).unwrap(),
            Some(WorkbenchHandoff {
                url: "http://127.0.0.1:50691/".into(),
                session_dir: None,
            })
        );

        let unrelated = vec!["oasis-companion.exe".into(), "--background".into()];
        assert_eq!(handoff_action(&unrelated).unwrap(), None);

        let invalid = vec![
            "oasis-companion.exe".into(),
            "--ui-workbench-url".into(),
            "https://example.com:50691/".into(),
        ];
        assert!(handoff_action(&invalid).is_err());
    }
}
