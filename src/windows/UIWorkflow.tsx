import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  selectedWorkflowTask,
  workflowStageRows,
  type DeliveryPreflightEvidence,
  type DeliveryPreflightState,
  type UiWorkflowStore,
  type WidgetBlueprintCandidate,
  type WidgetBlueprintSearchResult,
  type WorkflowStageId,
} from "./uiWorkflowModel";
import "./UIWorkflow.css";

const EMPTY_STORE: UiWorkflowStore = { selected_task_id: null, tasks: [] };

type SourceMode = "continue" | "generate" | "import";

type UiSourceDispatchResult = {
  new_task_url: string;
};

type CodexPromptSubmissionResult = {
  submitted: boolean;
  message: string;
};

export default function UIWorkflow() {
  const [store, setStore] = useState<UiWorkflowStore>(EMPTY_STORE);
  const [activeStage, setActiveStage] = useState<WorkflowStageId>("source");
  const [sourceMode, setSourceMode] = useState<SourceMode>("continue");
  const [workspace, setWorkspace] = useState("");
  const [targetQuery, setTargetQuery] = useState("");
  const [candidates, setCandidates] = useState<WidgetBlueprintCandidate[]>([]);
  const [selectedLoadPath, setSelectedLoadPath] = useState("");
  const [preflight, setPreflight] = useState<DeliveryPreflightEvidence | null>(null);
  const [preflightState, setPreflightState] = useState<DeliveryPreflightState>("idle");
  const [notice, setNotice] = useState("正在读取 UI 任务");
  const [busy, setBusy] = useState(false);
  const task = useMemo(() => selectedWorkflowTask(store), [store]);
  const stages = useMemo(() => task ? workflowStageRows(task.stages) : [], [task]);

  useEffect(() => {
    invoke<UiWorkflowStore>("list_ui_workflow_tasks")
      .then((next) => {
        setStore(next);
        setNotice(next.tasks.length ? "已恢复最近的 UI 任务" : "请先从目标 Agent 对话启动 oasis-wiki UI 流程");
      })
      .catch((error) => setNotice(`UI 任务读取失败: ${error}`));
    const unlisten = listen<UiWorkflowStore>("ui-workflow://progress", (event) => {
      setStore(event.payload);
      setNotice(selectedWorkflowTask(event.payload)?.latest_message ?? "进度已同步");
    });
    return () => { unlisten.then((stop) => stop()); };
  }, []);

  useEffect(() => {
    setSourceMode("continue");
    setWorkspace(task?.target.project_workspace || task?.agent_context?.workspace || "");
    setTargetQuery(task?.target.widget_blueprint_name || "");
    const evidence = task?.target.preflight?.status === "ready"
      && task.target.preflight.selected_load_path === task.target.widget_blueprint
      ? task.target.preflight
      : null;
    setPreflight(evidence);
    setPreflightState(evidence ? "ready" : "idle");
    setSelectedLoadPath(evidence?.selected_load_path || "");
    setCandidates(evidence ? [{
      display_name: task?.target.widget_blueprint_name || evidence.selected_load_path,
      load_path: evidence.selected_load_path,
      class_name: task?.target.widget_blueprint_class as WidgetBlueprintCandidate["class_name"],
    }] : []);
  }, [task?.task_id]);

  async function selectTask(taskId: string) {
    try {
      setStore(await invoke<UiWorkflowStore>("select_ui_workflow_task", { taskId }));
    } catch (error) {
      setNotice(`切换任务失败: ${error}`);
    }
  }

  async function searchTargets() {
    if (!task) return;
    if (!workspace.trim() || !targetQuery.trim()) {
      setPreflightState("blocked");
      setNotice("请填写项目工作区和至少两个字符的搜索内容");
      return;
    }
    setBusy(true);
    setPreflight(null);
    setSelectedLoadPath("");
    setCandidates([]);
    setPreflightState("searching_assets");
    try {
      const result = await invoke<WidgetBlueprintSearchResult>("search_widget_blueprints", {
        taskId: task.task_id,
        projectWorkspace: workspace,
        query: targetQuery,
      });
      setCandidates(result.candidates);
      setPreflightState(result.candidates.length ? result.state : "blocked");
      setNotice(result.candidates.length ? result.message : "编辑器没有返回匹配的 WidgetBlueprint");
    } catch (error) {
      setPreflightState("blocked");
      setNotice(`搜索失败: ${error}`);
    } finally {
      setBusy(false);
    }
  }

  async function selectTarget(candidate: WidgetBlueprintCandidate) {
    if (!task) return;
    setBusy(true);
    setSelectedLoadPath(candidate.load_path);
    setPreflight(null);
    setPreflightState("checking_mcp");
    setNotice("正在确认编辑器项目、资产类型和精确 load_path");
    try {
      const next = await invoke<UiWorkflowStore>("preflight_ui_delivery", {
        taskId: task.task_id,
        projectWorkspace: workspace,
        selectedLoadPath: candidate.load_path,
      });
      setStore(next);
      const updated = next.tasks.find((item) => item.task_id === task.task_id);
      const evidence = updated?.target.preflight ?? null;
      setPreflight(evidence);
      setPreflightState(evidence?.status === "ready" ? "ready" : "blocked");
      setNotice(evidence?.message || "编辑器预检没有返回可交付证据");
    } catch (error) {
      setPreflightState("blocked");
      setNotice(`预检失败: ${error}`);
    } finally {
      setBusy(false);
    }
  }

  async function startOrResume() {
    if (sourceMode !== "continue") {
      if (!workspace.trim()) {
        setNotice("请先填写新 UI 使用的项目工作区");
        return;
      }
      setBusy(true);
      setNotice(sourceMode === "generate" ? "正在新建 UI 生成任务" : "正在新建 UI 导入任务");
      try {
        const dispatch = await invoke<UiSourceDispatchResult>("start_ui_source_task", {
          projectWorkspace: workspace,
          sourceMode,
        });
        await openUrl(dispatch.new_task_url);
        const submission = await invoke<CodexPromptSubmissionResult>("submit_codex_ui_source_prompt");
        setNotice(submission.message);
      } catch (error) {
        setNotice(`新建 Codex 任务失败: ${error}`);
      } finally {
        setBusy(false);
      }
      return;
    }
    if (!task?.agent_context?.thread_id) {
      setNotice("当前没有来源任务，请先在 Codex 中调用 oasis-wiki 创建 UI");
      return;
    }
    if (activeStage === "workbench" || activeStage === "layering") {
      await invoke("open_ui_workbench");
      return;
    }
    setNotice(`已切换到 ${stages.find((stage) => stage.id === activeStage)?.label ?? "当前"} 阶段`);
  }

  return (
    <main className="ui-workflow-window">
      <header className="workflow-header">
        <div>
          <small>OASIS WIKI · {task?.agent_context?.provider?.toUpperCase() ?? "LOCAL"}</small>
          <h1>UI 生图工具链</h1>
          <p>统一管理视觉、控件、Workbench 与编辑器交付进度。</p>
        </div>
        <div className="workflow-header-actions">
          {store.tasks.length > 1 && (
            <select value={task?.task_id ?? ""} onChange={(event) => void selectTask(event.target.value)}>
              {store.tasks.map((item) => <option key={item.task_id} value={item.task_id}>{item.title}</option>)}
            </select>
          )}
          <button type="button" className="icon-button" onClick={() => getCurrentWebviewWindow().hide()} title="隐藏窗口">×</button>
        </div>
      </header>

      <nav className="workflow-stepper" aria-label="UI 工具链阶段">
        {stages.length ? stages.map((stage) => (
          <button
            key={stage.id}
            type="button"
            className={`workflow-step status-${stage.status} ${activeStage === stage.id ? "active" : ""}`}
            onClick={() => setActiveStage(stage.id)}
          >
            <span>{stage.index}</span>
            <strong>{stage.label}</strong>
            <small>{stage.statusLabel}</small>
          </button>
        )) : ["来源", "UI Tree", "视觉稿", "分层", "Workbench", "UMG", "逻辑", "验收"].map((label, index) => (
          <button key={label} type="button" className="workflow-step" disabled>
            <span>{index + 1}</span><strong>{label}</strong><small>未开始</small>
          </button>
        ))}
      </nav>

      <section className="workflow-content">
        {!task ? (
          <div className="workflow-empty">
            <h2>还没有可恢复的 UI 任务</h2>
            <p>从 Codex 任务调用 oasis-wiki 创建 UI 后，这里会记录来源任务；编辑器交付会创建新的 Codex 实现任务。</p>
          </div>
        ) : (
          <>
            <div className="workflow-section-heading">
              <div>
                <h2>{stages.find((stage) => stage.id === activeStage)?.label}</h2>
                <p>{task.title} · {task.control_count} 个控件</p>
              </div>
              <span className={`workflow-status-badge status-${task.stages[activeStage].status}`}>
                {stages.find((stage) => stage.id === activeStage)?.statusLabel}
              </span>
            </div>

            {activeStage === "source" && (
              <div className="workflow-source-grid">
                <button type="button" className={`source-choice ${sourceMode === "continue" ? "selected" : ""}`} onClick={() => setSourceMode("continue")}><strong>继续现有任务</strong><small>使用当前页面并保留来源任务</small></button>
                <button type="button" className={`source-choice ${sourceMode === "generate" ? "selected" : ""}`} onClick={() => setSourceMode("generate")}><strong>生成新 UI</strong><small>新建 Codex 任务并进入生图流程</small></button>
                <button type="button" className={`source-choice ${sourceMode === "import" ? "selected" : ""}`} onClick={() => setSourceMode("import")}><strong>使用已有图</strong><small>新建 Codex 任务并进入图片导入</small></button>
              </div>
            )}

            <div className="workflow-form-grid">
              <label><span>页面名称</span><input value={sourceMode === "continue" ? task.title : "将在新 Codex 任务中确认"} readOnly /></label>
              <label><span>来源 Agent</span><input value={sourceMode === "continue" ? (task.agent_context ? `${task.agent_context.provider} · ${task.agent_context.thread_id}` : "未记录") : "Codex 新任务"} readOnly /></label>
              <label className="full"><span>项目工作区</span><input value={workspace} onChange={(event) => {
                setWorkspace(event.target.value);
                setCandidates([]);
                setSelectedLoadPath("");
                setPreflight(null);
                setPreflightState("idle");
              }} placeholder="E:\...\UGCProjects\RedCliff" /></label>
              {sourceMode === "continue" && <div className="workflow-target-search full">
                <label><span>目标 WidgetBlueprint</span><input value={targetQuery} onChange={(event) => {
                  setTargetQuery(event.target.value);
                  setCandidates([]);
                  setSelectedLoadPath("");
                  setPreflight(null);
                  setPreflightState("idle");
                }} placeholder="名称或 /RedCliff/Asset/UI/..." /></label>
                <button type="button" onClick={() => void searchTargets()} disabled={busy}>搜索编辑器</button>
              </div>}
              {sourceMode === "continue" && <div className="workflow-target-results full" role="listbox" aria-label="WidgetBlueprint 搜索结果">
                {candidates.length === 0 && <span>搜索后选择一个编辑器返回的 WidgetBlueprint</span>}
                {candidates.map((candidate) => (
                  <button
                    type="button"
                    role="option"
                    aria-selected={selectedLoadPath === candidate.load_path}
                    className={selectedLoadPath === candidate.load_path ? "selected" : ""}
                    key={candidate.load_path}
                    onClick={() => void selectTarget(candidate)}
                    disabled={busy}
                  >
                    <strong>{candidate.display_name}</strong>
                    <span>{candidate.load_path}</span>
                    <small>{candidate.class_name}</small>
                  </button>
                ))}
              </div>}
              {sourceMode === "continue" && <div className={`workflow-preflight full status-${preflightState}`}>
                <strong>{preflightState === "ready" ? "编辑器预检通过" : "编辑器预检"}</strong>
                <span>{preflight?.message || notice}</span>
                {preflight?.status === "ready" && <small>{preflight.selected_load_path} · {preflight.selected_class_name} · {preflight.mcp_server_name} {preflight.mcp_server_version}</small>}
              </div>}
            </div>

            <div className="workflow-progress-detail">
              <strong>{sourceMode === "continue" ? "当前进度" : "新任务启动方式"}</strong>
              <p>{sourceMode === "continue"
                ? (task.stages[activeStage].message || task.latest_message || "等待下一步操作")
                : (sourceMode === "generate"
                  ? "新任务将调用 oasis-wiki，从 SOURCE 开始读取项目风格并生成新的 UI。"
                  : "新任务将调用 oasis-wiki，从 SOURCE 开始接收已有图片并建立独立 UI 页面。")}</p>
              {sourceMode === "continue" && activeStage === "workbench" && <button type="button" onClick={() => invoke("open_ui_workbench")}>打开当前 UI 工作台</button>}
            </div>
          </>
        )}
      </section>

      <footer className="workflow-footer">
        <span>{notice}</span>
        <div>
          <button type="button" onClick={() => getCurrentWebviewWindow().hide()}>取消</button>
          <button
            type="button"
            className="primary"
            onClick={startOrResume}
            disabled={!task || busy || (sourceMode === "continue" ? !task.agent_context?.thread_id : !workspace.trim())}
          >
            {sourceMode === "continue" ? "切换并开始" : sourceMode === "generate" ? "新建任务并生成" : "新建任务并导入"}
          </button>
        </div>
      </footer>
    </main>
  );
}
