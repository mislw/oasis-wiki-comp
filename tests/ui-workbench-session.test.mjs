import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import http from "node:http";
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

test("loads a loopback workbench session and resolves contained assets", async (t) => {
  const { loadWorkbenchSession, resolveSessionAssetUrl } = await importTypeScript("../src/windows/uiWorkbenchSession.ts");
  const server = http.createServer((request, response) => {
    if (request.url === "/review/session.json") {
      response.setHeader("Content-Type", "application/json");
      response.end(JSON.stringify({
        source_image: "source.png",
        nodes: [{
          id: "button.buy",
          visual_assets: { clean_layer: "layers/button.png" },
        }],
      }));
      return;
    }
    response.writeHead(404).end();
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  t.after(() => server.close());
  const { port } = server.address();

  const loaded = await loadWorkbenchSession(`http://127.0.0.1:${port}/review`);
  assert.equal(loaded.sourceImageUrl, `http://127.0.0.1:${port}/review/source.png`);
  assert.equal(
    resolveSessionAssetUrl(loaded.assetBaseUrl, "layers/button.png"),
    `http://127.0.0.1:${port}/review/layers/button.png`,
  );
  assert.equal(resolveSessionAssetUrl(loaded.assetBaseUrl, "data:image/png;base64,abc"), "data:image/png;base64,abc");
  assert.equal(resolveSessionAssetUrl(loaded.assetBaseUrl, "blob:https://example.test/id"), "blob:https://example.test/id");
  assert.equal(resolveSessionAssetUrl(loaded.assetBaseUrl, "https://cdn.example.test/button.png"), "https://cdn.example.test/button.png");
  assert.throws(() => resolveSessionAssetUrl(loaded.assetBaseUrl, "../outside.png"), /outside the workbench session/);
});

test("connects pending and forwarded sessions without replacing event data with fallback", async () => {
  const { connectWorkbenchSessions } = await importTypeScript("../src/windows/uiWorkbenchSession.ts");
  const loaded = [];
  let listener;
  let fallbackCount = 0;
  let unsubscribed = false;

  const unsubscribe = await connectWorkbenchSessions({
    getPendingUrl: async () => null,
    subscribe: async (handler) => {
      listener = handler;
      handler("http://127.0.0.1:50691/");
      return () => { unsubscribed = true; };
    },
    loadExternal: async (url) => { loaded.push(url); },
    loadFallback: async () => { fallbackCount += 1; },
  });

  assert.equal(typeof listener, "function");
  assert.deepEqual(loaded, ["http://127.0.0.1:50691/"]);
  assert.equal(fallbackCount, 0);
  unsubscribe();
  assert.equal(unsubscribed, true);
});

test("loads one persisted page as an isolated session with contained assets", async () => {
  const {
    collectPersistedAssetPaths,
    loadPersistedWorkbenchPage,
    preferredWorkbenchPage,
  } = await importTypeScript("../src/windows/uiWorkbenchSession.ts");
  const catalog = {
    selected_page_id: "currency",
    pages: [
      { page_id: "currency", title: "货币兑换", control_count: 2, available: true },
      { page_id: "gem-draw", title: "宝石抽奖", control_count: 1, available: true },
    ],
  };
  const reads = [];
  const loaded = await loadPersistedWorkbenchPage("currency", {
    loadPage: async () => ({
      page_id: "currency",
      title: "货币兑换",
      control_count: 3,
      source_image: "source.png",
      session: {
        source_image: "source.png",
        controls: [
          { component_id: "background.currency", visual_assets: { source_crop: "__source__" } },
          { component_id: "button.currency.buy", visual_assets: { clean_layer: "layers/buy.png" } },
          { component_id: "icon.currency.jade", visual_assets: { native_preview: "native/jade.png" } },
        ],
      },
    }),
    readAsset: async (_pageId, path) => {
      reads.push(path);
      return `data:image/png;base64,${Buffer.from(path).toString("base64")}`;
    },
  });

  assert.equal(preferredWorkbenchPage(catalog), "currency");
  assert.deepEqual(collectPersistedAssetPaths(loaded.raw), ["layers/buy.png", "native/jade.png"]);
  assert.deepEqual(reads, ["source.png"]);
  assert.equal(loaded.pageId, "currency");
  assert.match(loaded.sourceImageUrl, /^data:image\/png;base64,/);
  assert.deepEqual(loaded.raw.controls.map((node) => node.component_id), [
    "background.currency",
    "button.currency.buy",
    "icon.currency.jade",
  ]);
});

test("falls back to the first available page when the persisted selection is unavailable", async () => {
  const { preferredWorkbenchPage, workbenchPageNavRows } = await importTypeScript("../src/windows/uiWorkbenchSession.ts");
  assert.equal(preferredWorkbenchPage({
    selected_page_id: "missing",
    pages: [
      { page_id: "missing", available: false },
      { page_id: "castle", available: true },
    ],
  }), "castle");
  assert.equal(preferredWorkbenchPage({ selected_page_id: null, pages: [] }), null);
  assert.deepEqual(workbenchPageNavRows({
    selected_page_id: "castle",
    pages: [
      { page_id: "currency", title: "货币兑换", control_count: 63, available: true },
      { page_id: "castle", title: "城防", control_count: 41, available: true },
    ],
  }), [
    { pageId: "currency", title: "货币兑换", controlCountLabel: "63 控件", selected: false, available: true, thumbnailUrl: null },
    { pageId: "castle", title: "城防", controlCountLabel: "41 控件", selected: true, available: true, thumbnailUrl: null },
  ]);
});

test("loads available persisted assets without failing the whole page when one is missing", async () => {
  const { loadPersistedAssetUrls } = await importTypeScript("../src/windows/uiWorkbenchSession.ts");
  const result = await loadPersistedAssetUrls(
    "currency",
    {
      controls: [{
        component_id: "button.buy",
        visual_assets: {
          clean_layer: "layers/buy.png",
          assembly_preview: "preview/missing.png",
          native_preview: "native/jade.png",
        },
      }],
    },
    async (_pageId, path) => {
      if (path === "preview/missing.png") throw new Error("missing");
      return `data:image/png;base64,${Buffer.from(path).toString("base64")}`;
    },
  );

  assert.deepEqual([...result.keys()], ["layers/buy.png", "native/jade.png"]);
});

test("prefers node display text and keeps legacy currency sessions readable", async () => {
  const {
    nativeWorkbenchCloseTextStyle,
    nativeWorkbenchDisplayText,
  } = await importTypeScript("../src/windows/uiWorkbenchSession.ts");

  assert.equal(nativeWorkbenchDisplayText({
    id: "text.currency.title",
    category: "text",
    display_text: "自定义兑换标题",
    extraction: { target_component_id: "text.currency.title" },
  }), "自定义兑换标题");
  assert.equal(nativeWorkbenchDisplayText({
    id: "text.currency.name_element_02",
    category: "text",
    extraction: { target_component_id: "text.currency.name_element_02" },
  }), "一袋元素");
  assert.equal(nativeWorkbenchDisplayText({
    id: "counter.currency.amount_jade_03",
    category: "counter",
    extraction: { target_component_id: "counter.currency.amount_jade_03" },
  }), "980");
  assert.equal(nativeWorkbenchDisplayText({
    id: "text.unknown",
    category: "text",
    extraction: { target_component_id: "text.unknown" },
  }), "");
  assert.equal(nativeWorkbenchDisplayText({
    id: "counter.unknown",
    category: "counter",
    extraction: { target_component_id: "counter.unknown" },
  }), "0");
  const closeButton = {
    id: "button.currency.close",
    category: "hit_target",
    extraction: { target_component_id: "button.currency.close" },
  };
  assert.equal(nativeWorkbenchDisplayText(closeButton), "×");
  assert.deepEqual(nativeWorkbenchCloseTextStyle(closeButton), {
    font_size: 30,
    color: "#fff3cf",
    outline_color: "#6b3515",
    outline_size: 2,
    horizontal_alignment: "center",
    vertical_alignment: "middle",
  });
});

test("renders native preview assets without promoting them to reusable bitmaps", async () => {
  const source = await readFile(new URL("../src/windows/UIWorkbench.tsx", import.meta.url), "utf8");
  const css = await readFile(new URL("../src/windows/UIWorkbench.css", import.meta.url), "utf8");

  assert.match(source, /function nativePreviewPath\(node: UINode\)/);
  assert.match(source, /node\.node_kind === "native" \? visualUrl\(node, "clean"\) : null/);
  assert.match(source, /className="native-preview-layer"/);
  assert.match(source, /!node\.reusable_bitmap/);
  assert.match(css, /\.native-preview-layer \{[^}]*object-fit: contain/);
});
test("keeps UMG-like text settings in design space so canvas zoom cannot reflow text", async () => {
  const { nativeWorkbenchTextCss } = await importTypeScript("../src/windows/uiWorkbenchSession.ts");

  assert.deepEqual(nativeWorkbenchTextCss({
    font_size: 24,
    color: "#f8e8c0",
    outline_color: "#6b3518",
    outline_size: 2,
    shadow_color: "#180c06",
    shadow_offset_x: 3,
    shadow_offset_y: 4,
    horizontal_alignment: "right",
    vertical_alignment: "bottom",
    auto_wrap: true,
  }, 18), {
    fontSize: "24px",
    color: "#f8e8c0",
    textShadow: "-2px -2px 0 #6b3518, 0 -2px 0 #6b3518, 2px -2px 0 #6b3518, -2px 0 0 #6b3518, 2px 0 0 #6b3518, -2px 2px 0 #6b3518, 0 2px 0 #6b3518, 2px 2px 0 #6b3518, 3px 4px 2px #180c06",
    justifyContent: "flex-end",
    alignItems: "flex-end",
    textAlign: "right",
    whiteSpace: "normal",
    overflowWrap: "anywhere",
  });
  assert.deepEqual(nativeWorkbenchTextCss(undefined, 18), { fontSize: "18px" });
  assert.equal(nativeWorkbenchTextCss({ outline_size: 0 }, 18).textShadow, "none");
});

test("scales the complete canvas instead of relaying out each node at every zoom", async () => {
  const source = await readFile(new URL("../src/windows/UIWorkbench.tsx", import.meta.url), "utf8");

  assert.match(source, /className="canvas-stage-frame"/);
  assert.match(source, /transform:\s*`scale\(\$\{scale\}\)`/);
  assert.match(source, /left:\s*node\.bounds\.x,/);
  assert.match(source, /width:\s*node\.bounds\.width,/);
  assert.doesNotMatch(source, /left:\s*node\.bounds\.x\s*\*\s*scale/);
  assert.doesNotMatch(source, /nativeWorkbenchTextCss\(node\.text_style,\s*nativeFontSize\(node,\s*scale\),\s*scale\)/);
});

test("resolves selected layer ancestors without looping on malformed trees", async () => {
  const { workbenchLayerAncestorIds } = await importTypeScript("../src/windows/uiWorkbenchSession.ts");

  assert.equal(typeof workbenchLayerAncestorIds, "function");
  const nodes = [
    { id: "root.panel" },
    { id: "panel.currency", parent_id: "root.panel" },
    { id: "button.buy", parent_id: "panel.currency" },
    { id: "cycle.a", parent_id: "cycle.b" },
    { id: "cycle.b", parent_id: "cycle.a" },
  ];
  assert.deepEqual(workbenchLayerAncestorIds(nodes, "button.buy"), ["panel.currency", "root.panel"]);
  assert.deepEqual(workbenchLayerAncestorIds(nodes, "missing"), []);
  assert.deepEqual(workbenchLayerAncestorIds(nodes, "cycle.a"), ["cycle.b"]);
});

test("canvas selection reveals and centers the matching hierarchy row", async () => {
  const source = await readFile(new URL("../src/windows/UIWorkbench.tsx", import.meta.url), "utf8");

  assert.match(source, /function selectCanvasNode\(id: string\)[\s\S]*?setQuery\(""\);[\s\S]*?setSelectedId\(id\);[\s\S]*?setTreeRevealRequest/);
  assert.match(source, /selectCanvasNode\(node\.id\)/);
  assert.match(source, /ref=\{treeListRef\}/);
  assert.match(source, /data-tree-node-id=\{node\.id\}/);
  assert.match(source, /workbenchLayerAncestorIds\(tree\.nodes,\s*treeRevealRequest\.id\)/);
  assert.match(source, /scrollIntoView\(\{\s*block: "center",\s*inline: "nearest",?\s*}\)/s);
});

test("workbench confirms the current page and starts a new Codex task", async () => {
  const source = await readFile(new URL("../src/windows/UIWorkbench.tsx", import.meta.url), "utf8");
  assert.match(source, /confirm_and_deliver_ui/);
  assert.match(source, /search_widget_blueprints/);
  assert.match(source, /preflight_ui_delivery/);
  assert.match(source, /evidenceId/);
  assert.doesNotMatch(source, /\|\|\s*"\/Game\/UI\/"/);
  assert.match(source, /submit_codex_new_task_prompt/);
  assert.match(source, /确认并交付到编辑器/);
  assert.match(source, /openUrl\(result\.new_task_url\)/);
  assert.match(source, /来源任务/);
  assert.match(source, /新建 Codex 任务/);
  assert.match(source, /确认并在新任务中实现/);
  assert.doesNotMatch(source, /process_id/);
  assert.match(source, /ui-workflow:\/\/progress/);
  assert.match(source, /workflowStageRows/);
  assert.match(source, /workbench-workflow-strip/);
  assert.match(source, /role="dialog"/);
  assert.doesNotMatch(source, /window\.prompt/);
  assert.doesNotMatch(source, /window\.confirm/);
});

test("ui workbench may open only the Codex new-task deep link", async () => {
  const capability = JSON.parse(
    await readFile(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"),
  );
  const openerPermission = capability.permissions.find(
    (permission) => typeof permission === "object"
      && permission.identifier === "opener:allow-open-url",
  );

  assert.deepEqual(openerPermission, {
    identifier: "opener:allow-open-url",
    allow: [{ url: "codex://threads/new*" }],
  });
});
