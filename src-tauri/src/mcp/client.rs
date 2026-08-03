//! MCP SSE client: full JSON-RPC over SSE transport.
//!
//! A session holds one persistent SSE GET connection (responses arrive here)
//! plus short-lived POST connections for requests. Responses are routed to
//! callers by JSON-RPC id via oneshot channels.
//!
//! Session lifecycle:
//!   GET /sse -> read `endpoint` event -> POST initialize -> read response
//!   -> POST notifications/initialized -> (hand stream to background reader)
//! Subsequent calls: POST to endpoint, response arrives on the SSE stream,
//! routed by id. Drop the session to abort the reader and close the stream.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::async_runtime;

use crate::config::McpToolInfo;

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const CLIENT_NAME: &str = "oasis-companion";
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const CALL_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug)]
pub enum McpError {
    Network(String),
    NoEndpoint,
    BadContentType(String),
    Timeout,
    JsonRpc(String),
    Closed,
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(s) => write!(f, "网络错误: {s}"),
            Self::NoEndpoint => write!(f, "SSE 握手失败: 未收到 endpoint 事件"),
            Self::BadContentType(s) => {
                write!(f, "非 SSE 端点 (期望 text/event-stream, 实际 {s})")
            }
            Self::Timeout => write!(f, "请求超时"),
            Self::JsonRpc(s) => write!(f, "JSON-RPC 错误: {s}"),
            Self::Closed => write!(f, "会话已关闭"),
        }
    }
}

impl std::error::Error for McpError {}

type Result<T> = std::result::Result<T, McpError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub content: Value,
    pub is_error: bool,
}

/// A live MCP session backed by a persistent SSE stream.
pub struct McpSession {
    client: reqwest::Client,
    base_url: String,
    endpoint: String,
    server_info: Option<ServerInfo>,
    next_id: AtomicU64,
    pending: Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<Value>>>>,
    _reader: async_runtime::JoinHandle<()>,
}

impl McpSession {
    /// Open SSE, complete the initialize handshake, then hand the stream to a
    /// background reader. Subsequent calls multiplex over POST + the SSE stream.
    pub async fn connect(host: &str, port: u16, sse_path: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(format!("{CLIENT_NAME}/{CLIENT_VERSION}"))
            .build()
            .map_err(|e| McpError::Network(e.to_string()))?;

        let path = normalize_path(sse_path);
        let base_url = format!("http://{}:{}", host.trim(), port);
        let sse_url = format!("{base_url}{path}");

        let resp = client
            .get(&sse_url)
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .send()
            .await
            .map_err(|e| McpError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(McpError::Network(format!("HTTP {}", resp.status())));
        }
        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        if !ct.contains("text/event-stream") {
            return Err(McpError::BadContentType(ct));
        }

        let mut response = resp;
        let mut buf = String::new();

        // endpoint event
        let endpoint = read_until_endpoint(&mut response, &mut buf, HANDSHAKE_TIMEOUT).await?;

        // initialize
        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": CLIENT_NAME, "version": CLIENT_VERSION }
            }
        });
        post_message(&client, &base_url, &endpoint, init_req).await?;
        let init_resp = read_until_response(&mut response, &mut buf, 1, HANDSHAKE_TIMEOUT).await?;
        if let Some(err) = init_resp.get("error") {
            return Err(McpError::JsonRpc(err.to_string()));
        }
        let server_info = init_resp
            .get("result")
            .and_then(|r| r.get("serverInfo"))
            .and_then(|si| {
                Some(ServerInfo {
                    name: si.get("name")?.as_str()?.to_string(),
                    version: si.get("version")?.as_str()?.to_string(),
                })
            });

        // notifications/initialized
        let notif = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        post_message(&client, &base_url, &endpoint, notif).await?;

        // hand the stream to a background reader
        let pending: Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending.clone();
        let reader = async_runtime::spawn(async move {
            reader_loop(response, buf, pending_clone).await;
        });

        Ok(McpSession {
            client,
            base_url,
            endpoint,
            server_info,
            next_id: AtomicU64::new(2),
            pending,
            _reader: reader,
        })
    }

    pub fn server_info(&self) -> Option<&ServerInfo> {
        self.server_info.as_ref()
    }

    pub async fn list_tools(&self) -> Result<Vec<McpToolInfo>> {
        let resp = self.call("tools/list", json!({})).await?;
        let mut out = Vec::new();
        if let Some(arr) = resp.get("tools").and_then(|v| v.as_array()) {
            for t in arr {
                let name = t
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let desc = t
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !name.is_empty() {
                    out.push(McpToolInfo {
                        name,
                        description: desc,
                    });
                }
            }
        }
        Ok(out)
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<ToolCallResult> {
        let resp = self
            .call(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            )
            .await?;
        let content = resp.get("content").cloned().unwrap_or(Value::Null);
        let is_error = resp
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Ok(ToolCallResult { content, is_error })
    }

    /// Send a JSON-RPC request and await its response (routed via the SSE reader).
    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let (tx, rx) = tokio::sync::oneshot::channel::<Value>();
        self.pending.lock().unwrap().insert(id, tx);

        if let Err(e) = post_message(&self.client, &self.base_url, &self.endpoint, req).await {
            self.pending.lock().unwrap().remove(&id);
            return Err(e);
        }

        match tokio::time::timeout(CALL_TIMEOUT, rx).await {
            Ok(Ok(val)) => {
                if let Some(err) = val.get("error") {
                    Err(McpError::JsonRpc(err.to_string()))
                } else {
                    Ok(val.get("result").cloned().unwrap_or(Value::Null))
                }
            }
            Ok(Err(_)) => Err(McpError::Closed),
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                Err(McpError::Timeout)
            }
        }
    }
}

/// Aborting the reader closes the SSE stream, ending the session.
impl Drop for McpSession {
    fn drop(&mut self) {
        self._reader.abort();
    }
}

// --- SSE stream helpers ---

async fn read_until_endpoint(
    response: &mut reqwest::Response,
    buf: &mut String,
    timeout: Duration,
) -> Result<String> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Some((event_type, data)) = take_event(buf) {
            if event_type.as_deref() == Some("endpoint") && !data.is_empty() {
                return Ok(data);
            }
            continue;
        }
        if !read_chunk_timed(response, buf, deadline).await? {
            return Err(McpError::NoEndpoint);
        }
    }
}

async fn read_until_response(
    response: &mut reqwest::Response,
    buf: &mut String,
    want_id: u64,
    timeout: Duration,
) -> Result<Value> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Some((_etype, data)) = take_event(buf) {
            if data.is_empty() {
                continue;
            }
            if let Ok(val) = serde_json::from_str::<Value>(&data) {
                if val.get("id").and_then(|v| v.as_u64()) == Some(want_id) {
                    return Ok(val);
                }
            }
            continue;
        }
        if !read_chunk_timed(response, buf, deadline).await? {
            return Err(McpError::Timeout);
        }
    }
}

/// Read one chunk into `buf` before `deadline`. Returns false on timeout/EOS.
async fn read_chunk_timed(
    response: &mut reqwest::Response,
    buf: &mut String,
    deadline: tokio::time::Instant,
) -> Result<bool> {
    if tokio::time::Instant::now() >= deadline {
        return Ok(false);
    }
    match tokio::time::timeout_at(deadline, response.chunk()).await {
        Ok(Ok(Some(chunk))) => {
            buf.push_str(&String::from_utf8_lossy(&chunk));
            Ok(true)
        }
        Ok(Ok(None)) => Ok(false),
        Ok(Err(e)) => Err(McpError::Network(e.to_string())),
        Err(_) => Ok(false),
    }
}

/// Background reader: parse SSE events, route JSON-RPC responses by id.
async fn reader_loop(
    mut response: reqwest::Response,
    mut buf: String,
    pending: Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<Value>>>>,
) {
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                buf.push_str(&String::from_utf8_lossy(&chunk));
                while let Some((_etype, data)) = take_event(&mut buf) {
                    if data.is_empty() {
                        continue;
                    }
                    if let Ok(val) = serde_json::from_str::<Value>(&data) {
                        if let Some(id) = val.get("id").and_then(|v| v.as_u64()) {
                            if let Some(tx) = pending.lock().unwrap().remove(&id) {
                                let _ = tx.send(val);
                            }
                        } else {
                            log::debug!(
                                "mcp notification: {}",
                                data.chars().take(200).collect::<String>()
                            );
                        }
                    }
                }
            }
            Ok(None) => {
                log::info!("mcp sse stream ended");
                break;
            }
            Err(e) => {
                log::warn!("mcp sse read error: {e}");
                break;
            }
        }
    }
    // stream closed: drop all pending senders -> receivers get Err (Closed)
    pending.lock().unwrap().clear();
}

async fn post_message(
    client: &reqwest::Client,
    base_url: &str,
    endpoint: &str,
    body: Value,
) -> Result<()> {
    let url = if endpoint.starts_with("http") {
        endpoint.to_string()
    } else {
        format!("{base_url}{endpoint}")
    };
    let response = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| McpError::Network(e.to_string()))?;
    if !response.status().is_success() {
        return Err(McpError::Network(format!(
            "POST {} failed: HTTP {}",
            url,
            response.status()
        )));
    }
    Ok(())
}

fn normalize_path(p: &str) -> String {
    if p.starts_with('/') {
        p.to_string()
    } else {
        format!("/{p}")
    }
}

/// Extract one complete SSE event from the front of `buf`.
/// Returns `(event_type, data)` and removes the event from the buffer.
fn take_event(buf: &mut String) -> Option<(Option<String>, String)> {
    let boundary = event_boundary(buf)?;
    let event_str: String = buf.drain(..boundary).collect();
    let mut event_type = None;
    let mut data_lines: Vec<String> = Vec::new();
    for line in event_str.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix("event:") {
            event_type = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("data:") {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            data_lines.push(rest.to_string());
        }
        // lines starting with ':' are comments, ignored
    }
    Some((event_type, data_lines.join("\n")))
}

/// Index *after* the blank-line separator, or None if no complete event yet.
fn event_boundary(buf: &str) -> Option<usize> {
    if let Some(idx) = buf.find("\r\n\r\n") {
        return Some(idx + 4);
    }
    if let Some(idx) = buf.find("\n\n") {
        return Some(idx + 2);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_path() {
        assert_eq!(normalize_path("/sse"), "/sse");
        assert_eq!(normalize_path("sse"), "/sse");
    }

    #[test]
    fn takes_endpoint_event() {
        let mut buf =
            String::from(": keepalive\n\nevent: endpoint\ndata: /messages?session=abc\n\n");
        // first event is a comment -> no event_type, empty data (skipped by read_until_endpoint)
        let (etype1, data1) = take_event(&mut buf).unwrap();
        assert_eq!(etype1.as_deref(), None);
        assert_eq!(data1, "");
        // second event is the endpoint
        let (etype2, data2) = take_event(&mut buf).unwrap();
        assert_eq!(etype2.as_deref(), Some("endpoint"));
        assert_eq!(data2, "/messages?session=abc");
        assert!(take_event(&mut buf).is_none());
    }

    #[test]
    fn takes_json_response_event() {
        let mut buf = String::from(
            "event: message\ndata: {\"id\":1,\"jsonrpc\":\"2.0\",\"result\":{\"tools\":[]}}\n\n",
        );
        let (_etype, data) = take_event(&mut buf).unwrap();
        let v: Value = serde_json::from_str(&data).unwrap();
        assert_eq!(v["id"], 1);
    }

    #[test]
    fn joins_multiline_data() {
        let mut buf = String::from("data: line1\ndata: line2\n\n");
        let (_etype, data) = take_event(&mut buf).unwrap();
        assert_eq!(data, "line1\nline2");
    }

    #[test]
    fn handles_crlf() {
        let mut buf = String::from("event: endpoint\r\ndata: /m?s=1\r\n\r\n");
        let (etype, data) = take_event(&mut buf).unwrap();
        assert_eq!(etype.as_deref(), Some("endpoint"));
        assert_eq!(data, "/m?s=1");
    }
}
