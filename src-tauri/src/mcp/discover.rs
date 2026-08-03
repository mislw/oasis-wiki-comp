//! Auto-discovery of the Oasis editor's MCP SSE endpoint.
//!
//! Two strategies, tried in order:
//! 1. Read `<project_path>/.mcp.json` (authoritative — the editor writes it).
//! 2. Scan the editor process's listening ports and probe each for an SSE
//!    endpoint, validating `Content-Type: text/event-stream`.

use std::time::Duration;

use serde_json::Value;
use sysinfo::System;

/// A resolved SSE endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscoveredEndpoint {
    pub host: String,
    pub port: u16,
    pub sse_path: String,
    /// Where it was found: `"mcp.json"` or `"port-scan"`.
    pub source: String,
}

const EDITOR_PROCESS_NAMES: &[&str] = &["ShadowTrackerExtraUGCEditor", "UGCEditor"];

/// Scan the editor process's listening ports; probe each for an SSE endpoint.
/// Returns the first port that serves a real SSE stream.
pub async fn discover_editor_endpoint() -> Option<DiscoveredEndpoint> {
    let ports = editor_listening_ports();
    log::info!("discovering MCP among {} ports: {:?}", ports.len(), ports);
    for port in ports {
        if let Some(ep) = probe_sse_port(port).await {
            log::info!("discovered MCP SSE on port {port} (via port-scan)");
            return Some(ep);
        }
    }
    None
}

/// Find TCP LISTENING ports owned by the editor process.
fn editor_listening_ports() -> Vec<u16> {
    let editor_pids = editor_pids();
    if editor_pids.is_empty() {
        return Vec::new();
    }
    let out = std::process::Command::new("netstat").arg("-ano").output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            log::warn!("netstat failed: {e}");
            return Vec::new();
        }
    };
    // netstat output is ASCII for addresses/ports/PIDs even on localized Windows;
    // lossy decode is sufficient to parse the columns we need.
    let text = String::from_utf8_lossy(&out.stdout);
    let mut ports = Vec::new();
    for line in text.lines() {
        if !line.contains("LISTENING") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        // TCP  0.0.0.0:12463  0.0.0.0:0  LISTENING  34076
        let local = parts[1];
        let pid_str = parts[parts.len() - 1];
        if !editor_pids.iter().any(|p: &String| p == pid_str) {
            continue;
        }
        if let Some(port_str) = local.rsplit(':').next() {
            if let Ok(p) = port_str.parse::<u16>() {
                if !ports.contains(&p) {
                    ports.push(p);
                }
            }
        }
    }
    ports
}

/// PIDs of running editor processes, as strings (for netstat matching).
fn editor_pids() -> Vec<String> {
    let sys = System::new_all();
    sys.processes()
        .iter()
        .filter(|(_, proc)| {
            let name = proc.name().to_string().to_ascii_lowercase();
            EDITOR_PROCESS_NAMES
                .iter()
                .any(|e| name.contains(&e.to_ascii_lowercase()))
        })
        .map(|(pid, _)| pid.as_u32().to_string())
        .collect()
}

/// Probe one port: GET /sse, validate Content-Type.
async fn probe_sse_port(port: u16) -> Option<DiscoveredEndpoint> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?;
    let url = format!("http://127.0.0.1:{port}/sse");
    let resp = client
        .get(&url)
        .header(reqwest::header::ACCEPT, "text/event-stream")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if ct.contains("text/event-stream") {
        Some(DiscoveredEndpoint {
            host: "127.0.0.1".into(),
            port,
            sse_path: "/sse".into(),
            source: "port-scan".into(),
        })
    } else {
        None
    }
}

/// Read `<project_path>/.mcp.json` and return the first SSE server URL.
pub fn read_mcp_json(project_path: &str) -> Option<DiscoveredEndpoint> {
    let p = std::path::Path::new(project_path).join(".mcp.json");
    let content = std::fs::read_to_string(&p).ok()?;
    let v: Value = serde_json::from_str(&content).ok()?;
    let servers = v.get("mcpServers")?.as_object()?;
    for (_name, cfg) in servers {
        let t = cfg.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if t == "sse" {
            if let Some(url) = cfg.get("url").and_then(|u| u.as_str()) {
                if let Some(mut ep) = parse_sse_url(url) {
                    ep.source = "mcp.json".into();
                    return Some(ep);
                }
            }
        }
    }
    None
}

fn parse_sse_url(url: &str) -> Option<DiscoveredEndpoint> {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    let (authority, path) = rest.split_once('/')?;
    let (host, port_str) = authority.rsplit_once(':')?;
    let port: u16 = port_str.parse().ok()?;
    Some(DiscoveredEndpoint {
        host: host.to_string(),
        port,
        sse_path: format!("/{path}"),
        source: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sse_url() {
        let ep = parse_sse_url("http://127.0.0.1:12463/sse").unwrap();
        assert_eq!(ep.host, "127.0.0.1");
        assert_eq!(ep.port, 12463);
        assert_eq!(ep.sse_path, "/sse");
    }

    #[test]
    fn rejects_non_http() {
        assert!(parse_sse_url("ftp://x/y").is_none());
    }

    #[test]
    fn parses_mcp_json_content() {
        let json =
            r#"{"mcpServers":{"ugcaskq":{"type":"sse","url":"http://127.0.0.1:12463/sse"}}}"#;
        let v: Value = serde_json::from_str(json).unwrap();
        let servers = v.get("mcpServers").unwrap().as_object().unwrap();
        let cfg = servers.get("ugcaskq").unwrap();
        assert_eq!(cfg.get("type").unwrap().as_str(), Some("sse"));
    }
}
