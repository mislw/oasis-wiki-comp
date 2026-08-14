import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

const source = await readFile(new URL("../src/windows/uiImageAnalyzer.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
  },
}).outputText;
const moduleUrl = `data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`;
const { assignParents } = await import(moduleUrl);

function node(id, category, bounds) {
  return {
    id,
    name: id,
    category,
    bounds,
    extraction: { mode: "composite", target_component_id: id, confidence: 1, reason: "test" },
    z_index: 0,
    node_kind: "composite",
    render_mode: "outline",
  };
}

test("assignParents follows fine-grained CityDefence ownership", () => {
  const result = assignParents([
    node("background.root", "background", { x: 0, y: 0, width: 1415, height: 794 }),
    node("body.main", "panel", { x: 18, y: 92, width: 1371, height: 681 }),
    node("panel.plan", "panel", { x: 228, y: 100, width: 529, height: 667 }),
    node("tabs.plan", "panel", { x: 250, y: 108, width: 300, height: 70 }),
    node("button.plan.1", "button", { x: 330, y: 118, width: 62, height: 50 }),
    node("text.plan.1", "text", { x: 349, y: 130, width: 20, height: 24 }),
    node("artwork.upgrade_path", "artwork", { x: 270, y: 185, width: 430, height: 440 }),
    node("stage.tower.1", "artwork", { x: 300, y: 500, width: 110, height: 140 }),
    node("badge.stage.1", "text", { x: 382, y: 610, width: 31, height: 28 }),
    node("panel.effects", "panel", { x: 770, y: 357, width: 616, height: 281 }),
    node("row.effect.1", "artwork", { x: 793, y: 408, width: 556, height: 48 }),
    node("icon.effect.1", "artwork", { x: 805, y: 418, width: 28, height: 28 }),
    node("text.effect.1", "text", { x: 850, y: 418, width: 120, height: 28 }),
  ]);
  const parents = Object.fromEntries(result.map((item) => [item.id, item.parent_id]));

  assert.equal(parents["panel.plan"], "body.main");
  assert.equal(parents["tabs.plan"], "panel.plan");
  assert.equal(parents["button.plan.1"], "tabs.plan");
  assert.equal(parents["text.plan.1"], "button.plan.1");
  assert.equal(parents["artwork.upgrade_path"], "panel.plan");
  assert.equal(parents["stage.tower.1"], "artwork.upgrade_path");
  assert.equal(parents["badge.stage.1"], "stage.tower.1");
  assert.equal(parents["row.effect.1"], "panel.effects");
  assert.equal(parents["icon.effect.1"], "row.effect.1");
  assert.equal(parents["text.effect.1"], "row.effect.1");
});

test("assignParents leaves top-level panels outside the bitmap background", () => {
  const result = assignParents([
    node("background.root", "background", { x: 0, y: 0, width: 1000, height: 700 }),
    node("window.main", "panel", { x: 10, y: 10, width: 980, height: 680 }),
  ]);
  assert.equal(result.find((item) => item.id === "window.main").parent_id, undefined);
});
const ownerSource = await readFile(new URL("../src/windows/uiHierarchy.ts", import.meta.url), "utf8");
const ownerCompiled = ts.transpileModule(ownerSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
  },
}).outputText;
const ownerModuleUrl = `data:text/javascript;base64,${Buffer.from(ownerCompiled).toString("base64")}`;
const { canOwnChildren } = await import(ownerModuleUrl);

test("composite controls can own manually added child items", () => {
  for (const category of ["button", "group", "layout", "grid", "row", "card", "container", "switcher", "artwork"]) {
    assert.equal(canOwnChildren({ category, node_kind: "composite", extraction: { mode: "composite" } }), true, category);
  }
});

test("native controls do not become accidental layout parents", () => {
  assert.equal(canOwnChildren({ category: "text", node_kind: "native", extraction: { mode: "native" } }), false);
});