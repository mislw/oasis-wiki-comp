//! MCP module: client + discovery + status bridging to settings/frontend.

pub mod client;
pub mod discover;

pub use client::{McpSession, ServerInfo, ToolCallResult};
pub use discover::DiscoveredEndpoint;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{self, McpConfig, McpToolInfo};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpConnectionState {
    /// Detection disabled in settings.
    Disabled,
    /// Enabled but never probed this run.
    Unchecked,
    /// Probed and the editor is not reachable.
    Disconnected,
    /// Connected: SSE open + initialized + tools listed.
    Connected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStatus {
    pub enabled: bool,
    pub state: McpConnectionState,
    pub url: String,
    pub server_info: Option<ServerInfo>,
    pub tools: Vec<McpToolInfo>,
    pub checked_at: String,
    pub error: Option<String>,
}

/// Full probe: resolve endpoint (auto-discover or config) -> connect -> tools/list.
/// Persists results to settings and returns the status.
pub async fn check(app: &AppHandle) -> McpStatus {
    let (enabled, mcp_cfg) = {
        let st = app.state::<AppState>();
        let s = st.settings.lock().unwrap();
        (s.skill_runtime.mcp.enabled, s.skill_runtime.mcp.clone())
    };

    let checked_at = now_epoch_seconds();

    if !enabled {
        return McpStatus {
            enabled: false,
            state: McpConnectionState::Disabled,
            url: format_sse_url(&mcp_cfg.host, mcp_cfg.port, &mcp_cfg.sse_path),
            server_info: None,
            tools: Vec::new(),
            checked_at,
            error: None,
        };
    }

    // resolve endpoint (auto-discover may scan ports)
    let (host, port, sse_path) = resolve_endpoint(app, &mcp_cfg).await;
    let url = format_sse_url(&host, port, &sse_path);

    let status = match McpSession::connect(&host, port, &sse_path).await {
        Ok(session) => {
            let server_info = session.server_info().cloned();
            match session.list_tools().await {
                Ok(tools) if has_required_tools(&tools) => {
                    // session drops here -> reader aborts -> SSE closes (per-request session)
                    McpStatus {
                        enabled: true,
                        state: McpConnectionState::Connected,
                        url,
                        server_info,
                        tools,
                        checked_at,
                        error: None,
                    }
                }
                Ok(tools) => McpStatus {
                    enabled: true,
                    state: McpConnectionState::Disconnected,
                    url,
                    server_info,
                    tools,
                    checked_at,
                    error: Some("MCP connected, but required UGCAskQ tools are missing".into()),
                },
                Err(e) => McpStatus {
                    enabled: true,
                    state: McpConnectionState::Disconnected,
                    url,
                    server_info,
                    tools: Vec::new(),
                    checked_at,
                    error: Some(format!("tools/list failed: {e}")),
                },
            }
        }
        Err(e) => McpStatus {
            enabled: true,
            state: McpConnectionState::Disconnected,
            url,
            server_info: None,
            tools: Vec::new(),
            checked_at,
            error: Some(e.to_string()),
        },
    };

    // persist to settings (host/port may have been auto-discovered)
    {
        let st = app.state::<AppState>();
        let mut s = st.settings.lock().unwrap();
        let m = &mut s.skill_runtime.mcp;
        m.host = host.clone();
        m.port = port;
        m.sse_path = sse_path.clone();
        m.last_checked_at = Some(status.checked_at.clone());
        m.last_status = Some(state_label(&status.state).to_string());
        m.last_error = status.error.clone();
        m.last_server_name = status.server_info.as_ref().map(|si| si.name.clone());
        m.last_server_version = status.server_info.as_ref().map(|si| si.version.clone());
        m.cached_tools = status.tools.clone();
        if let Err(e) = config::save(&s) {
            log::warn!("failed to save MCP status: {e}");
        }
    }

    let _ = app.emit("mcp-status-changed", &status);
    status
}

fn has_required_tools(tools: &[McpToolInfo]) -> bool {
    ["ue_read", "ue_py", "ue_plan_submit"]
        .iter()
        .all(|required| tools.iter().any(|tool| tool.name == *required))
}

/// One-click connect: force-enable + auto-discover + full handshake.
///
/// This is the "just connect me" entry point — no manual toggles needed.
/// Forces `enabled=true` and `auto_discover=true`, discovers the endpoint
/// (`.mcp.json` or port scan), then completes the full SSE handshake.
/// Returns `Connected` + tool list on success, `Disconnected` with a clear
/// message if the editor isn't running.
pub async fn connect_auto(app: &AppHandle) -> McpStatus {
    // 1. Force-enable MCP detection + auto-discover, persist.
    {
        let st = app.state::<AppState>();
        let mut s = st.settings.lock().unwrap();
        let m = &mut s.skill_runtime.mcp;
        m.enabled = true;
        m.auto_discover = true;
        if let Err(e) = config::save(&s) {
            log::warn!("failed to persist MCP auto-connect config: {e}");
        }
    }

    // 2. Discover the endpoint up front (for a better error message if the
    //    editor isn't running). check() will rediscover anyway, but this lets
    //    us short-circuit with a friendly message instead of "connection refused".
    match discover(app).await {
        Ok(Some(ep)) => {
            log::info!(
                "auto-connect: discovered {}:{}{} via {}",
                ep.host,
                ep.port,
                ep.sse_path,
                ep.source
            );
            // 3. Full handshake: resolve (rediscover) -> connect -> tools/list.
            //    check() persists host/port + last_* and emits the event.
            check(app).await
        }
        Ok(None) => {
            // No editor MCP endpoint found.
            let checked_at = now_epoch_seconds();
            let url = {
                let st = app.state::<AppState>();
                let s = st.settings.lock().unwrap();
                format_sse_url(
                    &s.skill_runtime.mcp.host,
                    s.skill_runtime.mcp.port,
                    &s.skill_runtime.mcp.sse_path,
                )
            };
            let status = McpStatus {
                enabled: true,
                state: McpConnectionState::Disconnected,
                url,
                server_info: None,
                tools: Vec::new(),
                checked_at: checked_at.clone(),
                error: Some(
                    "未找到编辑器 MCP 端点，请确认绿洲编辑器已启动并开启了 MCP 服务".into(),
                ),
            };
            {
                let st = app.state::<AppState>();
                let mut s = st.settings.lock().unwrap();
                let m = &mut s.skill_runtime.mcp;
                m.last_checked_at = Some(checked_at);
                m.last_status = Some("disconnected".into());
                m.last_error = status.error.clone();
                if let Err(e) = config::save(&s) {
                    log::warn!("failed to persist MCP failure: {e}");
                }
            }
            let _ = app.emit("mcp-status-changed", &status);
            status
        }
        Err(e) => {
            log::warn!("auto-connect discovery error: {e}");
            // Fall through to check() — it may still connect via cached config.
            check(app).await
        }
    }
}

/// Disable editor MCP usage and clear stale connection metadata.
pub fn disable(app: &AppHandle) -> McpStatus {
    let checked_at = now_epoch_seconds();
    let status = {
        let st = app.state::<AppState>();
        let mut s = st.settings.lock().unwrap();
        let m = &mut s.skill_runtime.mcp;
        m.enabled = false;
        m.last_checked_at = Some(checked_at.clone());
        m.last_status = Some("disabled".into());
        m.last_error = None;
        m.last_server_name = None;
        m.last_server_version = None;
        m.cached_tools.clear();
        let url = format_sse_url(&m.host, m.port, &m.sse_path);
        if let Err(e) = config::save(&s) {
            log::warn!("failed to persist MCP disabled state: {e}");
        }
        McpStatus {
            enabled: false,
            state: McpConnectionState::Disabled,
            url,
            server_info: None,
            tools: Vec::new(),
            checked_at,
            error: None,
        }
    };
    let _ = app.emit("mcp-status-changed", &status);
    status
}

/// Open a fresh session, call a tool, close. One-shot model.
/// Uses the persisted host/port (run `check` first to auto-discover).
pub async fn call_tool(
    app: &AppHandle,
    name: &str,
    arguments: Value,
) -> std::result::Result<ToolCallResult, String> {
    let mcp_cfg = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .skill_runtime
        .mcp
        .clone();
    if !mcp_cfg.enabled {
        return Err("MCP 检测未启用".into());
    }
    let session = McpSession::connect(&mcp_cfg.host, mcp_cfg.port, &mcp_cfg.sse_path)
        .await
        .map_err(|e| e.to_string())?;
    session
        .call_tool(name, arguments)
        .await
        .map_err(|e| e.to_string())
}

/// Try .mcp.json (if project_path set) then port-scan. Does NOT persist.
pub async fn discover(app: &AppHandle) -> std::result::Result<Option<DiscoveredEndpoint>, String> {
    let project_path = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .skill_runtime
        .mcp
        .project_path
        .clone();
    if let Some(pp) = &project_path {
        if let Some(ep) = discover::read_mcp_json(pp) {
            return Ok(Some(ep));
        }
    }
    Ok(discover::discover_editor_endpoint().await)
}

/// Resolve the endpoint to use for a `check`: auto-discover if enabled,
/// otherwise fall back to the configured host/port.
async fn resolve_endpoint(app: &AppHandle, mcp_cfg: &McpConfig) -> (String, u16, String) {
    if mcp_cfg.auto_discover {
        if let Ok(Some(ep)) = discover(app).await {
            return (ep.host, ep.port, ep.sse_path);
        }
    }
    (mcp_cfg.host.clone(), mcp_cfg.port, mcp_cfg.sse_path.clone())
}

fn state_label(s: &McpConnectionState) -> &'static str {
    match s {
        McpConnectionState::Disabled => "disabled",
        McpConnectionState::Unchecked => "unchecked",
        McpConnectionState::Disconnected => "disconnected",
        McpConnectionState::Connected => "connected",
    }
}

fn format_sse_url(host: &str, port: u16, sse_path: &str) -> String {
    let path = if sse_path.starts_with('/') {
        sse_path.to_string()
    } else {
        format!("/{sse_path}")
    };
    format!("http://{}:{}{}", host.trim(), port, path)
}

fn now_epoch_seconds() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_url() {
        assert_eq!(
            format_sse_url("127.0.0.1", 12463, "/sse"),
            "http://127.0.0.1:12463/sse"
        );
    }

    #[test]
    fn requires_core_ugcaskq_tools() {
        let tools = vec![
            McpToolInfo {
                name: "ue_read".into(),
                description: String::new(),
            },
            McpToolInfo {
                name: "ue_py".into(),
                description: String::new(),
            },
            McpToolInfo {
                name: "ue_plan_submit".into(),
                description: String::new(),
            },
        ];
        assert!(has_required_tools(&tools));
        assert!(!has_required_tools(&tools[..2]));
    }
}
