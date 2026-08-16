//! Agent target registry — defines supported MCP-compatible Agent platforms
//! where the oasis-wiki Skill can be installed.
//!
//! Supported targets: Codex, Claude Code, WorkBuddy.
//! Each target knows its Skill install directory, MCP config file path,
//! process names, and window titles for detection.

use serde::{Deserialize, Serialize};

/// A supported Agent platform where the oasis-wiki Skill can be installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTarget {
    /// Short identifier: "codex", "claude-code", "workbuddy"
    pub id: String,
    /// Human-readable name for UI display
    pub display_name: String,
    /// Skill install directory (may contain %USERPROFILE%)
    pub skill_dir: String,
    /// Directories that may contain current or legacy copies of this Skill.
    pub skill_roots: Vec<String>,
    /// MCP config file path (may contain %USERPROFILE%)
    pub mcp_config_path: String,
    /// Process names to detect (Windows exe names)
    pub process_names: Vec<String>,
    /// Window titles to detect
    pub window_titles: Vec<String>,
}

/// Return all supported Agent targets.
pub fn all_targets() -> Vec<AgentTarget> {
    vec![
        AgentTarget {
            id: "codex".into(),
            display_name: "Codex".into(),
            skill_dir: "%USERPROFILE%\\.codex\\skills\\oasis-wiki".into(),
            skill_roots: vec!["%USERPROFILE%\\.codex\\skills".into()],
            mcp_config_path: "%USERPROFILE%\\.codex\\config.json".into(),
            process_names: vec!["Codex.exe".into(), "ChatGPT.exe".into()],
            window_titles: vec!["Codex".into(), "ChatGPT".into()],
        },
        AgentTarget {
            id: "claude-code".into(),
            display_name: "Claude Code".into(),
            skill_dir: "%USERPROFILE%\\.claude\\commands\\oasis-wiki".into(),
            skill_roots: vec![
                "%USERPROFILE%\\.claude\\commands".into(),
                "%USERPROFILE%\\.claude\\skills".into(),
            ],
            mcp_config_path: "%USERPROFILE%\\.claude\\claude_desktop_config.json".into(),
            process_names: vec!["claude".into(), "Claude.exe".into()],
            window_titles: vec!["Claude".into()],
        },
        AgentTarget {
            id: "workbuddy".into(),
            display_name: "WorkBuddy".into(),
            skill_dir: "%USERPROFILE%\\.workbuddy\\skills\\oasis-wiki".into(),
            skill_roots: vec!["%USERPROFILE%\\.workbuddy\\skills".into()],
            mcp_config_path: "%USERPROFILE%\\.workbuddy\\mcp.json".into(),
            process_names: vec!["WorkBuddy.exe".into(), "workbuddy.exe".into()],
            window_titles: vec!["WorkBuddy".into()],
        },
    ]
}

/// Find a target by id.
#[allow(dead_code)]
pub fn find_target(id: &str) -> Option<AgentTarget> {
    all_targets().into_iter().find(|t| t.id == id)
}

/// Return all process names across all targets (for default detection config).
#[allow(dead_code)]
pub fn all_process_names() -> Vec<String> {
    all_targets()
        .iter()
        .flat_map(|t| t.process_names.clone())
        .collect()
}

/// Return all window titles across all targets (for default detection config).
#[allow(dead_code)]
pub fn all_window_titles() -> Vec<String> {
    all_targets()
        .iter()
        .flat_map(|t| t.window_titles.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_three_targets() {
        assert_eq!(all_targets().len(), 3);
    }

    #[test]
    fn find_by_id() {
        assert!(find_target("codex").is_some());
        assert!(find_target("claude-code").is_some());
        assert!(find_target("workbuddy").is_some());
        assert!(find_target("nonexistent").is_none());
    }

    #[test]
    fn all_process_names_covers_all_targets() {
        let names = all_process_names();
        assert!(names.contains(&"Codex.exe".to_string()));
        assert!(names.contains(&"claude".to_string()));
        assert!(names.contains(&"WorkBuddy.exe".to_string()));
    }

    #[test]
    fn claude_target_scans_current_and_legacy_skill_roots() {
        let target = find_target("claude-code").unwrap();
        assert_eq!(
            target.skill_roots,
            vec![
                "%USERPROFILE%\\.claude\\commands".to_string(),
                "%USERPROFILE%\\.claude\\skills".to_string(),
            ]
        );
    }
}
