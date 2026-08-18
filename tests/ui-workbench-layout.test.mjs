import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

async function importTypeScript(relativePath) {
  const source = await readFile(new URL(relativePath, import.meta.url), "utf8");
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
    },
  }).outputText;
  const moduleUrl = `data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`;
  return import(moduleUrl);
}

const tree = {
  artifact_type: "ui_tree",
  schema_version: 1,
  status: "candidate",
  page_size: { width: 1415, height: 794 },
  nodes: [{
    id: "button.confirm",
    category: "button",
    parent_id: "panel.main",
    bounds: { x: 820, y: 665, width: 220, height: 72 },
    extraction: { mode: "native", target_component_id: "button.confirm" },
    node_kind: "interaction",
    render_mode: "outline",
    visible: true,
    locked: false,
    opacity: 1,
    z_index: 12,
    display_text: "确认",
    visual_assets: { source_crop: null, assembly_preview: "layers/button.png" },
  }],
};

test("layout projection preserves the complete editable node contract", async () => {
  const { workbenchLayoutProjection } = await importTypeScript("../src/windows/uiWorkbenchLayout.ts");

  assert.deepEqual(workbenchLayoutProjection(tree), {
    page_size: { height: 794, width: 1415 },
    nodes: [{
      bounds: { height: 72, width: 220, x: 820, y: 665 },
      category: "button",
      display_text: "确认",
      extraction: { mode: "native", target_component_id: "button.confirm" },
      id: "button.confirm",
      locked: false,
      node_kind: "interaction",
      opacity: 1,
      parent_id: "panel.main",
      render_mode: "outline",
      visible: true,
      visual_assets: { assembly_preview: "layers/button.png", source_crop: null },
      z_index: 12,
    }],
  });
});

test("layout fingerprint is stable for object key order and changes for layout edits", async () => {
  const { workbenchLayoutFingerprint } = await importTypeScript("../src/windows/uiWorkbenchLayout.ts");
  const reordered = {
    nodes: [{ ...tree.nodes[0], bounds: { height: 72, y: 665, width: 220, x: 820 } }],
    page_size: { height: 794, width: 1415 },
  };
  const moved = structuredClone(tree);
  moved.nodes[0].bounds.x = 900;

  assert.equal(workbenchLayoutFingerprint(reordered), workbenchLayoutFingerprint(tree));
  assert.notEqual(workbenchLayoutFingerprint(moved), workbenchLayoutFingerprint(tree));
});

test("save state stays dirty when the tree changes while an older save is in flight", async () => {
  const { workbenchLayoutFingerprint, workbenchLayoutSaveState } = await importTypeScript("../src/windows/uiWorkbenchLayout.ts");
  const submitted = workbenchLayoutFingerprint(tree);
  const edited = structuredClone(tree);
  edited.nodes[0].bounds.x += 20;
  const current = workbenchLayoutFingerprint(edited);

  assert.deepEqual(workbenchLayoutSaveState(submitted, { revision: 3, fingerprint: submitted }), {
    dirty: false,
    label: "已保存 v3",
    revision: 3,
  });
  assert.deepEqual(workbenchLayoutSaveState(current, { revision: 3, fingerprint: submitted }), {
    dirty: true,
    label: "未保存",
    revision: 3,
  });
  assert.deepEqual(workbenchLayoutSaveState(current, null), {
    dirty: true,
    label: "未保存",
    revision: null,
  });
});
