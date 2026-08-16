export type WorkflowStageId =
  | "source"
  | "ui_tree"
  | "visual"
  | "layering"
  | "workbench"
  | "umg"
  | "logic"
  | "review";

export type WorkflowStatus =
  | "not_started"
  | "in_progress"
  | "awaiting_confirmation"
  | "completed"
  | "blocked"
  | "stale";

export type WorkflowStageState = {
  status: WorkflowStatus;
  message?: string;
  updated_at_unix_ms?: number;
};

export type WorkflowStages = Record<WorkflowStageId, WorkflowStageState>;

export type DeliveryPreflightState =
  | "idle"
  | "checking_mcp"
  | "searching_assets"
  | "ready"
  | "blocked";

export type WidgetBlueprintCandidate = {
  display_name: string;
  load_path: string;
  class_name: "UGCWidgetBlueprint" | "WidgetBlueprint";
};

export type DeliveryPreflightEvidence = {
  status: DeliveryPreflightState;
  checked_at_unix_ms: number;
  mcp_server_name: string;
  mcp_server_version: string;
  editor_project_root: string;
  selected_load_path: string;
  selected_class_name: string;
  evidence_id: string;
  message: string;
};

export type WidgetBlueprintSearchResult = {
  state: DeliveryPreflightState;
  candidates: WidgetBlueprintCandidate[];
  message: string;
};

export type UiWorkflowTask = {
  task_id: string;
  page_id: string;
  title: string;
  control_count: number;
  agent_context: {
    provider: string;
    thread_id: string;
    session_id?: string;
    workspace?: string;
  } | null;
  target: {
    project_workspace?: string;
    widget_blueprint: string;
    widget_blueprint_name?: string;
    widget_blueprint_class?: string;
    preflight?: DeliveryPreflightEvidence | null;
  };
  stages: WorkflowStages;
  latest_message?: string;
  updated_at_unix_ms: number;
};

export type UiWorkflowStore = {
  selected_task_id: string | null;
  tasks: UiWorkflowTask[];
};

const STAGES: Array<{ id: WorkflowStageId; label: string }> = [
  { id: "source", label: "来源" },
  { id: "ui_tree", label: "UI Tree" },
  { id: "visual", label: "视觉稿" },
  { id: "layering", label: "分层" },
  { id: "workbench", label: "Workbench" },
  { id: "umg", label: "UMG" },
  { id: "logic", label: "逻辑" },
  { id: "review", label: "验收" },
];

const STATUS_LABELS: Record<WorkflowStatus, string> = {
  not_started: "未开始",
  in_progress: "进行中",
  awaiting_confirmation: "待确认",
  completed: "已完成",
  blocked: "阻塞",
  stale: "已过期",
};

export function workflowStageRows(stages: WorkflowStages) {
  return STAGES.map((stage, index) => ({
    ...stage,
    index: index + 1,
    status: stages[stage.id].status,
    statusLabel: STATUS_LABELS[stages[stage.id].status],
    message: stages[stage.id].message ?? "",
  }));
}

export function deliveryReadiness(task: Partial<UiWorkflowTask>): { ready: boolean; reason: string } {
  if (!task.agent_context?.thread_id || task.agent_context.provider !== "codex") {
    return { ready: false, reason: "当前 UI 没有可追溯的 Codex 来源任务" };
  }
  if (!task.target?.widget_blueprint?.trim()) {
    return { ready: false, reason: "请先从编辑器搜索并选择目标 WidgetBlueprint" };
  }
  const preflight = task.target.preflight;
  if (preflight?.status !== "ready") {
    return { ready: false, reason: preflight?.message || "目标 WidgetBlueprint 尚未通过编辑器只读预检" };
  }
  if (preflight.selected_load_path !== task.target.widget_blueprint) {
    return { ready: false, reason: "目标 WidgetBlueprint 已变化，请重新预检" };
  }
  if (!task.control_count) {
    return { ready: false, reason: "当前 UI 没有可交付控件" };
  }
  if (task.stages?.workbench?.status !== "completed") {
    return { ready: false, reason: "请先确认 Workbench 控件与文字样式" };
  }
  return { ready: true, reason: "" };
}

export function selectWidgetBlueprintCandidate(
  candidates: WidgetBlueprintCandidate[],
  loadPath: string,
): WidgetBlueprintCandidate | null {
  const matches = candidates.filter((candidate) => candidate.load_path === loadPath);
  return matches.length === 1 ? matches[0] : null;
}

export function selectedWorkflowTask(store: UiWorkflowStore): UiWorkflowTask | null {
  return store.tasks.find((task) => task.task_id === store.selected_task_id) ?? store.tasks[0] ?? null;
}
