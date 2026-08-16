import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

async function importTypeScript(relativePath) {
  const source = await readFile(new URL(relativePath, import.meta.url), "utf8");
  const compiled = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`);
}

test("presents the eight workflow stages in production order", async () => {
  const { workflowStageRows } = await importTypeScript("../src/windows/uiWorkflowModel.ts");
  assert.deepEqual(workflowStageRows({
    source: { status: "completed" },
    ui_tree: { status: "completed" },
    visual: { status: "completed" },
    layering: { status: "completed" },
    workbench: { status: "awaiting_confirmation" },
    umg: { status: "not_started" },
    logic: { status: "not_started" },
    review: { status: "not_started" },
  }).map((row) => [row.id, row.label, row.statusLabel]), [
    ["source", "来源", "已完成"],
    ["ui_tree", "UI Tree", "已完成"],
    ["visual", "视觉稿", "已完成"],
    ["layering", "分层", "已完成"],
    ["workbench", "Workbench", "待确认"],
    ["umg", "UMG", "未开始"],
    ["logic", "逻辑", "未开始"],
    ["review", "验收", "未开始"],
  ]);
});

test("delivery becomes ready only for a confirmed page with matching editor evidence", async () => {
  const { deliveryReadiness } = await importTypeScript("../src/windows/uiWorkflowModel.ts");
  const loadPath = "/RedCliff/Asset/UI/CurrencyExchange.CurrencyExchange";
  const task = {
    control_count: 63,
    agent_context: { provider: "codex", thread_id: "thread-1" },
    target: {
      project_workspace: "E:\\UGCProjects\\RedCliff",
      widget_blueprint: loadPath,
      widget_blueprint_name: "CurrencyExchange",
      widget_blueprint_class: "UGCWidgetBlueprint",
      preflight: {
        status: "ready",
        checked_at_unix_ms: 123,
        mcp_server_name: "UGCAskQ",
        mcp_server_version: "1.0.0",
        editor_project_root: "/RedCliff",
        selected_load_path: loadPath,
        selected_class_name: "UGCWidgetBlueprint",
        evidence_id: "sha256:ready",
        message: "目标 WidgetBlueprint 已通过编辑器只读预检",
      },
    },
    stages: { workbench: { status: "completed" } },
  };
  assert.deepEqual(deliveryReadiness(task), { ready: true, reason: "" });
  assert.equal(deliveryReadiness({
    ...task,
    target: { ...task.target, widget_blueprint: "" },
  }).ready, false);
  assert.equal(deliveryReadiness({
    ...task,
    target: {
      ...task.target,
      widget_blueprint: "/RedCliff/Asset/UI/Other.Other",
    },
  }).ready, false);
  assert.equal(deliveryReadiness({
    ...task,
    target: { ...task.target, preflight: { ...task.target.preflight, status: "blocked" } },
  }).ready, false);
  assert.equal(deliveryReadiness({ ...task, agent_context: null }).ready, false);
  assert.equal(deliveryReadiness({
    ...task,
    stages: { workbench: { status: "stale" } },
  }).ready, false);
});

test("candidate selection keeps exactly one editor-returned WidgetBlueprint", async () => {
  const { selectWidgetBlueprintCandidate } = await importTypeScript("../src/windows/uiWorkflowModel.ts");
  const candidates = [
    {
      display_name: "CurrencyExchange",
      load_path: "/RedCliff/Asset/UI/CurrencyExchange.CurrencyExchange",
      class_name: "UGCWidgetBlueprint",
    },
    {
      display_name: "CityDefence",
      load_path: "/RedCliff/Asset/UI/CityDefence.CityDefence",
      class_name: "WidgetBlueprint",
    },
  ];

  assert.deepEqual(
    selectWidgetBlueprintCandidate(candidates, candidates[1].load_path),
    candidates[1],
  );
  assert.equal(selectWidgetBlueprintCandidate(candidates, "/RedCliff/Asset/UI/Missing.Missing"), null);
});

test("native workflow window consumes the persistent store and progress event", async () => {
  const source = await readFile(new URL("../src/windows/UIWorkflow.tsx", import.meta.url), "utf8");
  assert.match(source, /list_ui_workflow_tasks/);
  assert.match(source, /ui-workflow:\/\/progress/);
  assert.match(source, /search_widget_blueprints/);
  assert.match(source, /preflight_ui_delivery/);
  assert.doesNotMatch(source, /placeholder="\/Game\/UI\/\.\.\."/);
  assert.match(source, /切换并开始/);
});

test("source choices launch new Codex tasks instead of staying disabled", async () => {
  const source = await readFile(new URL("../src/windows/UIWorkflow.tsx", import.meta.url), "utf8");

  assert.match(source, /sourceMode/);
  assert.match(source, /start_ui_source_task/);
  assert.match(source, /submit_codex_ui_source_prompt/);
  assert.match(source, /openUrl\(dispatch\.new_task_url\)/);
  assert.doesNotMatch(
    source,
    /source-choice"\s+disabled><strong>生成新 UI/,
  );
  assert.doesNotMatch(
    source,
    /source-choice"\s+disabled><strong>使用已有图/,
  );
  assert.match(source, /新建任务并生成/);
  assert.match(source, /新建任务并导入/);
});

test("workflow window has permission to subscribe to native progress events", async () => {
  const source = await readFile(
    new URL("../src-tauri/capabilities/default.json", import.meta.url),
    "utf8",
  );
  const capability = JSON.parse(source);
  assert.ok(capability.windows.includes("ui-workflow"));
  assert.ok(capability.permissions.includes("core:event:default"));
});
