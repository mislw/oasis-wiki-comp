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

test("selects Agent details inside the existing settings window", async () => {
  const {
    selectSettingsPageForAgent,
    settingsOverviewPage,
  } = await importTypeScript("../src/windows/settingsNavigation.ts");

  assert.deepEqual(selectSettingsPageForAgent("codex", ["codex", "workbuddy"]), {
    kind: "agent",
    agentId: "codex",
  });
  assert.throws(
    () => selectSettingsPageForAgent("unknown", ["codex", "workbuddy"]),
    /unknown Agent target/,
  );
  assert.deepEqual(settingsOverviewPage(), { kind: "overview" });
});

test("the packaged window policy has no Agent-specific native window", async () => {
  const capability = JSON.parse(
    await readFile(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"),
  );

  assert.deepEqual(
    capability.windows.filter((label) => label.startsWith("settings-")),
    [],
  );
});

test("Agent details invoke uses Tauri's camelCase command argument", async () => {
  const source = await readFile(
    new URL("../src/windows/Settings.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /invoke\("open_agent_settings", \{ targetId \}\)/);
  assert.doesNotMatch(source, /invoke\("open_agent_settings", \{ target_id:/);
});

test("Skill handoffs never launch a duplicate Companion process while it is running", async () => {
  const scripts = await Promise.all([
    readFile(
      new URL(
        "../src-tauri/resources/skill/scripts/cowart-ui/component-extractor/open_ui_workflow.py",
        import.meta.url,
      ),
      "utf8",
    ),
    readFile(
      new URL(
        "../src-tauri/resources/skill/scripts/cowart-ui/component-extractor/create_ui_workbench.py",
        import.meta.url,
      ),
      "utf8",
    ),
    readFile(
      new URL(
        "../src-tauri/resources/skill/scripts/cowart-ui/component-extractor/report_ui_workflow_progress.py",
        import.meta.url,
      ),
      "utf8",
    ),
  ]);

  for (const source of scripts) {
    assert.match(source, /dispatch_companion_handoff/);
    assert.doesNotMatch(
      source,
      /Popen\([\s\S]{0,300}--ui-(?:workflow|workbench)/,
    );
  }
});

test("the UI workflow title-bar close preserves its pre-created WebView", async () => {
  const source = await readFile(
    new URL("../src-tauri/src/tray.rs", import.meta.url),
    "utf8",
  );

  assert.match(source, /WindowEvent::CloseRequested/);
  assert.match(source, /should_preserve_window_on_close/);
  assert.match(source, /api\.prevent_close\(\)/);
  assert.match(source, /"ui-workflow"/);
});
