import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  selectSettingsPageForAgent,
  settingsOverviewPage,
  type SettingsPage,
} from "./settingsNavigation";
import type {
  BallState,
  DiscoveredEndpoint,
  McpState,
  McpStatus,
  McpToolCallResult,
  MultiTargetStatus,
  Settings,
  UpdateInstallResult,
  UpdateStatus,
} from "../types";

const EXPECTED_VERSION = "1.260818.5";
const CORE_MCP_TOOLS = ["ue_read", "ue_py", "ue_plan_submit"];
const MCP_AUTO_CHECK_INTERVAL_MS = 5 * 60 * 1000;

const AGENT_TARGETS = [
  { id: "codex", display_name: "Codex", skill_dir: "~/.codex/skills/oasis-wiki" },
  { id: "claude-code", display_name: "Claude Code", skill_dir: "~/.claude/commands/oasis-wiki" },
  { id: "workbuddy", display_name: "WorkBuddy", skill_dir: "~/.workbuddy/skills/oasis-wiki" },
];

type TabId = "overview" | "mcp" | "skill" | "updates";
type PopoverSide = "left" | "right" | "floating";

const TABS: Array<{ id: TabId; label: string }> = [
  { id: "overview", label: "总览" },
  { id: "mcp", label: "MCP" },
  { id: "skill", label: "Skill" },
  { id: "updates", label: "更新" },
];

export default function SettingsWindow() {
  const [settingsPage, setSettingsPage] = useState<SettingsPage>(settingsOverviewPage);
  const agentId = settingsPage.kind === "agent" ? settingsPage.agentId : null;
  const isAgentPage = settingsPage.kind === "agent";
  const currentAgent = useMemo(
    () => AGENT_TARGETS.find((target) => target.id === agentId) ?? null,
    [agentId],
  );
  const [activeTab, setActiveTab] = useState<TabId>(() => (isAgentPage ? "skill" : "mcp"));
  const [settings, setSettings] = useState<Settings | null>(null);
  const [skill, setSkill] = useState<MultiTargetStatus | null>(null);
  const [activeTargets, setActiveTargets] = useState<string[]>([]);
  const [ballState, setBallState] = useState<BallState>("hidden");
  const [agentPresent, setAgentPresent] = useState(false);
  const [update, setUpdate] = useState<UpdateStatus | null>(null);
  const [mcpStatus, setMcpStatus] = useState<McpStatus | null>(null);
  const [mcpResult, setMcpResult] = useState<string | null>(null);
  const [namesText, setNamesText] = useState("");
  const [busy, setBusy] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  const [popoverSide, setPopoverSide] = useState<PopoverSide>("floating");
  const [showCompactHome, setShowCompactHome] = useState(true);

  async function reload() {
    const s = await invoke<Settings>("get_settings");
    setSettings(s);
    setNamesText(s.agent_detection.process_names.join(", "));
    setSkill(await invoke<MultiTargetStatus>("get_skill_status"));
    setActiveTargets(await invoke<string[]>("get_active_targets"));
    setBallState(await invoke<BallState>("get_ball_state"));
    setAgentPresent(await invoke<boolean>("get_agent_present"));
    setUpdate({
      checked: Boolean(s.updates.last_check_at),
      update_available: s.updates.update_available,
      source: null,
      current_version: EXPECTED_VERSION,
      latest_version: s.updates.latest_version,
      latest_revision: s.updates.latest_revision,
      latest_revision_date: s.updates.latest_revision_date,
      installed_revision: s.updates.installed_revision,
      latest_url: s.updates.latest_url,
      error: s.updates.last_error,
    });

    const mcpState: McpState = s.skill_runtime.mcp.enabled
      ? (s.skill_runtime.mcp.last_status as McpState | null) ?? "unchecked"
      : "disabled";
    setMcpStatus({
      enabled: s.skill_runtime.mcp.enabled,
      state: mcpState,
      url: mcpUrl(s),
      server_info: s.skill_runtime.mcp.last_server_name
        ? {
            name: s.skill_runtime.mcp.last_server_name,
            version: s.skill_runtime.mcp.last_server_version ?? "",
          }
        : null,
      tools: s.skill_runtime.mcp.cached_tools,
      checked_at: s.skill_runtime.mcp.last_checked_at ?? "",
      error: s.skill_runtime.mcp.last_error,
    });
  }

  useEffect(() => {
    reload();
    const unlistenBall = listen<BallState>("ball://state", (event) => setBallState(event.payload));
    const unlistenAgent = listen<boolean>("agent://presence", (event) => setAgentPresent(event.payload));
    const unlistenActiveTargets = listen<string[]>("agent://active-targets", (event) =>
      setActiveTargets(event.payload),
    );
    const unlistenMcp = listen<McpStatus>("mcp-status-changed", (event) => setMcpStatus(event.payload));
    const unlistenPopover = listen<PopoverSide>("settings://popover-side", (event) =>
      setPopoverSide(event.payload),
    );
    const unlistenShowHome = listen("settings://show-home", () => {
      setSettingsPage(settingsOverviewPage());
      setShowCompactHome(true);
    });
    const unlistenSelectAgent = listen<string>("settings://select-agent", (event) => {
      try {
        setSettingsPage(selectSettingsPageForAgent(
          event.payload,
          AGENT_TARGETS.map((target) => target.id),
        ));
        setShowCompactHome(false);
      } catch (error) {
        flash(String(error));
      }
    });
    return () => {
      unlistenBall.then((fn) => fn());
      unlistenAgent.then((fn) => fn());
      unlistenActiveTargets.then((fn) => fn());
      unlistenMcp.then((fn) => fn());
      unlistenPopover.then((fn) => fn());
      unlistenShowHome.then((fn) => fn());
      unlistenSelectAgent.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const window = getCurrentWebviewWindow();
    const compact = !isAgentPage && showCompactHome;
    window.setSize(new LogicalSize(compact ? 380 : 560, compact ? 390 : 680)).catch(() => undefined);
  }, [isAgentPage, showCompactHome]);

  useEffect(() => {
    if (
      activeTab !== "mcp" ||
      !settings?.skill_runtime.mcp.enabled ||
      (settings.companion.follow_agent_lifecycle && !agentPresent)
    ) return;
    const timer = window.setInterval(() => {
      probeMcpStatus(true);
    }, MCP_AUTO_CHECK_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [activeTab, settings?.skill_runtime.mcp.enabled, settings?.companion.follow_agent_lifecycle, agentPresent]);

  function flash(message: string) {
    setToast(message);
    window.setTimeout(() => setToast(null), 2500);
  }

  async function closeSettings() {
    try {
      await invoke("close_settings");
    } catch (error) {
      flash("关闭设置失败: " + error);
    }
  }

  async function openAgentSettings(targetId: string) {
    try {
      await invoke("open_agent_settings", { targetId });
    } catch (error) {
      flash("打开专页失败: " + error);
    }
  }

  async function openUIWorkbench() {
    try {
      await invoke("open_ui_workbench");
    } catch (error) {
      flash("打开 UI 工作台失败: " + error);
    }
  }

  async function openUIWorkflow() {
    try {
      await invoke("open_ui_workflow");
    } catch (error) {
      flash("打开 UI 生图工具链失败: " + error);
    }
  }

  async function save(next: Settings) {
    setBusy(true);
    try {
      await invoke("save_settings", { settings: next });
      setSettings(next);
      setBallState(await invoke<BallState>("get_ball_state"));
    } catch (error) {
      flash("保存失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  function patch(partial: Partial<Settings>) {
    if (!settings) return;
    save({ ...settings, ...partial });
  }

  function patchMcp(partial: Partial<Settings["skill_runtime"]["mcp"]>) {
    if (!settings) return;
    patch({
      skill_runtime: {
        ...settings.skill_runtime,
        mcp: {
          ...settings.skill_runtime.mcp,
          ...partial,
        },
      },
    });
  }

  function setRuntimeMode(mode: "normal" | "teaching") {
    if (!settings || settings.skill_runtime.mode === mode) return;
    save({
      ...settings,
      skill_runtime: {
        ...settings.skill_runtime,
        mode,
      },
    });
    flash(mode === "teaching" ? "已切换到教学模式" : "已切换到正常模式");
  }

  async function toggleAutostart(enabled: boolean) {
    setBusy(true);
    try {
      await invoke("set_autostart_enabled", { enabled });
      if (settings) {
        setSettings({ ...settings, companion: { ...settings.companion, autostart: enabled } });
      }
      flash(enabled ? "已启用开机后台启动" : "已关闭开机后台启动");
    } catch (error) {
      flash("自启设置失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  async function togglePause(paused: boolean) {
    setBusy(true);
    try {
      await invoke("set_pause_detection", { paused });
      if (settings) {
        setSettings({ ...settings, companion: { ...settings.companion, pause_detection: paused } });
      }
      setBallState(await invoke<BallState>("get_ball_state"));
      flash(paused ? "已暂停 Agent 检测" : "已恢复 Agent 检测");
    } catch (error) {
      flash("检测设置失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  async function reinstall() {
    const ok = window.confirm(
      `将使用内置 oasis-wiki v${EXPECTED_VERSION} 覆盖选中 Agent 的现有 Skill。确认继续？`,
    );
    if (!ok) return;
    setBusy(true);
    try {
      const status = await invoke<MultiTargetStatus>("reinstall_skill");
      setSkill(status);
      flash("Skill 已重新安装到选中的 Agent");
    } catch (error) {
      flash("安装失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  async function reinstallCurrentTarget() {
    if (!currentAgent) return;
    setBusy(true);
    try {
      const status = await invoke<MultiTargetStatus>("reinstall_skill_for_target", {
        target_id: currentAgent.id,
      });
      setSkill(status);
      flash(`已重装到 ${currentAgent.display_name}`);
    } catch (error) {
      flash("安装失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  async function refreshSkill() {
    const status = await invoke<MultiTargetStatus>("refresh_skill_status_cmd");
    setSkill(status);
    flash("已刷新 Skill 状态");
  }

  async function checkUpdates() {
    setBusy(true);
    try {
      const status = await invoke<UpdateStatus>("check_updates");
      setUpdate(status);
      setBallState(await invoke<BallState>("get_ball_state"));
      await reload();
      if (status.update_available) {
        flash("发现 GitHub 更新");
      } else if (status.error) {
        flash("检查失败: " + status.error);
      } else {
        flash("当前已是最新");
      }
    } catch (error) {
      flash("检查更新失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  async function installUpdate() {
    const ok = window.confirm("将从 GitHub 下载最新 oasis-wiki Skill，并安装到当前选中的 Agent 目标。继续？");
    if (!ok) return;
    setBusy(true);
    try {
      const result = await invoke<UpdateInstallResult>("install_latest_update");
      setUpdate(result.status);
      setSkill(result.skill_status);
      setBallState(await invoke<BallState>("get_ball_state"));
      await reload();
      flash("更新已安装");
    } catch (error) {
      flash("自动更新失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  async function probeMcpStatus(silent: boolean) {
    if (!silent) setBusy(true);
    try {
      const status = await invoke<McpStatus>("check_mcp_status");
      setMcpStatus(status);
      await reload();
      if (!silent) {
        flash(status.state === "connected" ? "MCP 已连接" : "MCP 未连接: " + (status.error ?? status.url));
      }
    } catch (error) {
      if (!silent) flash("MCP 检查失败: " + error);
    } finally {
      if (!silent) setBusy(false);
    }
  }

  async function checkMcpStatus() {
    await probeMcpStatus(false);
  }

  async function connectMcpAuto() {
    setBusy(true);
    try {
      const status = await invoke<McpStatus>("connect_mcp_auto");
      setMcpStatus(status);
      await reload();
      if (status.state === "connected") {
        flash(`HTTP 直连已启用，发现 ${status.tools.length} 个工具`);
      } else {
        flash("HTTP 直连失败: " + (status.error ?? status.url));
      }
    } catch (error) {
      flash("HTTP 直连失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  async function disableMcp() {
    setBusy(true);
    try {
      const status = await invoke<McpStatus>("disable_mcp");
      setMcpResult(null);
      setMcpStatus(status);
      await reload();
      setBallState(await invoke<BallState>("get_ball_state"));
      flash("已跳过编辑器 MCP");
    } catch (error) {
      flash("跳过编辑器 MCP 失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  async function enableMcp() {
    if (!settings) return;
    setBusy(true);
    const next: Settings = {
      ...settings,
      skill_runtime: {
        ...settings.skill_runtime,
        mcp: {
          ...settings.skill_runtime.mcp,
          enabled: true,
          last_status: null,
          last_error: null,
        },
      },
    };
    try {
      await invoke("save_settings", { settings: next });
      setSettings(next);
      setMcpStatus(null);
      setMcpResult(null);
      setBallState(await invoke<BallState>("get_ball_state"));
      flash("已解除 MCP 禁用");
    } catch (error) {
      flash("解除禁用失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  async function discoverMcp() {
    setBusy(true);
    try {
      const ep = await invoke<DiscoveredEndpoint | null>("discover_mcp");
      if (ep) {
        flash(`已发现 MCP 端点 (${ep.source}): ${ep.host}:${ep.port}${ep.sse_path}`);
        await reload();
      } else {
        flash("未发现编辑器 MCP 端点，请确认编辑器 MCP Server 已启动");
      }
    } catch (error) {
      flash("发现失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  async function callMcpRead() {
    setBusy(true);
    setMcpResult(null);
    try {
      const result = await invoke<McpToolCallResult>("call_mcp_tool", {
        name: "ue_read",
        arguments: { queries: ["ctx:"] },
      });
      setMcpResult(JSON.stringify(result.content, null, 2));
      if (result.is_error) flash("ue_read 返回错误");
    } catch (error) {
      setMcpResult("调用失败: " + error);
    } finally {
      setBusy(false);
    }
  }

  const skillText = useMemo(() => {
    if (!skill || skill.targets.length === 0) return "检测中";
    const installed = skill.targets.filter((t) => t.status.kind === "installed").length;
    const total = skill.targets.length;
    if (installed === total) return `v${EXPECTED_VERSION} · 已安装`;
    if (installed === 0) return "全部未安装";
    return `${installed}/${total} 已安装`;
  }, [skill]);

  const currentTargetStatus = useMemo(() => {
    if (!currentAgent || !skill) return null;
    return skill.targets.find((t) => t.target_id === currentAgent.id) ?? null;
  }, [currentAgent, skill]);

  if (!settings) {
    return <div className="settings"><p>加载中...</p></div>;
  }

  if (isAgentPage && currentAgent) {
    const currentEnabled = settings.skill.targets.includes(currentAgent.id);
    const currentDetected = activeTargets.includes(currentAgent.id);
    const currentStatusText = !currentTargetStatus
      ? "未检测"
      : currentTargetStatus.status.kind === "installed"
        ? "已安装"
        : currentTargetStatus.status.kind === "not_installed"
          ? "未安装"
          : "版本不一致";

    return (
      <div className={`settings settings-wide settings-popover settings-agent-page settings-popover-${popoverSide}`}>
        <header className="settings-header">
          <span className="settings-logo" aria-hidden="true" />
          <div className="settings-title">
            <h1>{currentAgent.display_name}</h1>
            <p>只管理这个 agent 的 oasis-wiki Skill</p>
          </div>
          <span className={`ball-pill ball-pill-${ballState}`}>状态: {ballStateLabel(ballState)}</span>
          <button className="window-close" type="button" onClick={closeSettings} aria-label="关闭设置">
            ×
          </button>
        </header>

        <section>
          <h2>Skill</h2>
          <div className="row">
            <span>当前状态</span>
            <strong>{currentStatusText}</strong>
          </div>
          <div className="row">
            <span>检测结果</span>
            <strong>{currentDetected ? "正在运行" : "未检测到"}</strong>
          </div>
          <label className="row">
            <span>启用这个 agent 的 Skill</span>
            <input
              type="checkbox"
              checked={currentEnabled}
              onChange={(event) => {
                const next = event.target.checked
                  ? [...settings.skill.targets, currentAgent.id]
                  : settings.skill.targets.filter((id) => id !== currentAgent.id);
                patch({ skill: { ...settings.skill, targets: next } });
              }}
            />
          </label>
          <div className="col">
            <span>Skill 路径</span>
            <code>{currentAgent.skill_dir}</code>
          </div>
          <div className="actions action-wrap">
            <button onClick={refreshSkill} disabled={busy}>刷新状态</button>
            <button onClick={reinstallCurrentTarget} disabled={busy}>重装当前 agent Skill</button>
            <button className="btn-secondary" onClick={() => setSettingsPage(settingsOverviewPage())}>打开总览</button>
          </div>
        </section>

        {toast && <div className="toast">{toast}</div>}
        {busy && <div className="busy">处理中..</div>}
      </div>
    );
  }

  const currentMcpUrl = mcpUrl(settings);
  const mcpState: McpState = !settings.skill_runtime.mcp.enabled
    ? "disabled"
    : mcpStatus?.state === "disabled"
      ? "unchecked"
      : mcpStatus?.state ?? "unchecked";
  const mcpLabel = mcpStateLabel(mcpState);
  const coreTools = new Set(mcpStatus?.tools.map((tool) => tool.name) ?? []);
  const updateText = updateSummary(update);

  if (showCompactHome) {
    return (
      <div className={`settings compact-home settings-popover settings-popover-${popoverSide}`}>
        <header className="compact-home-header">
          <span className="settings-logo" aria-hidden="true" />
          <div className="settings-title">
            <div className="settings-title-line">
              <h1>Oasis Companion</h1>
              <span className="app-version">v{EXPECTED_VERSION}</span>
            </div>
            <p>随 Agent 自动出现的桌面控制器</p>
          </div>
          <span className={`ball-pill ball-pill-${ballState}`}>{agentPresent ? "Agent 在线" : "待机"}</span>
          <button className="compact-icon-button" type="button" onClick={() => setShowCompactHome(false)} aria-label="打开设置" title="打开设置">
            ⚙
          </button>
          <button className="window-close" type="button" onClick={closeSettings} aria-label="关闭窗口">
            ×
          </button>
        </header>

        <section className="compact-card compact-mode-card">
          <h2>工作模式</h2>
          <div className="segmented" role="group" aria-label="Skill runtime mode">
            <button
              type="button"
              className={settings.skill_runtime.mode === "normal" ? "active" : ""}
              onClick={() => setRuntimeMode("normal")}
              disabled={busy}
            >
              正常模式
            </button>
            <button
              type="button"
              className={settings.skill_runtime.mode === "teaching" ? "active" : ""}
              onClick={() => setRuntimeMode("teaching")}
              disabled={busy}
            >
              教学模式
            </button>
          </div>
        </section>

        <section className={`compact-card compact-mcp-card mcp-status-${mcpState}`}>
          <div className="compact-status-line">
            <div>
              <h2>编辑器 MCP</h2>
              <strong>{mcpLabel}</strong>
            </div>
            <span className={`status-dot status-${mcpState}`} aria-hidden="true" />
          </div>
          <button className="btn-primary compact-primary-action" onClick={connectMcpAuto} disabled={busy || mcpState === "connected"}>
            {mcpState === "connected" ? "编辑器已连接" : "连接编辑器 MCP"}
          </button>
          <button className="compact-text-action" type="button" onClick={() => { setShowCompactHome(false); setActiveTab("mcp"); }}>
            查看 MCP 详情
          </button>
        </section>

        <footer className="compact-footer">
          <button className="compact-tool-launch" type="button" onClick={openUIWorkflow}>
            <span>UI 生图工具链</span><strong>八阶段进度与交付</strong>
          </button>
          <button className="compact-tool-launch" type="button" onClick={openUIWorkbench}>
            <span>UI 工作台</span><strong>切图与控件编辑</strong>
          </button>
          <button type="button" onClick={() => { setShowCompactHome(false); setActiveTab("skill"); }}>
            <span>Skill 版本</span><strong>{skillText}</strong>
          </button>
          <button type="button" onClick={() => { setShowCompactHome(false); setActiveTab("updates"); }}>
            <span>更新</span><strong>{update?.update_available ? "有新版本" : "已是最新"}</strong>
          </button>
        </footer>

        {toast && <div className="toast">{toast}</div>}
        {busy && <div className="busy">处理中...</div>}
      </div>
    );
  }

  return (
    <div className={`settings settings-wide settings-popover settings-main-page settings-popover-${popoverSide}`}>
      <header className="settings-header">
        <span className="settings-logo" aria-hidden="true" />
        <div className="settings-title">
          <div className="settings-title-line">
            <h1>Oasis Companion</h1>
            <span className="app-version">v{EXPECTED_VERSION}</span>
          </div>
          <p>随 Agent 自动出现的 oasis-wiki 桌面控制器</p>
        </div>
        <span className={`ball-pill ball-pill-${ballState}`}>状态: {ballStateLabel(ballState)}</span>
        <button className="compact-text-action settings-home-link" type="button" onClick={() => setShowCompactHome(true)}>
          首页
        </button>
        <button className="window-close" type="button" onClick={closeSettings} aria-label="关闭设置">
          ×
        </button>
      </header>

      <nav className="tabs" aria-label="设置分类">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            type="button"
            className={activeTab === tab.id ? "active" : ""}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </nav>

      {activeTab === "overview" && (
        <>
          <section>
            <h2>UI 工具</h2>
            <div className="row">
              <span>查看八阶段进度并继续 Agent、Workbench 与编辑器交付</span>
              <button type="button" onClick={openUIWorkflow}>打开 UI 生图工具链</button>
            </div>
            <div className="row">
              <span>导入 UI 图与 UI Tree，编辑控件范围并查看切图候选</span>
              <button type="button" onClick={openUIWorkbench}>打开 UI 工作台</button>
            </div>
          </section>

          <section>
            <h2>模式</h2>
            <div className="segmented" role="group" aria-label="Skill runtime mode">
              <button
                type="button"
                className={settings.skill_runtime.mode === "normal" ? "active" : ""}
                onClick={() => setRuntimeMode("normal")}
                disabled={busy}
              >
                正常模式
              </button>
              <button
                type="button"
                className={settings.skill_runtime.mode === "teaching" ? "active" : ""}
                onClick={() => setRuntimeMode("teaching")}
                disabled={busy}
              >
                教学模式
              </button>
            </div>
          </section>

          <section>
            <h2>显示</h2>
            <label className="row">
              <span>打开 Agent 时显示小球</span>
              <input
                type="checkbox"
                checked={settings.ball.show_on_agent}
                onChange={(event) => patch({ ball: { ...settings.ball, show_on_agent: event.target.checked } })}
              />
            </label>
          </section>

          <section>
            <h2>系统</h2>
            <label className="row">
              <span>Windows 登录后后台启动</span>
              <input
                type="checkbox"
                checked={settings.companion.autostart}
                onChange={(event) => toggleAutostart(event.target.checked)}
              />
            </label>
            <label className="row">
              <span>暂停 Agent 检测</span>
              <input
                type="checkbox"
                checked={settings.companion.pause_detection}
                onChange={(event) => togglePause(event.target.checked)}
              />
            </label>
            <label className="row">
              <span>跟随 Agent 开关显示插件</span>
              <input
                type="checkbox"
                checked={settings.companion.follow_agent_lifecycle}
                onChange={(event) =>
                  patch({
                    companion: {
                      ...settings.companion,
                      follow_agent_lifecycle: event.target.checked,
                    },
                  })
                }
              />
            </label>
          </section>

          <section>
            <h2>Agent 检测</h2>
            <label className="col">
              <span>检测进程名，以英文逗号分隔</span>
              <input
                type="text"
                value={namesText}
                onChange={(event) => setNamesText(event.target.value)}
                onBlur={() =>
                  patch({
                    agent_detection: {
                      ...settings.agent_detection,
                      process_names: namesText.split(",").map((name) => name.trim()).filter(Boolean),
                    },
                  })
                }
                placeholder="Codex.exe, ChatGPT.exe"
              />
            </label>
            <label className="col">
              <span>检测间隔（秒）</span>
              <input
                type="number"
                min={1}
                value={settings.agent_detection.interval_seconds}
                onChange={(event) =>
                  patch({
                    agent_detection: {
                      ...settings.agent_detection,
                      interval_seconds: Math.max(1, Number(event.target.value) || 3),
                    },
                  })
                }
              />
            </label>
          </section>

          <section>
            <h2>Oasis Agent 快捷方式</h2>
            <label className="col">
              <span>Agent 可执行文件路径，留空时打开设置页</span>
              <input
                type="text"
                value={settings.companion.agent_launch_path ?? ""}
                onChange={(event) =>
                  patch({
                    companion: {
                      ...settings.companion,
                      agent_launch_path: event.target.value || null,
                    },
                  })
                }
                placeholder="C:\\Program Files\\Codex\\Codex.exe"
              />
            </label>
            <button onClick={() => invoke("launch_agent")}>立即启动 Agent</button>
          </section>
        </>
      )}

      {activeTab === "mcp" && (
        <section className="mcp-page">
          <div className={`mcp-status-card mcp-status-${mcpState}`}>
            <div>
              <h2>编辑器 MCP</h2>
              <strong>{mcpLabel}</strong>
              <p>HTTP/SSE 直连 UGCAskQ 编辑器服务，不写 Codex 原生 MCP 配置。</p>
              <p className="mcp-auto-note">MCP 页打开时每 5 分钟自动复查一次；这里显示的是上次检查结果。</p>
            </div>
            <span className={`status-dot status-${mcpState}`} />
          </div>

          <div className="direct-mcp-box">
            <div>
              <strong>编辑器直连</strong>
              <span>由 Companion 连接本机 UGCAskQ 的 HTTP/SSE 端点，用于检查工具和读取上下文。</span>
            </div>
            <button
              className="btn-primary"
              onClick={connectMcpAuto}
              disabled={busy}
            >
              连接编辑器（HTTP 直连）
            </button>
          </div>
          {settings.skill_runtime.mcp.enabled && (
            <div className="actions">
              <button className="btn-danger" onClick={disableMcp} disabled={busy}>
                跳过编辑器 MCP
              </button>
            </div>
          )}
          {!settings.skill_runtime.mcp.enabled && (
            <div className="mcp-bypass-box">
              已跳过编辑器 MCP。oasis-wiki Skill 会优先使用 wiki、项目文件和本地配置回答。
            </div>
          )}

          <div className="mcp-grid">
            <label className="row">
              <span>允许 Skill 使用 MCP</span>
              <input
                type="checkbox"
                checked={settings.skill_runtime.mcp.enabled}
                onChange={(event) => {
                  const enabled = event.target.checked;
                  if (enabled) {
                    enableMcp();
                  } else {
                    disableMcp();
                  }
                }}
              />
            </label>
            <label className="row">
              <span>自动发现端点</span>
              <input
                type="checkbox"
                checked={settings.skill_runtime.mcp.auto_discover}
                onChange={(event) => patchMcp({ auto_discover: event.target.checked })}
              />
            </label>
          </div>

          <label className="col">
            <span>UGC 项目路径（用于读取 .mcp.json）</span>
            <input
              type="text"
              value={settings.skill_runtime.mcp.project_path ?? ""}
              onChange={(event) => patchMcp({ project_path: event.target.value || null })}
              placeholder="E:\\...\\UGCProjects\\RedCliff"
            />
          </label>

          <div className="mcp-endpoint">
            <label className="col">
              <span>Host</span>
              <input
                type="text"
                value={settings.skill_runtime.mcp.host}
                onChange={(event) => patchMcp({ host: event.target.value || "127.0.0.1" })}
                placeholder="127.0.0.1"
              />
            </label>
            <label className="col">
              <span>Port</span>
              <input
                type="number"
                min={1}
                max={65535}
                value={settings.skill_runtime.mcp.port}
                onChange={(event) =>
                  patchMcp({ port: Math.max(1, Math.min(65535, Number(event.target.value) || 33444)) })
                }
              />
            </label>
            <label className="col">
              <span>SSE Path</span>
              <input
                type="text"
                value={settings.skill_runtime.mcp.sse_path}
                onChange={(event) => patchMcp({ sse_path: event.target.value || "/sse" })}
                placeholder="/sse"
              />
            </label>
          </div>

          <div className="url-box">
            <span>当前直连地址</span>
            <code>{currentMcpUrl}</code>
          </div>

          <div className="actions action-wrap">
            <button onClick={discoverMcp} disabled={busy}>自动发现</button>
            <button onClick={checkMcpStatus} disabled={busy}>检查连接</button>
            <button onClick={callMcpRead} disabled={busy || !settings.skill_runtime.mcp.enabled}>
              读取编辑器上下文
            </button>
          </div>

          {mcpStatus?.server_info && (
            <div className="info-row">
              <span>Server</span>
              <strong>{mcpStatus.server_info.name} {mcpStatus.server_info.version}</strong>
            </div>
          )}
          {mcpStatus?.checked_at && (
            <div className="info-row">
              <span>上次检查</span>
              <strong>{formatEpochSeconds(mcpStatus.checked_at)}</strong>
            </div>
          )}
          {mcpStatus?.error && <p className="error-text">{mcpStatus.error}</p>}

          <div className="col">
            <span>核心工具</span>
            <div className="core-tools">
              {CORE_MCP_TOOLS.map((name) => (
                <span key={name} className={coreTools.has(name) ? "ok" : "missing"}>
                  {name}
                </span>
              ))}
            </div>
          </div>

          {mcpStatus?.tools && mcpStatus.tools.length > 0 && (
            <div className="col">
              <span>可用工具 ({mcpStatus.tools.length})</span>
              <ul className="tool-list">
                {mcpStatus.tools.map((tool) => (
                  <li key={tool.name} className={CORE_MCP_TOOLS.includes(tool.name) ? "core" : ""}>
                    <code>{tool.name}</code>
                    <small>{tool.description.slice(0, 120)}</small>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {mcpResult && (
            <div className="col">
              <span>ue_read ctx: 结果</span>
              <pre className="result-box">{mcpResult}</pre>
            </div>
          )}
        </section>
      )}

      {activeTab === "skill" && (
        <section>
          <h2>Skill 安装</h2>
          <div className="warning-box">
            重新安装会把内置 Skill 覆盖到选中目标。当前内置资源如仍是 stub，请先替换为完整 oasis-wiki。
          </div>
          <div className="row">
            <span>安装状态</span>
            <strong>{skillText}</strong>
          </div>
          <div className="col">
            <span>安装目标</span>
            {AGENT_TARGETS.map((target) => {
              const enabled = settings.skill.targets.includes(target.id);
              const targetStatus = skill?.targets.find((t) => t.target_id === target.id);
              const statusText = !targetStatus
                ? "未检测"
                : targetStatus.status.kind === "installed"
                  ? "已安装"
                  : targetStatus.status.kind === "not_installed"
                    ? "未安装"
                    : "版本不一致";
              return (
                <div key={target.id} className="row target-row">
                  <span>
                    {target.display_name}
                    <small>{target.skill_dir} / {statusText}</small>
                  </span>
                  <div className="target-actions">
                    <button className="btn-secondary" onClick={() => openAgentSettings(target.id)}>
                      打开专页
                    </button>
                    <input
                      type="checkbox"
                      checked={enabled}
                      onChange={(event) => {
                        const next = event.target.checked
                          ? [...settings.skill.targets, target.id]
                          : settings.skill.targets.filter((id) => id !== target.id);
                        patch({ skill: { ...settings.skill, targets: next } });
                      }}
                    />
                  </div>
                </div>
              );
            })}
          </div>
          <div className="actions">
            <button onClick={refreshSkill}>检查 Skill 状态</button>
            <button onClick={reinstall} disabled={busy}>重新安装 Skill</button>
          </div>
        </section>
      )}

      {activeTab === "updates" && (
        <section>
          <h2>Skill 更新</h2>
          <div className="warning-box">
            这里只更新 oasis-wiki Skill，不会安装 Companion MSI。Companion 当前版本为 v{EXPECTED_VERSION}。
          </div>
          <label className="row">
            <span>自动检查</span>
            <input
              type="checkbox"
              checked={settings.updates.auto_check}
              onChange={(event) => patch({ updates: { ...settings.updates, auto_check: event.target.checked } })}
            />
          </label>
          <label className="col">
            <span>Skill 仓库</span>
            <input
              type="text"
              value={settings.updates.github_repo}
              onChange={(event) => patch({ updates: { ...settings.updates, github_repo: event.target.value } })}
              placeholder="mislw/oasis-wiki"
            />
          </label>
          <div className="row">
            <span>{updateText}</span>
          </div>
          <div className="actions">
            <button onClick={checkUpdates} disabled={busy}>检查更新</button>
            {update?.update_available && update.latest_url && (
              <button onClick={installUpdate} disabled={busy}>自动更新</button>
            )}
            {update?.latest_url && (
              <button className="btn-secondary" onClick={() => openUrl(update.latest_url!)}>查看详情</button>
            )}
          </div>
        </section>
      )}

      {toast && <div className="toast">{toast}</div>}
      {busy && <div className="busy">处理中...</div>}
    </div>
  );
}

function mcpUrl(settings: Settings) {
  const path = settings.skill_runtime.mcp.sse_path.startsWith("/")
    ? settings.skill_runtime.mcp.sse_path
    : `/${settings.skill_runtime.mcp.sse_path}`;
  return `http://${settings.skill_runtime.mcp.host}:${settings.skill_runtime.mcp.port}${path}`;
}

function ballStateLabel(state: BallState) {
  switch (state) {
    case "idle":
      return "待机";
    case "error":
      return "异常";
    default:
      return "已隐藏";
  }
}

function mcpStateLabel(state: McpState) {
  switch (state) {
    case "connected":
      return "上次检查已连接";
    case "disconnected":
      return "上次检查未连接";
    case "disabled":
      return "未启用";
    default:
      return "未检查";
  }
}

function updateSummary(update: UpdateStatus | null) {
  if (update?.error) return update.error;
  if (update?.latest_version) return `最新 Release: v${update.latest_version}`;
  if (update?.latest_revision_date) return `最新提交: ${formatCommitDate(update.latest_revision_date)}`;
  if (update?.latest_revision) return `最新提交: ${update.latest_revision}`;
  return "尚未检查";
}

function formatCommitDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

function formatEpochSeconds(value: string) {
  const n = Number(value);
  if (!Number.isFinite(n) || n <= 0) return value;
  return new Date(n * 1000).toLocaleString();
}
