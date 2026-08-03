// TypeScript mirrors of the Rust serde types (all snake_case).

export type BallState =
  | "hidden"
  | "idle"
  | "error";

export interface McpToolInfo {
  name: string;
  description: string;
}

export interface Settings {
  schema_version: number;
  agent_detection: {
    enabled: boolean;
    interval_seconds: number;
    process_names: string[];
    window_titles: string[];
    install_paths: string[];
  };
  ball: {
    show_on_agent: boolean;
    anchor: string;
    position: { x: number | null; y: number | null };
  };
  companion: {
    autostart: boolean;
    pause_detection: boolean;
    follow_agent_lifecycle: boolean;
    agent_launch_path: string | null;
  };
  skill: {
    targets: string[];
    installed_version: string | null;
  };
  skill_runtime: {
    mode: string;
    mcp: {
      enabled: boolean;
      auto_discover: boolean;
      project_path: string | null;
      port: number;
      host: string;
      sse_path: string;
      last_checked_at: string | null;
      last_status: string | null;
      last_error: string | null;
      last_server_name: string | null;
      last_server_version: string | null;
      cached_tools: McpToolInfo[];
    };
    safety_locked: string[];
  };
  updates: {
    auto_check: boolean;
    provider: string;
    github_repo: string;
    channel: string;
    last_check_at: string | null;
    latest_version: string | null;
    latest_revision: string | null;
    installed_revision: string | null;
    latest_url: string | null;
    update_available: boolean;
    last_error: string | null;
  };
}

export type SkillStatus =
  | { kind: "not_installed" }
  | { kind: "installed" }
  | { kind: "version_mismatch"; installed: string; expected: string };

export interface TargetStatus {
  target_id: string;
  display_name: string;
  status: SkillStatus;
}

export interface MultiTargetStatus {
  targets: TargetStatus[];
}

export interface AgentTarget {
  id: string;
  display_name: string;
  skill_dir: string;
  mcp_config_path: string;
  process_names: string[];
  window_titles: string[];
}

export interface UpdateStatus {
  checked: boolean;
  update_available: boolean;
  source: "release" | "commit" | null;
  current_version: string;
  latest_version: string | null;
  latest_revision: string | null;
  installed_revision: string | null;
  latest_url: string | null;
  error: string | null;
}

export interface UpdateInstallResult {
  status: UpdateStatus;
  skill_status: MultiTargetStatus;
}

export interface McpServerInfo {
  name: string;
  version: string;
}

export type McpState = "disabled" | "unchecked" | "disconnected" | "connected";

export interface McpStatus {
  enabled: boolean;
  state: McpState;
  url: string;
  server_info: McpServerInfo | null;
  tools: McpToolInfo[];
  checked_at: string;
  error: string | null;
}

export interface McpToolCallResult {
  content: unknown;
  is_error: boolean;
}

export interface DiscoveredEndpoint {
  host: string;
  port: number;
  sse_path: string;
  source: string;
}
