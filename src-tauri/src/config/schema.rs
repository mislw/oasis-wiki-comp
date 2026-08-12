use serde::{Deserialize, Serialize};

/// Top-level runtime settings shared between Companion and the oasis-wiki Skill.
///
/// Lives at `%USERPROFILE%\.oasis-companion\settings.json` — Agent-neutral,
/// not tied to any specific Agent's directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Bumped on every schema change. `config::load` migrates or, on failure,
    /// backs up the old file and writes defaults.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,

    #[serde(default)]
    pub agent_detection: AgentDetection,

    #[serde(default)]
    pub ball: BallConfig,

    #[serde(default)]
    pub companion: CompanionConfig,

    #[serde(default)]
    pub skill: SkillConfig,

    #[serde(default)]
    pub skill_runtime: SkillRuntime,

    #[serde(default)]
    pub updates: UpdatesConfig,
}

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDetection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_process_names")]
    pub process_names: Vec<String>,
    #[serde(default = "default_window_titles")]
    pub window_titles: Vec<String>,
    #[serde(default)]
    pub install_paths: Vec<String>,
}

fn default_true() -> bool {
    true
}
fn default_interval() -> u64 {
    3
}
fn default_process_names() -> Vec<String> {
    vec![
        "Codex.exe".into(),
        "ChatGPT.exe".into(),
        "claude".into(),
        "Claude.exe".into(),
        "WorkBuddy.exe".into(),
        "workbuddy.exe".into(),
    ]
}
fn default_window_titles() -> Vec<String> {
    vec![
        "Codex".into(),
        "ChatGPT".into(),
        "Claude".into(),
        "WorkBuddy".into(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallConfig {
    #[serde(default = "default_true")]
    pub show_on_agent: bool,
    #[serde(default = "default_anchor")]
    pub anchor: String,
    #[serde(default)]
    pub position: BallPosition,
}

fn default_anchor() -> String {
    "right-center".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BallPosition {
    /// `None` => use `anchor` to auto-position on the primary monitor.
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionConfig {
    #[serde(default = "default_true")]
    pub autostart: bool,
    #[serde(default)]
    pub pause_detection: bool,
    /// When true, Companion UI features appear only while a supported Agent is running.
    #[serde(default = "default_true")]
    pub follow_agent_lifecycle: bool,
    /// Path to the Agent executable, used by the "Oasis" shortcut.
    #[serde(default)]
    pub agent_launch_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    /// Enabled Agent target IDs: e.g. ["codex", "claude-code", "workbuddy"].
    /// The Skill is installed to each target's directory.
    #[serde(default = "default_skill_targets")]
    pub targets: Vec<String>,
    #[serde(default)]
    pub installed_version: Option<String>,
}

fn default_skill_targets() -> Vec<String> {
    vec!["codex".into(), "claude-code".into(), "workbuddy".into()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRuntime {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub mcp: McpConfig,
    /// Safety-locked rules. NEVER overridable by project/user config.
    #[serde(default = "default_safety_locked")]
    pub safety_locked: Vec<String>,
}

fn default_mode() -> String {
    "normal".into()
}
fn default_safety_locked() -> Vec<String> {
    vec![
        "never modify UGC project files without explicit override".into(),
        "always back up .uasset outside the UGC project tree".into(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub enabled: bool,
    /// When true, Companion scans the editor process listening ports at startup
    /// and on demand, overriding `host`/`port` with the discovered SSE endpoint.
    #[serde(default = "default_true")]
    pub auto_discover: bool,
    /// Optional UGC project root, used to read its `.mcp.json` for the SSE URL.
    #[serde(default)]
    pub project_path: Option<String>,
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    #[serde(default = "default_mcp_host")]
    pub host: String,
    #[serde(default = "default_mcp_sse_path")]
    pub sse_path: String,
    #[serde(default)]
    pub last_checked_at: Option<String>,
    #[serde(default)]
    pub last_status: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub last_server_name: Option<String>,
    #[serde(default)]
    pub last_server_version: Option<String>,
    /// Cached result of the last successful `tools/list`.
    #[serde(default)]
    pub cached_tools: Vec<McpToolInfo>,
}

/// A single tool descriptor cached from `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpToolInfo {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

fn default_mcp_host() -> String {
    "127.0.0.1".into()
}

fn default_mcp_port() -> u16 {
    33444
}

fn default_mcp_sse_path() -> String {
    "/sse".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatesConfig {
    #[serde(default = "default_true")]
    pub auto_check: bool,
    #[serde(default = "default_update_provider")]
    pub provider: String,
    #[serde(default = "default_github_repo")]
    pub github_repo: String,
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default)]
    pub last_check_at: Option<String>,
    #[serde(default)]
    pub latest_version: Option<String>,
    #[serde(default)]
    pub latest_revision: Option<String>,
    #[serde(default)]
    pub latest_revision_date: Option<String>,
    #[serde(default)]
    pub installed_revision: Option<String>,
    #[serde(default)]
    pub latest_url: Option<String>,
    #[serde(default)]
    pub update_available: bool,
    #[serde(default)]
    pub last_error: Option<String>,
}

fn default_update_provider() -> String {
    "github".into()
}
fn default_github_repo() -> String {
    "mislw/oasis-wiki".into()
}
fn default_channel() -> String {
    "stable".into()
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema_version: 1,
            agent_detection: AgentDetection::default(),
            ball: BallConfig::default(),
            companion: CompanionConfig::default(),
            skill: SkillConfig::default(),
            skill_runtime: SkillRuntime::default(),
            updates: UpdatesConfig::default(),
        }
    }
}

impl Default for AgentDetection {
    fn default() -> Self {
        AgentDetection {
            enabled: true,
            interval_seconds: 3,
            process_names: default_process_names(),
            window_titles: default_window_titles(),
            install_paths: Vec::new(),
        }
    }
}

impl Default for BallConfig {
    fn default() -> Self {
        BallConfig {
            show_on_agent: true,
            anchor: default_anchor(),
            position: BallPosition::default(),
        }
    }
}

impl Default for CompanionConfig {
    fn default() -> Self {
        CompanionConfig {
            autostart: true,
            pause_detection: false,
            follow_agent_lifecycle: true,
            agent_launch_path: None,
        }
    }
}

impl Default for SkillConfig {
    fn default() -> Self {
        SkillConfig {
            targets: default_skill_targets(),
            installed_version: None,
        }
    }
}

impl Default for SkillRuntime {
    fn default() -> Self {
        SkillRuntime {
            mode: default_mode(),
            mcp: McpConfig::default(),
            safety_locked: default_safety_locked(),
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        McpConfig {
            enabled: false,
            auto_discover: true,
            project_path: None,
            port: default_mcp_port(),
            host: default_mcp_host(),
            sse_path: default_mcp_sse_path(),
            last_checked_at: None,
            last_status: None,
            last_error: None,
            last_server_name: None,
            last_server_version: None,
            cached_tools: Vec::new(),
        }
    }
}

impl Default for UpdatesConfig {
    fn default() -> Self {
        UpdatesConfig {
            auto_check: true,
            provider: default_update_provider(),
            github_repo: default_github_repo(),
            channel: default_channel(),
            last_check_at: None,
            latest_version: None,
            latest_revision: None,
            latest_revision_date: None,
            installed_revision: None,
            latest_url: None,
            update_available: false,
            last_error: None,
        }
    }
}
