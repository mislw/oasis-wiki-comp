import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const raw = JSON.parse(await readFile(new URL("../public/ui-workbench-city-defence/workbench-controls.json", import.meta.url), "utf8"));
const nodes = raw.controls;
const byId = new Map(nodes.map((node) => [node.component_id, node]));

test("CityDefence built-in tree mirrors the real layout containers", () => {
  const expectedParents = {
    "content.main": "body.main",
    "panel.plan": "content.main",
    "detail.column": "content.main",
    "panel.detail": "detail.column",
    "panel.effects": "detail.column",
    "panel.resources": "detail.column",
    "button.upgrade": "detail.column",
    "detail.header": "detail.info",
    "text.tower_name": "detail.header",
    "badge.tag.ranged": "detail.header",
    "detail.progress": "detail.info",
    "text.tower_level": "detail.progress",
    "text.tower_exp": "detail.progress",
    "progress.tower_exp": "detail.progress",
    "resources.list": "panel.resources",
    "resource.attack": "resources.list",
    "resource.range": "resources.list",
    "resource.gold": "resources.list",
    "plan.slot.1": "tabs.plan",
    "plan.state.1": "plan.slot.1",
    "button.plan.1": "plan.state.1",
    "text.plan.1": "button.plan.1",
    "plan.slot.2": "tabs.plan",
    "plan.state.2": "plan.slot.2",
    "button.plan.2": "plan.state.2",
    "text.plan.2": "button.plan.2",
    "plan.slot.3": "tabs.plan",
    "plan.state.3": "plan.slot.3",
    "button.plan.3": "plan.state.3",
    "text.plan.3": "button.plan.3",
    "button_layer.tab.defence_tower": "tab.defence_tower",
    "button_layer.tab.wall": "tab.wall",
    "button_layer.plan.1": "button.plan.1",
    "button_layer.plan.2": "button.plan.2",
    "button_layer.plan.3": "button.plan.3",
    "button_layer.manage_building": "button.manage_building",
    "button_layer.upgrade": "button.upgrade",
  };

  for (const [id, parent] of Object.entries(expectedParents)) {
    assert.ok(byId.has(id), `missing ${id}`);
    assert.equal(byId.get(id).parent_id, parent, `${id} parent`);
  }
});

test("CityDefence built-in tree has no missing parents", () => {
  for (const node of nodes) {
    assert.ok(!node.parent_id || node.parent_id === "root" || byId.has(node.parent_id), `${node.component_id} -> ${node.parent_id}`);
  }
});
test("CityDefence clickable controls expose a separate interaction layer", () => {
  const buttonLayers = nodes.filter((node) => node.component_id.startsWith("button_layer."));
  assert.equal(buttonLayers.length, 7);
  for (const node of buttonLayers) {
    assert.equal(node.node_kind, "interaction", node.component_id);
    assert.equal(node.category, "button_layer", node.component_id);
    assert.equal(node.render_mode, "outline", node.component_id);
    assert.match(node.extraction.target_component_id, /^Button_/);
  }
});