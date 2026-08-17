import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { openUrl } from "@tauri-apps/plugin-opener";
import { analyzeUIImage } from "./uiImageAnalyzer";
import { canOwnChildren } from "./uiHierarchy";
import {
  connectWorkbenchSessions,
  loadPersistedWorkbenchPage,
  loadPersistedAssetUrls,
  loadWorkbenchSessionWithRetry,
  nativeWorkbenchCloseTextStyle,
  nativeWorkbenchDisplayText,
  nativeWorkbenchTextCss,
  preferredWorkbenchPage,
  resolveSessionAssetUrl,
  workbenchLayerAncestorIds,
  workbenchPageNavRows,
  type WorkbenchCatalog,
  type WorkbenchTextStyle,
} from "./uiWorkbenchSession";
import {
  workflowStageRows,
  type DeliveryPreflightEvidence,
  type DeliveryPreflightState,
  type UiWorkflowStore,
  type UiWorkflowTask,
  type WidgetBlueprintCandidate,
  type WidgetBlueprintSearchResult,
} from "./uiWorkflowModel";
import "./UIWorkbench.css";

type ExtractionMode = "native" | "extract_artwork" | "reconstruct_skin" | "composite";
type NodeKind = "composite" | "skin" | "artwork" | "native" | "interaction";
type RenderMode = "bitmap" | "outline" | "ghost" | "assembly" | "hidden";
type VisualMode = "source" | "clean" | "assembly";
type GalleryTab = "assets" | "structure";
type GalleryFilter = "all" | NodeKind | "needs_cleanup" | "ready";
type MoveScope = "layer" | "group";
type Bounds = { x: number; y: number; width: number; height: number };
type Extraction = {
  mode: ExtractionMode;
  target_component_id: string;
  confidence?: number;
  reason?: string;
  remove_content?: string[];
  transparent?: boolean;
  evaluate_nine_slice?: boolean;
};
type UINode = {
  id: string;
  name?: string;
  display_text?: string;
  content_hint?: string;
  text_style?: WorkbenchTextStyle;
  category: string;
  parent_id?: string;
  bounds: Bounds;
  source_bounds?: Bounds;
  extraction: Extraction;
  visible?: boolean;
  locked?: boolean;
  opacity?: number;
  z_index?: number;
  node_kind?: NodeKind;
  render_mode?: RenderMode;
  visual_assets?: { source_crop: string | null; clean_layer?: string | null; clean_asset?: string | null; assembly_preview: string | null; native_preview?: string | null };
  interaction?: { role: "button"; target_widget: string };
  review?: { status: "candidate" | "pending_review"; cleanup_status: "not_applicable" | "needs_cleanup" | "requested" | "in_progress" | "clean" | "ready" | "failed" };
  reusable_bitmap?: boolean;
  reuse_of?: string;
  texture_asset?: string;
  component_reuse?: { component_id: string; source_asset: string; asset_id?: string | null; state: string; status: "ready" };
  derived_from?: string;
};
type UITree = {
  artifact_type: "ui_tree";
  schema_version: number;
  status: string;
  page_size: { width: number; height: number };
  nodes: UINode[];
};
type Interaction = {
  nodeId: string;
  kind: "drag" | "resize";
  handle?: string;
  startX: number;
  startY: number;
  affectedIds: string[];
  originals: Record<string, Bounds>;
};
type PanInteraction = { startX: number; startY: number; panX: number; panY: number };
type LayerRow = { node: UINode; depth: number; hasChildren: boolean };
type AssetUrls = Record<string, Partial<Record<VisualMode, string>>>;
type DeliveryDispatchResult = {
  delivery_id: string;
  request_path: string;
  new_task_url: string;
};
type CodexPromptSubmissionResult = {
  submitted: boolean;
  message: string;
};
type DeliveryDialogState = {
  task: UiWorkflowTask;
  projectWorkspace: string;
  query: string;
  candidates: WidgetBlueprintCandidate[];
  selectedLoadPath: string;
  preflight: DeliveryPreflightEvidence | null;
  state: DeliveryPreflightState;
  message: string;
};
type TreeRevealRequest = { id: string; sequence: number };

const DEMO_IMAGE = "/ui-workbench-demo.png";
const CITY_DEFENCE_SESSION_BASE = "/ui-workbench-city-defence";
const CITY_DEFENCE_IMAGE = `${CITY_DEFENCE_SESSION_BASE}/visual-final.png`;
const CITY_DEFENCE_CONTROLS = `${CITY_DEFENCE_SESSION_BASE}/workbench-controls.json`;
const CITY_DEFENCE_PAGE_SIZE = { width: 1415, height: 794 };
const MODE_LABELS: Record<ExtractionMode, string> = {
  native: "原生控件",
  extract_artwork: "直接切图",
  reconstruct_skin: "重建皮肤",
  composite: "组合控件",
};
const NODE_KIND_LABELS: Record<NodeKind, string> = { composite: "Composite", skin: "Skin", artwork: "Artwork", native: "Native", interaction: "Interaction" };
const RENDER_MODE_LABELS: Record<RenderMode, string> = { bitmap: "Bitmap", outline: "Outline", ghost: "Ghost", assembly: "Assembly", hidden: "Hidden" };
const GALLERY_FILTER_LABELS: Record<GalleryFilter, string> = {
  all: "全部",
  skin: "Skin",
  artwork: "Artwork",
  composite: "Composite",
  native: "Native",
  interaction: "Interaction",
  needs_cleanup: "待净化",
  ready: "Ready",
};
const DEFAULT_TREE: UITree = {
  artifact_type: "ui_tree",
  schema_version: 1,
  status: "candidate",
  page_size: { width: 1228, height: 690 },
  nodes: [
    { id: "header.title", category: "text", bounds: { x: 394, y: 18, width: 440, height: 65 }, extraction: { mode: "native", target_component_id: "text.header.title", confidence: 1, reason: "Page title remains native." } },
    { id: "button.close", category: "button", bounds: { x: 1130, y: 12, width: 76, height: 76 }, extraction: { mode: "reconstruct_skin", target_component_id: "button.close.cream", transparent: true, confidence: 0.98, reason: "Reusable close-button artwork." } },
    { id: "panel.main", category: "panel", bounds: { x: 22, y: 118, width: 957, height: 548 }, extraction: { mode: "reconstruct_skin", target_component_id: "panel.parchment.large", remove_content: ["all_text", "pool_cards", "buttons"], transparent: true, evaluate_nine_slice: true, confidence: 0.95, reason: "Reusable parchment panel skin." } },
    { id: "pool.cards", category: "artwork", parent_id: "panel.main", bounds: { x: 244, y: 175, width: 512, height: 330 }, extraction: { mode: "composite", target_component_id: "composite.gem.pool.cards", confidence: 0.94, reason: "Card fan is assembled from reusable card and gem artwork." } },
    { id: "button.draw.single", category: "button", parent_id: "panel.main", bounds: { x: 74, y: 559, width: 335, height: 78 }, extraction: { mode: "reconstruct_skin", target_component_id: "button.draw.gold", remove_content: ["label", "subtitle"], transparent: true, evaluate_nine_slice: true, confidence: 0.99, reason: "Single and ten-draw buttons share one skin." } },
    { id: "button.draw.ten", category: "button", parent_id: "panel.main", bounds: { x: 542, y: 559, width: 335, height: 78 }, extraction: { mode: "reconstruct_skin", target_component_id: "button.draw.gold", remove_content: ["label", "subtitle"], transparent: true, evaluate_nine_slice: true, confidence: 0.99, reason: "Single and ten-draw buttons share one skin." } },
    { id: "text.draw.single", category: "text", parent_id: "button.draw.single", bounds: { x: 151, y: 572, width: 180, height: 48 }, extraction: { mode: "native", target_component_id: "text.draw.single", confidence: 1, reason: "Button label and subtitle remain native." } },
    { id: "text.draw.ten", category: "text", parent_id: "button.draw.ten", bounds: { x: 618, y: 572, width: 180, height: 48 }, extraction: { mode: "native", target_component_id: "text.draw.ten", confidence: 1, reason: "Button label and subtitle remain native." } },
    { id: "cost.single", category: "counter", parent_id: "panel.main", bounds: { x: 421, y: 563, width: 78, height: 70 }, extraction: { mode: "native", target_component_id: "counter.draw.cost", confidence: 1, reason: "Owned and required amounts are dynamic." } },
    { id: "cost.ten", category: "counter", parent_id: "panel.main", bounds: { x: 889, y: 563, width: 78, height: 70 }, extraction: { mode: "native", target_component_id: "counter.draw.cost", confidence: 1, reason: "Owned and required amounts are dynamic." } },
    { id: "tabs.pool", category: "tabs", bounds: { x: 992, y: 118, width: 214, height: 548 }, extraction: { mode: "composite", target_component_id: "composite.gem.pool.tabs", confidence: 0.97, reason: "Tabs use native labels and reusable selected/unselected skins." } },
  ],
};

function cloneTree(tree: UITree): UITree {
  return JSON.parse(JSON.stringify(tree)) as UITree;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function sameBounds(left: Bounds, right: Bounds) {
  return left.x === right.x && left.y === right.y && left.width === right.width && left.height === right.height;
}

function nativeDisplayClass(node: UINode) {
  if (node.id.includes("tab") || node.id.includes("upgrade") || node.id.includes("manage_building")) return "native-strong";
  if (node.id.includes("badge")) return "native-badge";
  if (node.category === "counter" || node.id.startsWith("stat.") || node.id.includes("value.") || node.id.includes("owned.")) return "native-small";
  return "native-default";
}

function nativeFontSize(node: UINode) {
  const closeStyle = nativeWorkbenchCloseTextStyle(node);
  if (closeStyle?.font_size !== undefined) return closeStyle.font_size;
  const height = node.bounds.height;
  const width = node.bounds.width;
  const cap = node.id.includes("tab") || node.id.includes("upgrade") || node.id.includes("manage_building") ? 28 : 18;
  return Math.round(clamp(Math.min(height * 0.58, width * 0.22), 9, cap));
}

function nativeTextDefaultColor(node: UINode) {
  const closeStyle = nativeWorkbenchCloseTextStyle(node);
  if (closeStyle?.color) return closeStyle.color;
  if (node.category === "counter" || node.id.startsWith("stat.") || node.id.includes("value.") || node.id.includes("owned.")) return "#4f3322";
  if (node.id.includes("badge")) return "#fff7da";
  if (node.id.includes("tab") || node.id.includes("upgrade") || node.id.includes("manage_building")) return "#fff2c6";
  return "#5b351e";
}

function nativeTextDefaultOutlineSize(node: UINode) {
  const closeStyle = nativeWorkbenchCloseTextStyle(node);
  if (closeStyle?.outline_size !== undefined) return closeStyle.outline_size;
  return node.id.includes("badge") || node.id.includes("tab") || node.id.includes("upgrade") || node.id.includes("manage_building") ? 1 : 0;
}

function nativeTextDefaultOutlineColor(node: UINode) {
  return nativeWorkbenchCloseTextStyle(node)?.outline_color ?? "#693716";
}

function containsBounds(parent: Bounds, child: Bounds) {
  return parent.x <= child.x && parent.y <= child.y
    && parent.x + parent.width >= child.x + child.width
    && parent.y + parent.height >= child.y + child.height
    && parent.width * parent.height > child.width * child.height;
}

function inferParentIds(nodes: UINode[]) {
  return nodes.map((node) => {
    if (node.parent_id) return node;
    const parent = nodes
      .filter((candidate) => {
        const canContainChildren = candidate.node_kind === "composite"
          || candidate.extraction.mode === "composite"
          || ["panel", "button", "tabs", "container", "group"].includes(candidate.category);
        return candidate.id !== node.id && canContainChildren && containsBounds(candidate.bounds, node.bounds);
      })
      .sort((left, right) => left.bounds.width * left.bounds.height - right.bounds.width * right.bounds.height)[0];
    return parent ? { ...node, parent_id: parent.id } : node;
  });
}

function nodeKindFor(node: UINode, hasChildren: boolean): NodeKind {
  if (node.node_kind && ["composite", "skin", "artwork", "native", "interaction"].includes(node.node_kind)) return node.node_kind;
  if (node.extraction.mode === "native") return "native";
  if (node.extraction.mode === "extract_artwork") return "artwork";
  if (node.extraction.mode === "composite") return "composite";
  return hasChildren ? "composite" : "skin";
}

function defaultRenderMode(nodeKind: NodeKind): RenderMode {
  return nodeKind === "composite" || nodeKind === "native" || nodeKind === "interaction" ? "outline" : "bitmap";
}

function normalizeTree(input: UITree): UITree {
  const sourceNodes = inferParentIds(input.nodes.filter((node) => !node.derived_from));
  const parentIds = new Set(sourceNodes.map((node) => node.parent_id).filter(Boolean));
  const normalized: UINode[] = [];
  for (const node of sourceNodes) {
    const hasChildren = parentIds.has(node.id);
    const nodeKind = nodeKindFor(node, hasChildren);
    const assets = {
      source_crop: node.visual_assets?.source_crop ?? "__source__",
      clean_layer: node.visual_assets?.clean_layer ?? node.visual_assets?.clean_asset ?? null,
      assembly_preview: node.visual_assets?.assembly_preview ?? null,
      native_preview: node.visual_assets?.native_preview ?? null,
    };
    const cleanupStatus = nodeKind === "composite" || nodeKind === "native" || nodeKind === "interaction" ? "not_applicable"
      : assets.clean_layer ? "clean" : node.review?.cleanup_status && node.review.cleanup_status !== "not_applicable" ? node.review.cleanup_status : "needs_cleanup";
    const parentNode: UINode = {
      ...node,
      node_kind: nodeKind,
      render_mode: node.render_mode ?? defaultRenderMode(nodeKind),
      visual_assets: nodeKind === "composite" ? { ...assets, clean_layer: null } : assets,
      review: { status: cleanupStatus === "clean" ? (node.review?.status ?? "pending_review") : "candidate", cleanup_status: cleanupStatus },
      reusable_bitmap: (nodeKind === "skin" || nodeKind === "artwork") && cleanupStatus === "clean" && Boolean(assets.clean_layer),
    };
    normalized.push(parentNode);

    if (nodeKind !== "composite" || node.extraction.mode !== "reconstruct_skin") continue;
    const backgroundId = `${node.id}.background`;
    normalized.push({
      ...node,
      id: backgroundId,
      name: `${node.name ?? node.id} Background`,
      parent_id: node.id,
      node_kind: "skin",
      render_mode: "bitmap",
      visual_assets: assets,
      review: { status: assets.clean_layer ? "pending_review" : "candidate", cleanup_status: assets.clean_layer ? "clean" : "needs_cleanup" },
      reusable_bitmap: Boolean(assets.clean_layer),
      derived_from: node.id,
      z_index: (node.z_index ?? 0) - 0.5,
    });
  }
  return { ...input, schema_version: 2, nodes: normalized };
}

function coerceTree(raw: unknown): UITree {
  if (!raw || typeof raw !== "object") throw new Error("JSON 根节点必须是对象");
  const data = raw as Record<string, unknown>;
  const rawNodes = (data.nodes ?? data.controls ?? data.components) as unknown;
  const source = data.source && typeof data.source === "object" ? data.source as Record<string, unknown> : {};
  const pageSize = (data.page_size ?? data.source_size ?? source.page_size ?? CITY_DEFENCE_PAGE_SIZE) as UITree["page_size"] | undefined;
  if (!pageSize || !Array.isArray(rawNodes)) throw new Error("缺少 page_size/source.page_size 或 nodes/controls/components");
  const nodes = rawNodes.map((value, index) => {
    const item = value as Partial<UINode> & { component_id?: string; layer?: number; status?: string };
    const id = item.id ?? item.component_id ?? `control.imported.${index + 1}`;
    const nodeKind = item.node_kind;
    const mode: ExtractionMode = item.extraction?.mode
      ?? (nodeKind === "native" || nodeKind === "interaction" ? "native" : nodeKind === "artwork" ? "extract_artwork" : nodeKind === "composite" ? "composite" : "reconstruct_skin");
    if (!item.bounds) throw new Error(`${id} 缺少 bounds`);
    return {
      ...item,
      id,
      category: item.category ?? "unknown",
      z_index: item.z_index ?? item.layer ?? index,
      extraction: item.extraction ?? { mode, target_component_id: id, confidence: 0.5, reason: "Imported semantic node." },
    } as UINode;
  });
  return normalizeTree({
    artifact_type: "ui_tree",
    schema_version: Number(data.schema_version ?? 1),
    status: String(data.status ?? "candidate"),
    page_size: { width: Number(pageSize.width), height: Number(pageSize.height) },
    nodes,
  });
}

function directVisualUrl(value: string | null | undefined) {
  return value && /^(\/|blob:|data:|https?:|asset:)/.test(value) ? value : null;
}

function cleanLayerPath(node: UINode) {
  return node.visual_assets?.clean_layer ?? node.visual_assets?.clean_asset ?? null;
}

function nativePreviewPath(node: UINode) {
  return node.visual_assets?.native_preview ?? null;
}

function displayPreviewPath(node: UINode) {
  return node.node_kind === "native" ? nativePreviewPath(node) : cleanLayerPath(node);
}

function makeSessionUrl(basePath: string, value: string | null | undefined) {
  if (!value || value === "__source__") return null;
  if (directVisualUrl(value)) return value;
  return `${basePath}/${value.replace(/\\/g, "/").replace(/^\/+/, "")}`;
}

function sessionAssetUrls(nodes: UINode[], basePath: string) {
  const urls: AssetUrls = {};
  for (const node of nodes) {
    urls[node.id] = {
      ...(makeSessionUrl(basePath, node.visual_assets?.source_crop) ? { source: makeSessionUrl(basePath, node.visual_assets?.source_crop)! } : {}),
      ...(makeSessionUrl(basePath, displayPreviewPath(node)) ? { clean: makeSessionUrl(basePath, displayPreviewPath(node))! } : {}),
      ...(makeSessionUrl(basePath, node.visual_assets?.assembly_preview) ? { assembly: makeSessionUrl(basePath, node.visual_assets?.assembly_preview)! } : {}),
    };
  }
  return urls;
}

function externalSessionAssetUrls(nodes: UINode[], baseUrl: string) {
  const urls: AssetUrls = {};
  for (const node of nodes) {
    const source = node.visual_assets?.source_crop;
    const clean = displayPreviewPath(node);
    const assembly = node.visual_assets?.assembly_preview;
    urls[node.id] = {
      ...(source && source !== "__source__" ? { source: resolveSessionAssetUrl(baseUrl, source) } : {}),
      ...(clean ? { clean: resolveSessionAssetUrl(baseUrl, clean) } : {}),
      ...(assembly ? { assembly: resolveSessionAssetUrl(baseUrl, assembly) } : {}),
    };
  }
  return urls;
}

function persistedSessionAssetUrls(nodes: UINode[], byPath: Map<string, string>) {
  const urls: AssetUrls = {};
  for (const node of nodes) {
    const clean = displayPreviewPath(node);
    const assembly = node.visual_assets?.assembly_preview;
    urls[node.id] = {
      ...(clean && byPath.has(clean) ? { clean: byPath.get(clean)! } : {}),
      ...(assembly && byPath.has(assembly) ? { assembly: byPath.get(assembly)! } : {}),
    };
  }
  return urls;
}

function descendantIds(nodes: UINode[], parentId: string) {
  const result: string[] = [];
  const pending = [parentId];
  while (pending.length > 0) {
    const current = pending.shift()!;
    for (const node of nodes) {
      if (node.parent_id !== current || result.includes(node.id)) continue;
      result.push(node.id);
      pending.push(node.id);
    }
  }
  return result;
}

function effectiveVisible(node: UINode, nodes: UINode[]) {
  const byId = new Map(nodes.map((item) => [item.id, item]));
  let current: UINode | undefined = node;
  const visited = new Set<string>();
  while (current) {
    if (current.visible === false) return false;
    if (!current.parent_id || visited.has(current.parent_id)) break;
    visited.add(current.parent_id);
    current = byId.get(current.parent_id);
  }
  return true;
}

function effectiveOpacity(node: UINode, nodes: UINode[]) {
  const byId = new Map(nodes.map((item) => [item.id, item]));
  let current: UINode | undefined = node;
  let opacity = 1;
  const visited = new Set<string>();
  while (current) {
    opacity *= current.opacity ?? 1;
    if (!current.parent_id || visited.has(current.parent_id)) break;
    visited.add(current.parent_id);
    current = byId.get(current.parent_id);
  }
  return opacity;
}

function buildLayerRows(nodes: UINode[], collapsed: Set<string>, query: string) {
  const children = new Map<string, UINode[]>();
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const sourceOrder = new Map(nodes.map((node, index) => [node.id, index]));
  for (const node of nodes) {
    const parent = node.parent_id && byId.has(node.parent_id) ? node.parent_id : "__root__";
    children.set(parent, [...(children.get(parent) ?? []), node]);
  }
  for (const siblings of children.values()) {
    siblings.sort((left, right) => (left.z_index ?? 0) - (right.z_index ?? 0)
      || (sourceOrder.get(left.id) ?? 0) - (sourceOrder.get(right.id) ?? 0));
  }
  const term = query.trim().toLowerCase();
  const included = new Set<string>();
  if (term) {
    for (const node of nodes) {
      if (!node.id.toLowerCase().includes(term) && !node.category.toLowerCase().includes(term)) continue;
      included.add(node.id);
      let parentId = node.parent_id;
      while (parentId && byId.has(parentId)) {
        included.add(parentId);
        parentId = byId.get(parentId)?.parent_id;
      }
    }
  }
  const rows: LayerRow[] = [];
  function visit(parentId: string, depth: number) {
    for (const node of children.get(parentId) ?? []) {
      const hasChildren = (children.get(node.id)?.length ?? 0) > 0;
      if (!term || included.has(node.id)) rows.push({ node, depth, hasChildren });
      if ((!collapsed.has(node.id) || term) && (!term || included.has(node.id))) visit(node.id, depth + 1);
    }
  }
  if (!collapsed.has("__root__") || term) visit("__root__", 1);
  return rows;
}

export default function UIWorkbench() {
  const [pageCatalog, setPageCatalog] = useState<WorkbenchCatalog>({ selected_page_id: null, pages: [] });
  const [tree, setTree] = useState<UITree>(() => normalizeTree(cloneTree(DEFAULT_TREE)));
  const [imageUrl, setImageUrl] = useState(DEMO_IMAGE);
  const [imageName, setImageName] = useState("gem-lottery-ui-draft-v1.png");
  const [selectedId, setSelectedId] = useState(DEFAULT_TREE.nodes[1].id);
  const [zoom, setZoom] = useState(72);
  const [gridVisible, setGridVisible] = useState(true);
  const [snap, setSnap] = useState(false);
  const [moveScope, setMoveScope] = useState<MoveScope>("layer");
  const [query, setQuery] = useState("");
  const [galleryTab, setGalleryTab] = useState<GalleryTab>("assets");
  const [galleryFilter, setGalleryFilter] = useState<GalleryFilter>("all");
  const [visualMode, setVisualMode] = useState<VisualMode>("source");
  const [referenceVisible, setReferenceVisible] = useState(true);
  const [referenceOpacity, setReferenceOpacity] = useState(0.88);
  const [assetUrls, setAssetUrls] = useState<AssetUrls>({});
  const [galleryOpen, setGalleryOpen] = useState(true);
  const [collapsedLayers, setCollapsedLayers] = useState<Set<string>>(() => new Set());
  const [panning, setPanning] = useState(false);
  const [canvasPan, setCanvasPan] = useState({ x: 0, y: 0 });
  const [notice, setNotice] = useState("演示项目已载入");
  const [analyzing, setAnalyzing] = useState(false);
  const [workflowStore, setWorkflowStore] = useState<UiWorkflowStore>({ selected_task_id: null, tasks: [] });
  const [deliveryDialog, setDeliveryDialog] = useState<DeliveryDialogState | null>(null);
  const [deliveryBusy, setDeliveryBusy] = useState(false);
  const [treeRevealRequest, setTreeRevealRequest] = useState<TreeRevealRequest | null>(null);
  const imageInput = useRef<HTMLInputElement>(null);
  const treeInput = useRef<HTMLInputElement>(null);
  const assetInput = useRef<HTMLInputElement>(null);
  const treeListRef = useRef<HTMLDivElement>(null);
  const viewportRef = useRef<HTMLDivElement>(null);
  const stageRef = useRef<HTMLDivElement>(null);
  const interactionRef = useRef<Interaction | null>(null);
  const panRef = useRef<PanInteraction | null>(null);
  const canvasPanRef = useRef(canvasPan);
  const wheelAnchorRef = useRef<{ clientX: number; clientY: number; canvasX: number; canvasY: number } | null>(null);
  const ownedImageUrlRef = useRef<string | null>(null);
  const treeRevealSequenceRef = useRef(0);

  const scale = zoom / 100;
  const selected = tree.nodes.find((node) => node.id === selectedId) ?? null;
  const selectedSupportsText = Boolean(selected
    && selected.node_kind === "native"
    && (["text", "counter"].includes(selected.category) || nativeWorkbenchDisplayText(selected)));
  const layerRows = useMemo(() => buildLayerRows(tree.nodes, collapsedLayers, query), [collapsedLayers, query, tree.nodes]);
  const renderRows = useMemo(() => buildLayerRows(tree.nodes, new Set(), ""), [tree.nodes]);
  const layerOrder = useMemo(() => new Map(renderRows.map((row, index) => [row.node.id, index])), [renderRows]);
  const nodeById = useMemo(() => new Map(tree.nodes.map((node) => [node.id, node])), [tree.nodes]);
  const directChildren = useMemo(() => {
    const result = new Map<string, UINode[]>();
    for (const node of tree.nodes) {
      const parentId = node.parent_id && nodeById.has(node.parent_id) ? node.parent_id : "__root__";
      result.set(parentId, [...(result.get(parentId) ?? []), node]);
    }
    return result;
  }, [nodeById, tree.nodes]);
  const galleryNodes = useMemo(() => tree.nodes.filter((node) => {
    const nodeKind = node.node_kind ?? "artwork";
    const inTab = galleryTab === "assets" ? nodeKind === "skin" || nodeKind === "artwork" : nodeKind === "composite" || nodeKind === "native" || nodeKind === "interaction";
    if (!inTab) return false;
    if (galleryFilter === "all") return true;
    if (galleryFilter === "needs_cleanup") return node.review?.cleanup_status === "needs_cleanup" || node.review?.cleanup_status === "requested";
    if (galleryFilter === "ready") return node.reusable_bitmap === true;
    return nodeKind === galleryFilter;
  }), [galleryFilter, galleryTab, tree.nodes]);
  const pageNavRows = useMemo(() => workbenchPageNavRows(pageCatalog), [pageCatalog]);
  const workflowTask = useMemo(
    () => workflowStore.tasks.find((task) => task.page_id === pageCatalog.selected_page_id) ?? null,
    [pageCatalog.selected_page_id, workflowStore.tasks],
  );
  const workflowStages = useMemo(
    () => workflowTask ? workflowStageRows(workflowTask.stages) : [],
    [workflowTask],
  );
  const deliveryReady = Boolean(
    deliveryDialog?.preflight?.status === "ready"
      && deliveryDialog.preflight.selected_load_path === deliveryDialog.selectedLoadPath,
  );

  useEffect(() => {
    canvasPanRef.current = canvasPan;
  }, [canvasPan]);

  useEffect(() => {
    let unsubscribe: (() => void) | undefined;
    let cancelled = false;
    void connectWorkbenchSessions({
      getPendingUrl: async () => invoke<string | null>("get_pending_ui_workbench_url").catch(() => null),
      subscribe: async (handler) => listen<string>("ui-workbench://session", (event) => handler(event.payload)),
      loadExternal: loadForwardedSession,
      loadFallback: loadCatalogOrBuiltIn,
    }).then((stop) => {
      if (cancelled) stop();
      else unsubscribe = stop;
    }).catch((error) => {
      setNotice(`Workbench session connection failed: ${error}`);
      void loadCatalogOrBuiltIn();
    });
    return () => {
      cancelled = true;
      unsubscribe?.();
    };
  }, []);

  useEffect(() => () => {
    if (ownedImageUrlRef.current) URL.revokeObjectURL(ownedImageUrlRef.current);
  }, []);

  useEffect(() => {
    invoke<UiWorkflowStore>("list_ui_workflow_tasks")
      .then(setWorkflowStore)
      .catch((error) => setNotice(`UI workflow could not be loaded: ${error}`));
    const unlisten = listen<UiWorkflowStore>("ui-workflow://progress", (event) => {
      setWorkflowStore(event.payload);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (!selected || event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement || event.target instanceof HTMLSelectElement) return;
      if (event.key === "Delete") deleteSelected();
      if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key) && !selected.locked) {
        event.preventDefault();
        const step = event.shiftKey ? 10 : 1;
        const dx = event.key === "ArrowLeft" ? -step : event.key === "ArrowRight" ? step : 0;
        const dy = event.key === "ArrowUp" ? -step : event.key === "ArrowDown" ? step : 0;
        moveNode(selected.id, dx, dy);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  useEffect(() => {
    if (!treeRevealRequest) return;
    const expandedIds = [
      "__root__",
      ...workbenchLayerAncestorIds(tree.nodes, treeRevealRequest.id),
    ];
    setCollapsedLayers((current) => {
      const next = new Set(current);
      let changed = false;
      for (const id of expandedIds) changed = next.delete(id) || changed;
      return changed ? next : current;
    });
  }, [tree.nodes, treeRevealRequest]);

  useLayoutEffect(() => {
    if (!treeRevealRequest || !treeListRef.current) return;
    const row = Array.from(
      treeListRef.current.querySelectorAll<HTMLElement>("[data-tree-node-id]"),
    ).find((candidate) => candidate.dataset.treeNodeId === treeRevealRequest.id);
    if (!row) return;
    row.scrollIntoView({ block: "center", inline: "nearest" });
    setTreeRevealRequest((current) => (
      current?.sequence === treeRevealRequest.sequence ? null : current
    ));
  }, [layerRows, treeRevealRequest]);

  function flash(message: string) {
    setNotice(message);
    window.setTimeout(() => setNotice("就绪"), 2400);
  }

  function selectCanvasNode(id: string) {
    setQuery("");
    setSelectedId(id);
    treeRevealSequenceRef.current += 1;
    setTreeRevealRequest({ id, sequence: treeRevealSequenceRef.current });
  }

  function patchNode(id: string, patch: Partial<UINode>) {
    setTree((current) => ({ ...current, nodes: current.nodes.map((node) => node.id === id ? { ...node, ...patch } : node) }));
  }

  function affectedNodeIds(id: string) {
    return moveScope === "group" ? [id, ...descendantIds(tree.nodes, id)] : [id];
  }

  function resizeNode(id: string, bounds: Bounds) {
    const root = tree.nodes.find((node) => node.id === id);
    if (!root) return;
    const affected = new Set(affectedNodeIds(id));
    const scaleX = bounds.width / root.bounds.width;
    const scaleY = bounds.height / root.bounds.height;
    setTree((current) => ({
      ...current,
      nodes: current.nodes.map((node) => {
        if (!affected.has(node.id)) return node;
        if (node.id === id) return { ...node, source_bounds: node.source_bounds ?? { ...node.bounds }, bounds };
        return {
          ...node,
          source_bounds: node.source_bounds ?? { ...node.bounds },
          bounds: {
            x: bounds.x + (node.bounds.x - root.bounds.x) * scaleX,
            y: bounds.y + (node.bounds.y - root.bounds.y) * scaleY,
            width: node.bounds.width * scaleX,
            height: node.bounds.height * scaleY,
          },
        };
      }),
    }));
  }

  function moveNode(id: string, dx: number, dy: number) {
    const affected = new Set(affectedNodeIds(id));
    setTree((current) => ({
      ...current,
      nodes: current.nodes.map((node) => affected.has(node.id)
        ? {
            ...node,
            source_bounds: node.source_bounds ?? { ...node.bounds },
            bounds: { ...node.bounds, x: node.bounds.x + dx, y: node.bounds.y + dy },
          }
        : node),
    }));
  }

  function patchSelected(patch: Partial<UINode>) {
    if (selected) patchNode(selected.id, patch);
  }

  function patchSelectedTextStyle(patch: Partial<WorkbenchTextStyle>) {
    if (selected) patchSelected({ text_style: { ...selected.text_style, ...patch } });
  }

  function renameSelected(nextId: string) {
    if (!selected) return;
    const previousId = selected.id;
    setTree((current) => ({
      ...current,
      nodes: current.nodes.map((node) => ({
        ...node,
        id: node.id === previousId ? nextId : node.id,
        parent_id: node.parent_id === previousId ? nextId : node.parent_id,
      })),
    }));
    setSelectedId(nextId);
  }

  function patchBounds(key: keyof Bounds, value: number) {
    if (!selected) return;
    const minimum = key === "width" || key === "height" ? 1 : -10000;
    const nextValue = Math.max(minimum, Number.isFinite(value) ? value : 0);
    if (key === "x" || key === "y") {
      moveNode(selected.id, key === "x" ? nextValue - selected.bounds.x : 0, key === "y" ? nextValue - selected.bounds.y : 0);
      return;
    }
    resizeNode(selected.id, { ...selected.bounds, [key]: nextValue });
  }

  function patchExtraction(patch: Partial<Extraction>) {
    if (!selected) return;
    setTree((current) => normalizeTree({
      ...current,
      nodes: current.nodes.map((node) => node.id === selected.id ? { ...node, extraction: { ...node.extraction, ...patch }, node_kind: undefined, render_mode: undefined } : node),
    }));
  }

  function patchNodeKind(nodeKind: NodeKind) {
    if (!selected) return;
    const renderMode = defaultRenderMode(nodeKind);
    patchNode(selected.id, {
      node_kind: nodeKind,
      render_mode: renderMode,
      reusable_bitmap: (nodeKind === "skin" || nodeKind === "artwork") && Boolean(cleanLayerPath(selected)),
      review: {
        status: cleanLayerPath(selected) ? "pending_review" : "candidate",
        cleanup_status: nodeKind === "composite" || nodeKind === "native" || nodeKind === "interaction"
          ? "not_applicable"
          : cleanLayerPath(selected) ? "clean" : "needs_cleanup",
      },
    });
  }

  function addNode(parent: UINode | null) {
    const next = tree.nodes.length + 1;
    const width = parent ? Math.max(8, Math.min(180, parent.bounds.width * 0.72)) : 180;
    const height = parent ? Math.max(8, Math.min(90, parent.bounds.height * 0.72)) : 90;
    const node: UINode = {
      id: `control.new.${next}`,
      category: parent ? "group" : "panel",
      parent_id: parent?.id,
      bounds: parent ? {
        x: parent.bounds.x + (parent.bounds.width - width) / 2,
        y: parent.bounds.y + (parent.bounds.height - height) / 2,
        width,
        height,
      } : { x: 40 + next * 4, y: 40 + next * 4, width, height },
      extraction: parent
        ? { mode: "composite", target_component_id: `group.new.${next}`, confidence: 0.5, reason: "Manual child group." }
        : { mode: "extract_artwork", target_component_id: `artwork.new.${next}`, confidence: 0.5, reason: "Manual candidate." },
      node_kind: parent ? "composite" : undefined,
      render_mode: parent ? "outline" : undefined,
      visible: true,
      opacity: 1,
      z_index: next,
    };
    setTree((current) => ({ ...current, nodes: [...current.nodes, node] }));
    if (parent) {
      setCollapsedLayers((current) => {
        const expanded = new Set(current);
        expanded.delete(parent.id);
        return expanded;
      });
    }
    setSelectedId(node.id);
    flash(parent ? `已在 ${parent.id} 下新建子项` : "已新建根范围");
  }

  function duplicateSelected() {
    if (!selected) return;
    let suffix = 2;
    let id = `${selected.id}.copy`;
    while (tree.nodes.some((node) => node.id === id)) id = `${selected.id}.copy${suffix++}`;
    const copy = cloneTree({ ...tree, nodes: [selected] }).nodes[0];
    copy.id = id;
    copy.source_bounds = copy.source_bounds ?? { ...copy.bounds };
    copy.bounds.x += 18;
    copy.bounds.y += 18;
    setTree((current) => ({ ...current, nodes: [...current.nodes, copy] }));
    setSelectedId(id);
    flash("已复制控件");
  }

  function deleteSelected() {
    if (!selected) return;
    const index = tree.nodes.findIndex((node) => node.id === selected.id);
    const nextNodes = tree.nodes.filter((node) => node.id !== selected.id).map((node) => node.parent_id === selected.id ? { ...node, parent_id: selected.parent_id } : node);
    setTree((current) => ({ ...current, nodes: nextNodes }));
    setSelectedId(nextNodes[Math.min(index, Math.max(0, nextNodes.length - 1))]?.id ?? "");
    flash("已删除控件");
  }

  function resetDemo() {
    void loadBuiltInCityDefenceSession();
  }

  function resetSelectedLayout() {
    if (!selected?.source_bounds) return;
    const affected = new Set(affectedNodeIds(selected.id));
    setTree((current) => ({
      ...current,
      nodes: current.nodes.map((node) => {
        if (!affected.has(node.id) || !node.source_bounds) return node;
        const source = { ...node.source_bounds };
        const { source_bounds: _sourceBounds, ...rest } = node;
        return { ...rest, bounds: source };
      }),
    }));
    flash("控件已回到原始位置");
  }

  function setSelectedParent(parentId: string) {
    if (!selected) return;
    const forbidden = new Set([selected.id, ...descendantIds(tree.nodes, selected.id)]);
    patchSelected({ parent_id: parentId && !forbidden.has(parentId) ? parentId : undefined });
  }

  function toggleLayer(id: string) {
    setCollapsedLayers((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  }

  function fitCanvas(pageSize = tree.page_size) {
    const viewport = viewportRef.current;
    if (!viewport) return;
    const next = Math.floor(Math.min((viewport.clientWidth - 72) / pageSize.width, (viewport.clientHeight - 72) / pageSize.height) * 100);
    setZoom(clamp(next, 10, 400));
    setCanvasPan({ x: 0, y: 0 });
  }

  useLayoutEffect(() => {
    const anchor = wheelAnchorRef.current;
    const viewport = viewportRef.current;
    const stage = stageRef.current;
    if (!anchor || !viewport || !stage) return;
    const rect = stage.getBoundingClientRect();
    const nextClientX = rect.left + anchor.canvasX * scale;
    const nextClientY = rect.top + anchor.canvasY * scale;
    setCanvasPan((current) => ({
      x: current.x + anchor.clientX - nextClientX,
      y: current.y + anchor.clientY - nextClientY,
    }));
    wheelAnchorRef.current = null;
  }, [scale]);

  function zoomAtPointer(event: WheelEvent) {
    event.preventDefault();
    const stage = stageRef.current;
    if (!stage) return;
    const rect = stage.getBoundingClientRect();
    wheelAnchorRef.current = {
      clientX: event.clientX,
      clientY: event.clientY,
      canvasX: (event.clientX - rect.left) / scale,
      canvasY: (event.clientY - rect.top) / scale,
    };
    const factor = event.deltaY < 0 ? 1.12 : 1 / 1.12;
    setZoom((value) => clamp(Math.round(value * factor), 10, 400));
  }

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) return;
    viewport.addEventListener("wheel", zoomAtPointer, { passive: false });
    return () => viewport.removeEventListener("wheel", zoomAtPointer);
  }, [scale]);

  useEffect(() => {
    function beginPan(event: MouseEvent) {
      const viewport = viewportRef.current;
      if (event.button !== 1 || !viewport || !(event.target instanceof Node) || !viewport.contains(event.target)) return;
      event.preventDefault();
      panRef.current = { startX: event.clientX, startY: event.clientY, panX: canvasPanRef.current.x, panY: canvasPanRef.current.y };
      setPanning(true);
    }

    function movePan(event: MouseEvent) {
      const pan = panRef.current;
      if (!pan) return;
      event.preventDefault();
      if ((event.buttons & 4) === 0) {
        panRef.current = null;
        setPanning(false);
        return;
      }
      setCanvasPan({ x: pan.panX + event.clientX - pan.startX, y: pan.panY + event.clientY - pan.startY });
    }

    function endPan(event: MouseEvent) {
      if (event.button !== 1 || !panRef.current) return;
      event.preventDefault();
      panRef.current = null;
      setPanning(false);
    }

    function suppressMiddleClick(event: MouseEvent) {
      const viewport = viewportRef.current;
      if (event.button === 1 && viewport && event.target instanceof Node && viewport.contains(event.target)) event.preventDefault();
    }

    window.addEventListener("mousedown", beginPan, true);
    window.addEventListener("mousemove", movePan, true);
    window.addEventListener("mouseup", endPan, true);
    window.addEventListener("auxclick", suppressMiddleClick, true);
    return () => {
      window.removeEventListener("mousedown", beginPan, true);
      window.removeEventListener("mousemove", movePan, true);
      window.removeEventListener("mouseup", endPan, true);
      window.removeEventListener("auxclick", suppressMiddleClick, true);
    };
  }, []);

  async function applyAutomaticAnalysis(image: HTMLImageElement, nextUrl: string, nextName: string) {
    if (analyzing) return;
    setAnalyzing(true);
    setNotice("正在自动识别背景、控件、文字和层级…");
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
    try {
      const result = await analyzeUIImage(image);
      const next = normalizeTree(result.tree as UITree);
      setTree(next);
      setImageUrl(nextUrl);
      setImageName(nextName);
      setSelectedId(next.nodes.find((node) => node.category === "panel")?.id ?? next.nodes[0]?.id ?? "");
      setAssetUrls({});
      setCollapsedLayers(new Set());
      setReferenceVisible(true);
      setReferenceOpacity(0.72);
      setGalleryTab("structure");
      setGalleryFilter("all");
      setSnap(false);
      setMoveScope("layer");
      window.setTimeout(() => fitCanvas(next.page_size), 0);
      flash(`自动识别完成：${result.stats.panels} 面板 · ${result.stats.buttons} 按钮 · ${result.stats.text} 文字 · ${result.stats.artwork} 图标`);
    } catch (error) {
      flash(`自动识别失败: ${error}`);
    } finally {
      setAnalyzing(false);
    }
  }

  async function refreshPageCatalog() {
    const catalog = await invoke<WorkbenchCatalog>("list_ui_workbench_pages");
    setPageCatalog(catalog);
    return catalog;
  }

  async function loadCatalogOrBuiltIn() {
    try {
      const catalog = await refreshPageCatalog();
      const pageId = preferredWorkbenchPage(catalog);
      if (pageId) {
        await loadPersistedPage(pageId);
        return;
      }
    } catch (error) {
      setNotice(`Persistent page catalog could not be loaded: ${error}`);
    }
    await loadBuiltInCityDefenceSession();
  }

  async function loadForwardedSession(baseUrl: string) {
    try {
      const catalog = await refreshPageCatalog();
      const pageId = preferredWorkbenchPage(catalog);
      if (pageId) {
        await loadPersistedPage(pageId);
        return;
      }
    } catch {
      // A URL-only handoff remains a supported ephemeral session.
    }
    await loadExternalSession(baseUrl);
  }

  async function selectPersistedPage(pageId: string) {
    if (pageId === pageCatalog.selected_page_id) return;
    try {
      const catalog = await invoke<WorkbenchCatalog>("select_ui_workbench_page", { pageId });
      await loadPersistedPage(pageId);
      setPageCatalog(catalog);
      const task = workflowStore.tasks.find((item) => item.page_id === pageId);
      if (task) {
        setWorkflowStore(await invoke<UiWorkflowStore>("select_ui_workflow_task", { taskId: task.task_id }));
      }
    } catch (error) {
      setNotice(`UI page could not be opened: ${error}`);
    }
  }

  async function loadPersistedPage(pageId: string) {
    const loaded = await loadPersistedWorkbenchPage(pageId, {
      loadPage: (targetPageId) => invoke("load_ui_workbench_page", { pageId: targetPageId }),
      readAsset: (targetPageId, assetPath) => invoke("read_ui_workbench_asset", {
        pageId: targetPageId,
        assetPath,
      }),
    });
    const next = coerceTree(loaded.raw);
    const persistedAssets = await loadPersistedAssetUrls(
      pageId,
      loaded.raw,
      (targetPageId, assetPath) => invoke("read_ui_workbench_asset", {
        pageId: targetPageId,
        assetPath,
      }),
    );
    const nextAssetUrls = persistedSessionAssetUrls(next.nodes, persistedAssets);

    setTree(next);
    setImageUrl(loaded.sourceImageUrl);
    setImageName(loaded.title);
    setSelectedId(next.nodes[0]?.id ?? "");
    setAssetUrls(nextAssetUrls);
    setCollapsedLayers(new Set());
    setReferenceVisible(false);
    setReferenceOpacity(0.35);
    setGalleryTab("assets");
    setGalleryFilter("ready");
    setVisualMode("clean");
    setSnap(false);
    setMoveScope("group");
    setCanvasPan({ x: 0, y: 0 });
    setPageCatalog((catalog) => ({ ...catalog, selected_page_id: pageId }));
    window.setTimeout(() => fitCanvas(next.page_size), 0);
    flash(`${loaded.title} loaded: ${next.nodes.length} controls`);
  }

  async function loadBuiltInCityDefenceSession() {
    setNotice("Loading City Defence clean layers...");
    try {
      const response = await fetch(CITY_DEFENCE_CONTROLS, { cache: "no-store" });
      if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
      const raw = await response.json();
      const next = coerceTree({ ...raw, source_size: raw.source_size ?? CITY_DEFENCE_PAGE_SIZE });
      setTree(next);
      setImageUrl(CITY_DEFENCE_IMAGE);
      setImageName("city-defence-layer-reconstruction.png");
      setSelectedId(next.nodes.find((node) => node.id === "tab.wall")?.id ?? next.nodes[0]?.id ?? "");
      setAssetUrls(sessionAssetUrls(next.nodes, CITY_DEFENCE_SESSION_BASE));
      setCollapsedLayers(new Set());
      setReferenceVisible(false);
      setReferenceOpacity(0.35);
      setGalleryTab("assets");
      setGalleryFilter("ready");
      setVisualMode("clean");
      setSnap(false);
      setMoveScope("group");
      setZoom(72);
      setCanvasPan({ x: 0, y: 0 });
      window.setTimeout(() => fitCanvas(next.page_size), 0);
      flash(`City Defence loaded: ${next.nodes.filter((node) => Boolean(cleanLayerPath(node))).length} clean layers`);
    } catch (error) {
      const image = new Image();
      image.onload = () => void applyAutomaticAnalysis(image, DEMO_IMAGE, "gem-lottery-ui-draft-v1.png");
      image.onerror = () => flash("内置演示图读取失败");
      image.src = DEMO_IMAGE;
      flash(`City Defence session load failed: ${error}`);
    }
  }

  async function loadExternalSession(baseUrl: string) {
    try {
      const loaded = await loadWorkbenchSessionWithRetry(baseUrl);
      const next = coerceTree(loaded.raw);
      setTree(next);
      setImageUrl(loaded.sourceImageUrl);
      setImageName(typeof loaded.raw.source_name === "string" ? loaded.raw.source_name : "external-workbench-session.png");
      setSelectedId(next.nodes[0]?.id ?? "");
      setAssetUrls(externalSessionAssetUrls(next.nodes, loaded.assetBaseUrl));
      setCollapsedLayers(new Set());
      setReferenceVisible(false);
      setReferenceOpacity(0.35);
      setGalleryTab("assets");
      setGalleryFilter("ready");
      setVisualMode("clean");
      setSnap(false);
      setMoveScope("group");
      setCanvasPan({ x: 0, y: 0 });
      window.setTimeout(() => fitCanvas(next.page_size), 0);
      flash(`External workbench loaded: ${next.nodes.length} controls`);
    } catch (error) {
      try {
        const catalog = await refreshPageCatalog();
        const pageId = preferredWorkbenchPage(catalog);
        if (pageId) {
          await loadPersistedPage(pageId);
          setNotice("本地预览服务中断，已从持久会话自动恢复。服务守护进程会继续尝试修复 localhost 预览。");
          flash("UI 工作台已自动恢复");
          return;
        }
      } catch (recoveryError) {
        setNotice(`本地预览服务中断，自动恢复失败：${recoveryError}`);
        return;
      }
      setNotice(`本地预览服务中断，且没有可恢复的持久会话。请重新生成或重新打开该 UI：${error}`);
    }
  }

  function analyzeCurrentImage() {
    if (analyzing) return;
    const image = new Image();
    image.onload = () => void applyAutomaticAnalysis(image, imageUrl, imageName);
    image.onerror = () => flash("当前图片读取失败");
    image.src = imageUrl;
  }

  function onImageFile(file?: File) {
    if (!file || analyzing) return;
    const nextUrl = URL.createObjectURL(file);
    const image = new Image();
    image.onload = () => {
      if (ownedImageUrlRef.current) URL.revokeObjectURL(ownedImageUrlRef.current);
      ownedImageUrlRef.current = nextUrl;
      void applyAutomaticAnalysis(image, nextUrl, file.name);
    };
    image.onerror = () => {
      URL.revokeObjectURL(nextUrl);
      flash("图片读取失败");
    };
    image.src = nextUrl;
  }

  function onTreeFile(file?: File) {
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const next = coerceTree(JSON.parse(String(reader.result)));
        setTree(next);
        setSelectedId(next.nodes[0]?.id ?? "");
        flash(`已导入 ${file.name}`);
      } catch (error) {
        flash("UI Tree 读取失败: " + error);
      }
    };
    reader.readAsText(file);
  }

  function onAssetFiles(files?: FileList | null) {
    if (!files?.length) return;
    const candidates = Array.from(files);
    const findFile = (path: string | null | undefined) => {
      if (!path || path === "__source__") return null;
      const normalized = path.replace(/\\/g, "/");
      const name = normalized.split("/").pop();
      return candidates.find((file) => (file.webkitRelativePath || file.name).replace(/\\/g, "/").endsWith(normalized) || file.name === name) ?? null;
    };
    setAssetUrls((current) => {
      for (const slots of Object.values(current)) for (const url of Object.values(slots)) if (url?.startsWith("blob:")) URL.revokeObjectURL(url);
      const next: AssetUrls = {};
      for (const node of tree.nodes) {
        const source = findFile(node.visual_assets?.source_crop);
        const clean = findFile(displayPreviewPath(node));
        const assembly = findFile(node.visual_assets?.assembly_preview);
        next[node.id] = {
          ...(source ? { source: URL.createObjectURL(source) } : {}),
          ...(clean ? { clean: URL.createObjectURL(clean) } : {}),
          ...(assembly ? { assembly: URL.createObjectURL(assembly) } : {}),
        };
      }
      return next;
    });
    const matched = tree.nodes.filter((node) => {
      const assets = node.visual_assets;
      return findFile(assets?.clean_layer ?? assets?.clean_asset ?? assets?.native_preview) || findFile(assets?.assembly_preview);
    }).length;
    flash(`已载入 ${matched} 个节点的 Clean / Assembly 资产`);
  }

  function visualUrl(node: UINode, mode: VisualMode) {
    if (mode === "clean" && !cleanLayerPath(node) && node.node_kind === "composite") {
      const background = tree.nodes.find((candidate) => candidate.derived_from === node.id || candidate.id === `${node.id}.background`);
      if (background) return visualUrl(background, mode);
    }
    const loaded = assetUrls[node.id]?.[mode];
    if (loaded) return loaded;
    const value = mode === "source" ? node.visual_assets?.source_crop : mode === "clean" ? displayPreviewPath(node) : node.visual_assets?.assembly_preview;
    return directVisualUrl(value);
  }

  function queueSelectedCleanup() {
    if (!selected) return;
    const target = selected.node_kind === "composite"
      ? tree.nodes.find((node) => node.derived_from === selected.id || node.id === `${selected.id}.background`)
      : selected.node_kind === "skin" ? selected : null;
    if (!target || target.review?.cleanup_status === "clean") return;
    patchNode(target.id, { review: { status: "candidate", cleanup_status: "requested" }, reusable_bitmap: false });
    flash(`已将 ${target.id} 加入 Precision Reconstruction 队列`);
  }

  function selectGalleryTab(tab: GalleryTab) {
    setGalleryTab(tab);
    setGalleryFilter("all");
  }

  function exportTree() {
    const payload = JSON.stringify(tree, null, 2);
    const url = URL.createObjectURL(new Blob([payload], { type: "application/json" }));
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = imageName.replace(/\.[^.]+$/, "") + "-ui-tree.json";
    anchor.click();
    URL.revokeObjectURL(url);
    flash("UI Tree 已导出");
  }

  async function beginDelivery() {
    const pageId = pageCatalog.selected_page_id;
    if (!pageId) {
      flash("请先选择一个持久化 UI 页面");
      return;
    }
    try {
      const workflow = await invoke<UiWorkflowStore>("list_ui_workflow_tasks");
      setWorkflowStore(workflow);
      const task = workflow.tasks.find((item) => item.page_id === pageId);
      if (!task?.agent_context?.thread_id) {
        flash("当前页面没有来源 Codex 任务，无法自动投递");
        return;
      }
      const preflight = task.target.preflight?.status === "ready"
        && task.target.preflight.selected_load_path === task.target.widget_blueprint
        ? task.target.preflight
        : null;
      const restoredCandidate = preflight ? [{
        display_name: task.target.widget_blueprint_name || task.target.widget_blueprint,
        load_path: task.target.widget_blueprint,
        class_name: task.target.widget_blueprint_class as WidgetBlueprintCandidate["class_name"],
      }] : [];
      setDeliveryDialog({
        task,
        projectWorkspace: task.target.project_workspace || task.agent_context.workspace || "",
        query: task.target.widget_blueprint_name || "",
        candidates: restoredCandidate,
        selectedLoadPath: preflight?.selected_load_path || "",
        preflight,
        state: preflight ? "ready" : "idle",
        message: preflight?.message || "输入名称或项目内 Asset 路径，从编辑器读取 WidgetBlueprint",
      });
    } catch (error) {
      flash(`交付信息读取失败: ${error}`);
    }
  }

  async function searchDeliveryTargets() {
    if (!deliveryDialog) return;
    const projectWorkspace = deliveryDialog.projectWorkspace.trim();
    const query = deliveryDialog.query.trim();
    if (!projectWorkspace || !query) {
      setDeliveryDialog({
        ...deliveryDialog,
        state: "blocked",
        message: "请填写项目工作区和至少两个字符的搜索内容",
        candidates: [],
        selectedLoadPath: "",
        preflight: null,
      });
      return;
    }
    setDeliveryDialog({
      ...deliveryDialog,
      state: "searching_assets",
      message: "正在通过编辑器 MCP 搜索 WidgetBlueprint",
      candidates: [],
      selectedLoadPath: "",
      preflight: null,
    });
    try {
      const result = await invoke<WidgetBlueprintSearchResult>("search_widget_blueprints", {
        taskId: deliveryDialog.task.task_id,
        projectWorkspace,
        query,
      });
      setDeliveryDialog((current) => current ? {
        ...current,
        candidates: result.candidates,
        selectedLoadPath: "",
        preflight: null,
        state: result.candidates.length ? result.state : "blocked",
        message: result.candidates.length ? result.message : "编辑器没有返回匹配的 WidgetBlueprint",
      } : current);
    } catch (error) {
      setDeliveryDialog((current) => current ? {
        ...current,
        candidates: [],
        selectedLoadPath: "",
        preflight: null,
        state: "blocked",
        message: `搜索失败: ${error}`,
      } : current);
    }
  }

  async function selectDeliveryTarget(candidate: WidgetBlueprintCandidate) {
    if (!deliveryDialog) return;
    const projectWorkspace = deliveryDialog.projectWorkspace.trim();
    setDeliveryDialog({
      ...deliveryDialog,
      selectedLoadPath: candidate.load_path,
      preflight: null,
      state: "checking_mcp",
      message: "正在确认编辑器项目、资产类型和精确 load_path",
    });
    try {
      const next = await invoke<UiWorkflowStore>("preflight_ui_delivery", {
        taskId: deliveryDialog.task.task_id,
        projectWorkspace,
        selectedLoadPath: candidate.load_path,
      });
      setWorkflowStore(next);
      const task = next.tasks.find((item) => item.task_id === deliveryDialog.task.task_id);
      const preflight = task?.target.preflight ?? null;
      setDeliveryDialog((current) => current ? {
        ...current,
        task: task ?? current.task,
        selectedLoadPath: candidate.load_path,
        preflight,
        state: preflight?.status === "ready" ? "ready" : "blocked",
        message: preflight?.message || "编辑器预检没有返回可交付证据",
      } : current);
    } catch (error) {
      setDeliveryDialog((current) => current ? {
        ...current,
        preflight: null,
        state: "blocked",
        message: `预检失败: ${error}`,
      } : current);
    }
  }

  async function confirmAndDeliver() {
    if (!deliveryDialog) return;
    const evidence = deliveryDialog.preflight;
    if (
      evidence?.status !== "ready"
      || evidence.selected_load_path !== deliveryDialog.selectedLoadPath
    ) {
      setDeliveryDialog({
        ...deliveryDialog,
        state: "blocked",
        message: "请先选择一个 WidgetBlueprint 并完成编辑器只读预检",
      });
      return;
    }
    setDeliveryBusy(true);
    let prepared = false;
    try {
      const result = await invoke<DeliveryDispatchResult>("confirm_and_deliver_ui", {
        pageId: deliveryDialog.task.page_id,
        tree,
        evidenceId: evidence.evidence_id,
      });
      prepared = true;
      await openUrl(result.new_task_url);
      const submission = await invoke<CodexPromptSubmissionResult>("submit_codex_new_task_prompt", {
        pageId: deliveryDialog.task.page_id,
      });
      setDeliveryDialog(null);
      flash(submission.submitted
        ? `已新建 Codex 任务并开始实现：${result.delivery_id}`
        : submission.message);
    } catch (error) {
      flash(prepared ? `交付文件已冻结，但 Codex 跳转失败: ${error}` : `交付失败: ${error}`);
    } finally {
      setDeliveryBusy(false);
    }
  }

  function beginInteraction(event: React.PointerEvent, node: UINode, kind: Interaction["kind"], handle?: string) {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    selectCanvasNode(node.id);
    if (node.locked) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    const affectedIds = affectedNodeIds(node.id);
    const originals = Object.fromEntries(tree.nodes.filter((item) => affectedIds.includes(item.id)).map((item) => [item.id, { ...item.bounds }]));
    setTree((current) => ({
      ...current,
      nodes: current.nodes.map((item) => affectedIds.includes(item.id) && !item.source_bounds ? { ...item, source_bounds: { ...item.bounds } } : item),
    }));
    interactionRef.current = { nodeId: node.id, kind, handle, startX: event.clientX, startY: event.clientY, affectedIds, originals };
  }

  function moveInteraction(event: React.PointerEvent) {
    const interaction = interactionRef.current;
    if (!interaction) return;
    const dx = (event.clientX - interaction.startX) / scale;
    const dy = (event.clientY - interaction.startY) / scale;
    const grid = event.altKey ? 1 : snap ? 4 : 1;
    const quantize = (value: number) => Math.round(value / grid) * grid;
    if (interaction.kind === "drag") {
      const rootOriginal = interaction.originals[interaction.nodeId];
      const rootX = quantize(rootOriginal.x + dx);
      const rootY = quantize(rootOriginal.y + dy);
      const snappedDx = rootX - rootOriginal.x;
      const snappedDy = rootY - rootOriginal.y;
      setTree((current) => ({
        ...current,
        nodes: current.nodes.map((node) => {
          const original = interaction.originals[node.id];
          return original ? { ...node, source_bounds: node.source_bounds ?? { ...original }, bounds: { ...original, x: original.x + snappedDx, y: original.y + snappedDy } } : node;
        }),
      }));
      return;
    }
    const original = interaction.originals[interaction.nodeId];
    const next = { ...original };
    const handle = interaction.handle ?? "se";
    if (handle.includes("e")) next.width = Math.max(8, quantize(original.width + dx));
    if (handle.includes("s")) next.height = Math.max(8, quantize(original.height + dy));
    if (handle.includes("w")) {
      const width = Math.max(8, quantize(original.width - dx));
      next.x = original.x + original.width - width;
      next.width = width;
    }
    if (handle.includes("n")) {
      const height = Math.max(8, quantize(original.height - dy));
      next.y = original.y + original.height - height;
      next.height = height;
    }
    const scaleX = next.width / original.width;
    const scaleY = next.height / original.height;
    setTree((current) => ({
      ...current,
      nodes: current.nodes.map((node) => {
        const childOriginal = interaction.originals[node.id];
        if (!childOriginal) return node;
        const bounds = node.id === interaction.nodeId ? next : {
          x: next.x + (childOriginal.x - original.x) * scaleX,
          y: next.y + (childOriginal.y - original.y) * scaleY,
          width: childOriginal.width * scaleX,
          height: childOriginal.height * scaleY,
        };
        return { ...node, source_bounds: node.source_bounds ?? { ...childOriginal }, bounds };
      }),
    }));
  }

  function handleCanvasPointerMove(event: React.PointerEvent<HTMLDivElement>) {
    moveInteraction(event);
  }

  function endInteraction() {
    interactionRef.current = null;
  }

  return (
    <main className="ui-workbench">
      <input ref={imageInput} type="file" accept="image/png,image/jpeg,image/webp" hidden onChange={(event) => onImageFile(event.target.files?.[0])} />
      <input ref={treeInput} type="file" accept="application/json,.json" hidden onChange={(event) => onTreeFile(event.target.files?.[0])} />
      <input ref={assetInput} type="file" accept="image/png,image/webp" multiple hidden onChange={(event) => onAssetFiles(event.target.files)} />

      <header className="workbench-topbar">
        <div className="workbench-brand"><span className="workbench-logo" /> <strong>Oasis UI 工作台</strong><small>{imageName}</small></div>
        <div className="toolbar-group">
          <button type="button" onClick={() => imageInput.current?.click()} title="导入图片并自动识别" disabled={analyzing}>导入图片</button>
          <button type="button" onClick={analyzeCurrentImage} title="重新分析当前图片的控件和层级" disabled={analyzing}>{analyzing ? "识别中…" : "重新自动识别"}</button>
          <button type="button" onClick={() => treeInput.current?.click()} title="导入 UI Tree JSON">导入 UI Tree</button>
          <button type="button" onClick={() => assetInput.current?.click()} title="载入 Clean Asset 与 Assembly Preview">导入资产</button>
          <button type="button" onClick={exportTree} className="accent">导出 UI Tree</button>
          <button type="button" onClick={beginDelivery} className="delivery-action">确认并交付到编辑器</button>
        </div>
        <div className="toolbar-group compact">
          <button type="button" onClick={() => addNode(null)} title="新建根控件范围">＋</button>
          <button type="button" onClick={duplicateSelected} disabled={!selected} title="复制所选控件">复制</button>
          <button type="button" onClick={resetSelectedLayout} disabled={!selected?.source_bounds} title="恢复控件原始布局">归位</button>
          <button type="button" onClick={deleteSelected} disabled={!selected} title="删除所选控件">删除</button>
          <button type="button" onClick={resetDemo} title="恢复内置演示">重置</button>
        </div>
        <div className="toolbar-spacer" />
        <button type="button" className="window-tool" onClick={() => getCurrentWebviewWindow().hide()} title="隐藏工作台">×</button>
      </header>

      <nav className="workbench-workflow-strip" aria-label="UI 工具链进度">
        {(workflowStages.length ? workflowStages : [
          "来源", "UI Tree", "视觉稿", "分层", "Workbench", "UMG", "逻辑", "验收",
        ].map((label, index) => ({
          id: `empty-${index}`,
          index: index + 1,
          label,
          status: "not_started",
          statusLabel: "未开始",
        }))).map((stage) => (
          <span
            key={stage.id}
            className={`workbench-workflow-stage status-${stage.status}`}
            title={`${stage.label} · ${stage.statusLabel}`}
          >
            <i>{stage.index}</i>
            <strong>{stage.label}</strong>
            <small>{stage.statusLabel}</small>
          </span>
        ))}
      </nav>

      <section className={`workbench-layout ${galleryOpen ? "gallery-visible" : ""}`}>
        <nav className="page-navigation panel-shell" aria-label="UI 页面导航">
          <div className="panel-heading"><strong>UI 页面</strong><span>{pageNavRows.length}</span></div>
          <div className="page-navigation-list">
            {pageNavRows.length === 0 ? <div className="page-navigation-empty">生成或导入 UI 后会显示在这里</div> : pageNavRows.map((page) => (
              <button
                type="button"
                key={page.pageId}
                className={`page-navigation-item ${page.selected ? "selected" : ""}`}
                disabled={!page.available}
                onClick={() => void selectPersistedPage(page.pageId)}
                title={page.available ? page.title : `${page.title} · 会话文件不可用`}
              >
                <span className="page-thumbnail">
                  {page.thumbnailUrl ? <img src={page.thumbnailUrl} alt="" /> : <span>UI</span>}
                </span>
                <span className="page-navigation-copy">
                  <strong>{page.title}</strong>
                  <small>{page.available ? page.controlCountLabel : "文件不可用"}</small>
                </span>
                <span className="page-selected-indicator" aria-hidden="true" />
              </button>
            ))}
          </div>
        </nav>

        <aside className="hierarchy-panel panel-shell">
          <div className="panel-heading"><strong>控件层级</strong><span>{tree.nodes.length}</span></div>
          <div className="panel-search"><span>⌕</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索控件" /></div>
          <div ref={treeListRef} className="tree-list">
            <div className="tree-row tree-root-row">
              <button type="button" className="tree-toggle" onClick={() => toggleLayer("__root__")} title="展开或折叠全部图层">{collapsedLayers.has("__root__") ? "▸" : "▾"}</button>
              <span className="node-kind kind-canvas">C</span>
              <span className="tree-name">CanvasRoot</span>
            </div>
            {layerRows.map(({ node, depth, hasChildren }) => (
              <div key={node.id} data-tree-node-id={node.id} className={`tree-row ${node.id === selectedId ? "selected" : ""} ${!effectiveVisible(node, tree.nodes) ? "muted" : ""}`} onClick={() => setSelectedId(node.id)}>
                <span className="tree-indent" style={{ width: Math.min(84, depth * 14) }} />
                {hasChildren ? <button type="button" className="tree-toggle" onClick={(event) => { event.stopPropagation(); toggleLayer(node.id); }} title="展开或折叠子图层">{collapsedLayers.has(node.id) ? "▸" : "▾"}</button> : <span className="tree-toggle-spacer" />}
                <span className={`node-kind kind-${node.category}`}>{node.category.slice(0, 1).toUpperCase()}</span>
                <span className="tree-name" title={node.id}>{node.name ?? node.id}</span>
                <button type="button" className={node.visible === false ? "off" : ""} onClick={(event) => { event.stopPropagation(); patchNode(node.id, { visible: node.visible === false }); }} title="显示或隐藏">{node.visible === false ? "○" : "●"}</button>
                <button type="button" className={node.locked ? "on" : ""} onClick={(event) => { event.stopPropagation(); patchNode(node.id, { locked: !node.locked }); }} title="锁定或解锁">{node.locked ? "◆" : "◇"}</button>
              </div>
            ))}
          </div>
          <div className="panel-footer"><button type="button" onClick={() => addNode(null)} title="在 CanvasRoot 下新建范围">＋ 根项</button><button type="button" onClick={() => selected && addNode(selected)} disabled={!canOwnChildren(selected)} title="在当前控件下添加子项">＋ 子项</button><button type="button" onClick={duplicateSelected} disabled={!selected}>复制</button></div>
        </aside>

        <section className="canvas-column">
          <div className="canvas-toolbar">
            <div className="tool-toggle">
              <button type="button" className={moveScope === "layer" ? "active" : ""} onClick={() => setMoveScope("layer")} title="只移动当前图层">单层</button>
              <button type="button" className={moveScope === "group" ? "active" : ""} onClick={() => setMoveScope("group")} title="移动当前图层及全部子层">整组</button>
              <button type="button" className={gridVisible ? "active" : ""} onClick={() => setGridVisible((value) => !value)}>网格</button>
              <button type="button" className={snap ? "active" : ""} onClick={() => setSnap((value) => !value)} title="开启时按住 Alt 可临时进行 1px 微调">吸附 4px</button>
              <button type="button" className={referenceVisible ? "active" : ""} onClick={() => setReferenceVisible((value) => !value)}>参考底图</button>
              <input className="reference-opacity" aria-label="参考底图透明度" type="range" min="0.15" max="1" step="0.05" value={referenceOpacity} onChange={(event) => setReferenceOpacity(Number(event.target.value))} />
            </div>
            <div className="zoom-tools" title="在画布上滚动鼠标滚轮缩放"><button type="button" onClick={() => setZoom((value) => clamp(value - 10, 10, 400))}>−</button><input aria-label="画布缩放" type="range" min="10" max="400" value={zoom} onChange={(event) => setZoom(Number(event.target.value))} /><button type="button" onClick={() => setZoom((value) => clamp(value + 10, 10, 400))}>＋</button><span>{zoom}%</span><button type="button" onClick={() => fitCanvas()}>适配</button></div>
          </div>
          <div
            className={`canvas-viewport ${gridVisible ? "show-grid" : ""} ${panning ? "panning" : ""}`}
            ref={viewportRef}
            title="按住鼠标中键拖动画板"
            onPointerMove={handleCanvasPointerMove}
            onPointerUp={endInteraction}
            onPointerCancel={endInteraction}
            onAuxClick={(event) => event.preventDefault()}
          >
            <div
              ref={stageRef}
              className="canvas-stage-frame"
              style={{
                width: tree.page_size.width * scale,
                height: tree.page_size.height * scale,
                left: "50%",
                top: "50%",
                transform: `translate(-50%, -50%) translate(${canvasPan.x}px, ${canvasPan.y}px)`,
              }}
            >
              <div
                className="canvas-stage"
                style={{
                  width: tree.page_size.width,
                  height: tree.page_size.height,
                  transform: `scale(${scale})`,
                }}
                onPointerDown={(event) => { if (event.button === 0) setSelectedId(""); }}
              >
                {referenceVisible && <img className="reference-layer" src={imageUrl} alt="UI source reference" draggable={false} style={{ opacity: referenceOpacity }} />}
                {referenceVisible && <span className="reference-badge">锁定参考底图 · 不参与资产导出</span>}
                {renderRows.map(({ node }) => {
                  const cleanUrl = visualUrl(node, "clean");
                  if (!effectiveVisible(node, tree.nodes) || !cleanUrl || !node.reusable_bitmap || (node.node_kind !== "skin" && node.node_kind !== "artwork")) return null;
                  return <img key={`asset-${node.id}`} className="clean-asset-layer" data-asset-id={node.id} src={cleanUrl} alt="" draggable={false} style={{ left: node.bounds.x, top: node.bounds.y, width: node.bounds.width, height: node.bounds.height, opacity: effectiveOpacity(node, tree.nodes), zIndex: 100 + (layerOrder.get(node.id) ?? 0) }} />;
                })}
                {renderRows.map(({ node }) => {
                  const previewUrl = node.node_kind === "native" ? visualUrl(node, "clean") : null;
                  if (!effectiveVisible(node, tree.nodes) || !previewUrl || node.render_mode === "hidden") return null;
                  return <img key={"native-preview-" + node.id} className="native-preview-layer" data-asset-id={node.id} src={previewUrl} alt="" draggable={false} style={{ left: node.bounds.x, top: node.bounds.y, width: node.bounds.width, height: node.bounds.height, opacity: effectiveOpacity(node, tree.nodes), zIndex: 900 + (layerOrder.get(node.id) ?? 0) }} />;
                })}
                {renderRows.map(({ node }) => {
                  if (!effectiveVisible(node, tree.nodes) || node.render_mode === "hidden") return null;
                  const moved = Boolean(node.source_bounds && !sameBounds(node.bounds, node.source_bounds));
                  const parent = node.parent_id ? nodeById.get(node.parent_id) : null;
                  const parentMoved = Boolean(parent?.source_bounds && !sameBounds(parent.bounds, parent.source_bounds));
                  const sourcePreview = node.node_kind !== "native" && moved && !parentMoved && node.source_bounds && !visualUrl(node, "clean");
                  return (
                    <div
                      key={node.id}
                      data-node-id={node.id}
                      className={`canvas-node kind-${node.node_kind} render-${node.render_mode} ${node.id === selectedId ? "selected" : ""} ${node.locked ? "locked" : ""} ${node.derived_from ? "derived-layer" : ""} ${moved ? "moved" : ""}`}
                      style={{
                        left: node.bounds.x,
                        top: node.bounds.y,
                        width: node.bounds.width,
                        height: node.bounds.height,
                        zIndex: node.id === selectedId && node.node_kind !== "composite" ? 5000 : 1000 + (layerOrder.get(node.id) ?? 0),
                        pointerEvents: node.derived_from && node.id !== selectedId ? "none" : undefined,
                      }}
                      onPointerDown={(event) => beginInteraction(event, node, "drag")}
                    >
                      {sourcePreview && <span className="moved-source-preview" style={sourceCropCanvasStyle(imageUrl, tree.page_size, node.source_bounds!, node.bounds)} />}
                      {node.node_kind === "native" && node.id === "progress.tower_exp" && <span className="native-progress"><i /></span>}
                      {node.node_kind === "native" && nativeWorkbenchDisplayText(node) && <span className={`native-text-content ${nativeDisplayClass(node)}`} style={nativeWorkbenchTextCss(node.text_style ?? nativeWorkbenchCloseTextStyle(node), nativeFontSize(node))}>{nativeWorkbenchDisplayText(node)}</span>}
                      <span className="canvas-node-label">{node.node_kind} · {node.id}{moved ? " · 已移动" : ""}</span>
                      {node.id === selectedId && !node.locked && ["nw", "ne", "sw", "se"].map((handle) => <span key={handle} className={`resize-handle handle-${handle}`} onPointerDown={(event) => beginInteraction(event, node, "resize", handle)} />)}
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
          <div className="canvas-status"><span>{notice}</span><span>{tree.page_size.width} × {tree.page_size.height}</span><span>{selected ? `${Math.round(selected.bounds.x)}, ${Math.round(selected.bounds.y)} · ${Math.round(selected.bounds.width)} × ${Math.round(selected.bounds.height)}` : "未选择控件"}</span></div>
        </section>

        <aside className="inspector-panel panel-shell">
          <div className="panel-heading"><strong>细节</strong><span>{selected?.category ?? "无选择"}</span></div>
          {!selected ? <div className="empty-state">从层级或画布中选择一个控件</div> : <>
            <InspectorSection title="标识">
              <Field label="控件 ID"><input value={selected.id} onChange={(event) => renameSelected(event.target.value)} /></Field>
              <Field label="分类"><input value={selected.category} onChange={(event) => patchSelected({ category: event.target.value })} /></Field>
              <Field label="父级图层"><select value={selected.parent_id ?? ""} onChange={(event) => setSelectedParent(event.target.value)}><option value="">CanvasRoot</option>{tree.nodes.filter((node) => node.id !== selected.id && !descendantIds(tree.nodes, selected.id).includes(node.id)).map((node) => <option key={node.id} value={node.id}>{node.id}</option>)}</select></Field>
              <Field label="目标组件"><input value={selected.extraction.target_component_id} onChange={(event) => patchExtraction({ target_component_id: event.target.value })} /></Field>
            </InspectorSection>
            <InspectorSection title="组件语义">
              <Field label="Node Kind"><select value={selected.node_kind} onChange={(event) => patchNodeKind(event.target.value as NodeKind)}>{Object.entries(NODE_KIND_LABELS).map(([kind, label]) => <option key={kind} value={kind}>{label}</option>)}</select></Field>
              <Field label="Render Mode"><select value={selected.render_mode} onChange={(event) => patchSelected({ render_mode: event.target.value as RenderMode })}>{Object.entries(RENDER_MODE_LABELS).map(([mode, label]) => <option key={mode} value={mode}>{label}</option>)}</select></Field>
              <div className="semantic-grid">
                <span><small>Children</small><strong>{directChildren.get(selected.id)?.length ?? 0}</strong></span>
                <span><small>Reusable Bitmap</small><strong>{selected.reusable_bitmap ? "Yes" : "No"}</strong></span>
                <span><small>Component Reuse</small><strong>{selected.component_reuse?.status === "ready" ? "Ready" : "No"}</strong></span>
                <span><small>Cleanup</small><strong>{selected.review?.cleanup_status ?? "not_applicable"}</strong></span>
                <span><small>Assembly</small><strong>{selected.visual_assets?.assembly_preview ? "Available" : "Missing"}</strong></span>
              </div>
              {selected.component_reuse?.status === "ready" && <Field label="Library Asset"><input value={selected.component_reuse.source_asset} readOnly /></Field>}
              <div className="visual-tabs" role="tablist" aria-label="资产查看模式">
                {(["source", "clean", "assembly"] as VisualMode[]).map((mode) => <button type="button" key={mode} className={visualMode === mode ? "active" : ""} onClick={() => setVisualMode(mode)}>{mode === "source" ? "Source" : mode === "clean" ? "Clean" : "Assembly"}</button>)}
              </div>
              <div className="visual-preview" data-visual-mode={visualMode}>
                {visualMode === "source" && (visualUrl(selected, "source")
                  ? <img src={visualUrl(selected, "source")!} alt={`${selected.id} source crop`} />
                  : <span className="preview-crop" style={cropStyle(imageUrl, tree.page_size, selected.source_bounds ?? selected.bounds, 282, 150)} />)}
                {visualMode === "clean" && (visualUrl(selected, "clean")
                  ? <img src={visualUrl(selected, "clean")!} alt={`${selected.id} clean asset`} />
                  : <div className="preview-empty"><strong>Clean asset not generated</strong><span>Source Crop 不能作为正式可复用资产。</span></div>)}
                {visualMode === "assembly" && (visualUrl(selected, "assembly")
                  ? <img src={visualUrl(selected, "assembly")!} alt={`${selected.id} assembly preview`} />
                  : <div className="preview-empty"><strong>Assembly preview not generated</strong><span>载入重组预览后可与原图对照。</span></div>)}
              </div>
              {(selected.node_kind === "skin" || selected.node_kind === "composite") && <button type="button" className="cleanup-action" onClick={queueSelectedCleanup} disabled={selected.node_kind === "skin" && selected.review?.cleanup_status === "clean"}>{selected.node_kind === "composite" ? "净化母版背景" : "净化母版"}</button>}
            </InspectorSection>
            <InspectorSection title="插槽（画布面板槽）">
              <div className="field-grid four"><NumberField label="位置 X" value={selected.bounds.x} onChange={(value) => patchBounds("x", value)} /><NumberField label="位置 Y" value={selected.bounds.y} onChange={(value) => patchBounds("y", value)} /><NumberField label="尺寸 X" value={selected.bounds.width} min={1} onChange={(value) => patchBounds("width", value)} /><NumberField label="尺寸 Y" value={selected.bounds.height} min={1} onChange={(value) => patchBounds("height", value)} /></div>
              <div className="field-grid two"><NumberField label="ZOrder" value={selected.z_index ?? 0} onChange={(value) => patchSelected({ z_index: value })} /><NumberField label="透明度" value={selected.opacity ?? 1} min={0} max={1} step={0.05} onChange={(value) => patchSelected({ opacity: clamp(value, 0, 1) })} /></div>
              {selected.source_bounds && !sameBounds(selected.bounds, selected.source_bounds) && <div className="layout-source-note"><span>原始切图 {Math.round(selected.source_bounds.x)}, {Math.round(selected.source_bounds.y)} · {Math.round(selected.source_bounds.width)} × {Math.round(selected.source_bounds.height)}</span><button type="button" onClick={resetSelectedLayout}>归位</button></div>}
            </InspectorSection>
            {selectedSupportsText && <InspectorSection title="文字外观（TextBlock）">
              <Field label="预览文字"><input value={selected.display_text ?? nativeWorkbenchDisplayText(selected)} onChange={(event) => patchSelected({ display_text: event.target.value })} /></Field>
              <div className="field-grid two">
                <NumberField label="字号" value={selected.text_style?.font_size ?? nativeFontSize(selected)} min={1} max={256} onChange={(value) => patchSelectedTextStyle({ font_size: clamp(value, 1, 256) })} />
                <NumberField label="描边" value={selected.text_style?.outline_size ?? nativeTextDefaultOutlineSize(selected)} min={0} max={12} step={0.5} onChange={(value) => patchSelectedTextStyle({ outline_size: clamp(value, 0, 12) })} />
              </div>
              <ColorField label="文字颜色" value={selected.text_style?.color ?? nativeTextDefaultColor(selected)} onChange={(value) => patchSelectedTextStyle({ color: value })} />
              <ColorField label="描边颜色" value={selected.text_style?.outline_color ?? nativeTextDefaultOutlineColor(selected)} onChange={(value) => patchSelectedTextStyle({ outline_color: value })} />
              <div className="field-grid two">
                <NumberField label="阴影 X" value={selected.text_style?.shadow_offset_x ?? 0} min={-32} max={32} step={0.5} onChange={(value) => patchSelectedTextStyle({ shadow_offset_x: clamp(value, -32, 32) })} />
                <NumberField label="阴影 Y" value={selected.text_style?.shadow_offset_y ?? 0} min={-32} max={32} step={0.5} onChange={(value) => patchSelectedTextStyle({ shadow_offset_y: clamp(value, -32, 32) })} />
              </div>
              <ColorField label="阴影颜色" value={selected.text_style?.shadow_color ?? "#180c06"} onChange={(value) => patchSelectedTextStyle({ shadow_color: value })} />
              <SegmentedField label="水平对齐" value={selected.text_style?.horizontal_alignment ?? "center"} options={[{ value: "left", label: "左" }, { value: "center", label: "中" }, { value: "right", label: "右" }]} onChange={(value) => patchSelectedTextStyle({ horizontal_alignment: value as WorkbenchTextStyle["horizontal_alignment"] })} />
              <SegmentedField label="垂直对齐" value={selected.text_style?.vertical_alignment ?? "middle"} options={[{ value: "top", label: "上" }, { value: "middle", label: "中" }, { value: "bottom", label: "下" }]} onChange={(value) => patchSelectedTextStyle({ vertical_alignment: value as WorkbenchTextStyle["vertical_alignment"] })} />
              <ToggleField label="自动换行" checked={selected.text_style?.auto_wrap ?? false} onChange={(checked) => patchSelectedTextStyle({ auto_wrap: checked })} />
            </InspectorSection>}
            <InspectorSection title="切图策略">
              <Field label="处理方式"><select value={selected.extraction.mode} onChange={(event) => patchExtraction({ mode: event.target.value as ExtractionMode })}>{Object.entries(MODE_LABELS).map(([mode, label]) => <option key={mode} value={mode}>{label}</option>)}</select></Field>
              <Field label="置信度"><input type="range" min="0" max="1" step="0.01" value={selected.extraction.confidence ?? 0.5} onChange={(event) => patchExtraction({ confidence: Number(event.target.value) })} /><output>{Math.round((selected.extraction.confidence ?? 0.5) * 100)}%</output></Field>
              <Field label="判断说明"><textarea value={selected.extraction.reason ?? ""} onChange={(event) => patchExtraction({ reason: event.target.value })} rows={3} /></Field>
              {selected.extraction.mode === "native" && <p className="native-note">文本、价格、计数、进度和点击区域保留为原生控件，不会烘焙进图片。</p>}
              {selected.extraction.mode === "reconstruct_skin" && <><ToggleField label="透明背景" checked={selected.extraction.transparent ?? false} onChange={(checked) => patchExtraction({ transparent: checked })} /><ToggleField label="评估九宫格" checked={selected.extraction.evaluate_nine_slice ?? false} onChange={(checked) => patchExtraction({ evaluate_nine_slice: checked })} /></>}
            </InspectorSection>
            <InspectorSection title="行为">
              <ToggleField label="可见" checked={selected.visible !== false} onChange={(checked) => patchSelected({ visible: checked })} />
              <ToggleField label="锁定" checked={selected.locked ?? false} onChange={(checked) => patchSelected({ locked: checked })} />
            </InspectorSection>
          </>}
        </aside>

        <section className="slice-gallery">
          <div className="gallery-heading"><div className="gallery-tabs"><button type="button" className={galleryTab === "assets" ? "active" : ""} onClick={() => selectGalleryTab("assets")}>Assets</button><button type="button" className={galleryTab === "structure" ? "active" : ""} onClick={() => selectGalleryTab("structure")}>Structure</button><span>{galleryTab === "assets" ? "仅显示可复用 Skin / Artwork" : "逻辑父级与原生控件"}</span></div><div className="gallery-actions"><span>{galleryNodes.length} / {tree.nodes.length}</span><button type="button" onClick={() => setGalleryOpen(false)}>收起</button></div></div>
          <div className="gallery-filters">{(Object.keys(GALLERY_FILTER_LABELS) as GalleryFilter[]).map((filter) => <button type="button" key={filter} className={galleryFilter === filter ? "active" : ""} onClick={() => setGalleryFilter(filter)}>{GALLERY_FILTER_LABELS[filter]}</button>)}</div>
          <div className="gallery-strip">
            {galleryNodes.length === 0 && <div className="gallery-empty">当前筛选没有节点</div>}
            {galleryNodes.map((node) => galleryTab === "assets" ? (
              <button type="button" key={node.id} className={`slice-card asset-card ${node.id === selectedId ? "selected" : ""}`} onClick={() => setSelectedId(node.id)}>
                <span className="crop-checker">
                  {visualUrl(node, "clean") && node.reusable_bitmap
                    ? <img className="asset-thumbnail" src={visualUrl(node, "clean")!} alt="" />
                    : <><span className="crop-image" style={cropStyle(imageUrl, tree.page_size, node.source_bounds ?? node.bounds, 90, 70)} /><em className="source-badge">SOURCE / {node.review?.cleanup_status === "requested" ? "已排队" : "待净化"}</em></>}
                </span>
                <span className="slice-meta"><strong>{node.id}</strong><small>{NODE_KIND_LABELS[node.node_kind ?? "artwork"]} · {node.reusable_bitmap ? "Ready" : node.review?.cleanup_status ?? "Needs Cleanup"}</small></span>
              </button>
            ) : (
              <button type="button" key={node.id} className={`structure-card kind-${node.node_kind} ${node.id === selectedId ? "selected" : ""}`} onClick={() => setSelectedId(node.id)}>
                <span className="structure-symbol">{node.node_kind === "composite" ? "C" : "N"}</span>
                <span><strong>{node.id}</strong><small>{NODE_KIND_LABELS[node.node_kind ?? "native"]} · {directChildren.get(node.id)?.length ?? 0} children · {RENDER_MODE_LABELS[node.render_mode ?? "outline"]}</small></span>
              </button>
            ))}
          </div>
        </section>
        {!galleryOpen && <button className="gallery-restore" type="button" onClick={() => setGalleryOpen(true)}>显示资产与结构</button>}
      </section>

      {deliveryDialog && (
        <div className="delivery-dialog-backdrop" role="presentation" onPointerDown={() => !deliveryBusy && setDeliveryDialog(null)}>
          <section
            className="delivery-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="delivery-dialog-title"
            onPointerDown={(event) => event.stopPropagation()}
          >
            <header>
              <div>
                <small>EDITOR DELIVERY</small>
                <h2 id="delivery-dialog-title">确认并交付到编辑器</h2>
              </div>
              <button type="button" className="window-tool" onClick={() => setDeliveryDialog(null)} disabled={deliveryBusy} title="关闭">×</button>
            </header>
            <dl className="delivery-summary">
              <div><dt>UI 页面</dt><dd>{deliveryDialog.task.title}</dd></div>
              <div><dt>控件数量</dt><dd>{tree.nodes.length}</dd></div>
              <div><dt>来源任务</dt><dd>{deliveryDialog.task.agent_context?.provider} · {deliveryDialog.task.agent_context?.thread_id}</dd></div>
              <div><dt>执行方式</dt><dd>新建 Codex 任务</dd></div>
            </dl>
            <label>
              <span>项目工作区</span>
              <input
                value={deliveryDialog.projectWorkspace}
                onChange={(event) => setDeliveryDialog({
                  ...deliveryDialog,
                  projectWorkspace: event.target.value,
                  candidates: [],
                  selectedLoadPath: "",
                  preflight: null,
                  state: "idle",
                  message: "工作区已变化，请重新搜索并预检",
                })}
                disabled={deliveryBusy}
              />
            </label>
            <section className="delivery-target-selector">
              <div className="delivery-target-search">
                <label>
                  <span>目标 WidgetBlueprint</span>
                  <input
                    value={deliveryDialog.query}
                    onChange={(event) => setDeliveryDialog({
                      ...deliveryDialog,
                      query: event.target.value,
                      candidates: [],
                      selectedLoadPath: "",
                      preflight: null,
                      state: "idle",
                      message: "搜索内容已变化，请重新读取编辑器资产",
                    })}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") {
                        event.preventDefault();
                        void searchDeliveryTargets();
                      }
                    }}
                    placeholder="名称或 /RedCliff/Asset/UI/..."
                    disabled={deliveryBusy}
                  />
                </label>
                <button
                  type="button"
                  onClick={() => void searchDeliveryTargets()}
                  disabled={deliveryBusy || deliveryDialog.state === "searching_assets" || deliveryDialog.state === "checking_mcp"}
                >
                  {deliveryDialog.state === "searching_assets" ? "搜索中…" : "搜索"}
                </button>
              </div>
              <div className="delivery-candidate-list" role="listbox" aria-label="WidgetBlueprint 搜索结果">
                {deliveryDialog.candidates.length === 0 && (
                  <div className="delivery-candidate-empty">搜索结果会显示名称、编辑器 load_path 和资产类型</div>
                )}
                {deliveryDialog.candidates.map((candidate) => (
                  <button
                    type="button"
                    role="option"
                    aria-selected={deliveryDialog.selectedLoadPath === candidate.load_path}
                    className={deliveryDialog.selectedLoadPath === candidate.load_path ? "selected" : ""}
                    key={candidate.load_path}
                    onClick={() => void selectDeliveryTarget(candidate)}
                    disabled={deliveryBusy || deliveryDialog.state === "checking_mcp"}
                  >
                    <strong>{candidate.display_name}</strong>
                    <span>{candidate.load_path}</span>
                    <small>{candidate.class_name}</small>
                  </button>
                ))}
              </div>
              <div className={`delivery-preflight status-${deliveryDialog.state}`}>
                <strong>{deliveryDialog.state === "ready" ? "预检通过" : "编辑器预检"}</strong>
                <span>{deliveryDialog.message}</span>
                {deliveryDialog.preflight?.status === "ready" && (
                  <dl>
                    <div><dt>load_path</dt><dd>{deliveryDialog.preflight.selected_load_path}</dd></div>
                    <div><dt>资产类型</dt><dd>{deliveryDialog.preflight.selected_class_name}</dd></div>
                    <div><dt>MCP</dt><dd>{deliveryDialog.preflight.mcp_server_name} {deliveryDialog.preflight.mcp_server_version}</dd></div>
                    <div><dt>检查时间</dt><dd>{new Date(deliveryDialog.preflight.checked_at_unix_ms).toLocaleString()}</dd></div>
                  </dl>
                )}
              </div>
            </section>
            <p>本次确认只授权实现当前冻结 UI Tree 对应的 WidgetBlueprint，不包含无关 Lua、DataTable、关卡或玩法修改。</p>
            <footer>
              <button type="button" onClick={() => setDeliveryDialog(null)} disabled={deliveryBusy}>取消</button>
              <button type="button" className="delivery-action" onClick={confirmAndDeliver} disabled={deliveryBusy || !deliveryReady}>
                {deliveryBusy ? "正在创建任务…" : "确认并在新任务中实现"}
              </button>
            </footer>
          </section>
        </div>
      )}
    </main>
  );
}

function cropStyle(imageUrl: string, page: UITree["page_size"], bounds: Bounds, width = 150, height = 84) {
  const scale = Math.min(width / bounds.width, height / bounds.height);
  return {
    backgroundImage: `url("${imageUrl}")`,
    backgroundSize: `${page.width * scale}px ${page.height * scale}px`,
    backgroundPosition: `${-bounds.x * scale}px ${-bounds.y * scale}px`,
    width: bounds.width * scale,
    height: bounds.height * scale,
  };
}

function sourceCropCanvasStyle(imageUrl: string, page: UITree["page_size"], source: Bounds, target: Bounds) {
  const scaleX = target.width / source.width;
  const scaleY = target.height / source.height;
  return {
    backgroundImage: `url("${imageUrl}")`,
    backgroundSize: `${page.width * scaleX}px ${page.height * scaleY}px`,
    backgroundPosition: `${-source.x * scaleX}px ${-source.y * scaleY}px`,
  };
}

function InspectorSection({ title, children }: { title: string; children: React.ReactNode }) {
  return <section className="inspector-section"><h2>{title}</h2>{children}</section>;
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="inspector-field"><span>{label}</span><div>{children}</div></label>;
}

function NumberField({ label, value, onChange, min, max, step }: { label: string; value: number; onChange: (value: number) => void; min?: number; max?: number; step?: number }) {
  return <label className="number-field"><span>{label}</span><input type="number" value={Math.round(value * 100) / 100} min={min} max={max} step={step} onChange={(event) => onChange(Number(event.target.value))} /></label>;
}

function ToggleField({ label, checked, onChange }: { label: string; checked: boolean; onChange: (checked: boolean) => void }) {
  return <label className="toggle-field"><span>{label}</span><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /></label>;
}

function ColorField({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  const swatch = /^#[0-9a-f]{6}$/i.test(value) ? value : "#ffffff";
  return <Field label={label}><input className="color-swatch" type="color" value={swatch} onChange={(event) => onChange(event.target.value)} /><input className="color-value" value={value} onChange={(event) => onChange(event.target.value)} /></Field>;
}

function SegmentedField({ label, value, options, onChange }: { label: string; value: string; options: Array<{ value: string; label: string }>; onChange: (value: string) => void }) {
  return <Field label={label}><span className="segmented-control">{options.map((option) => <button type="button" key={option.value} className={value === option.value ? "active" : ""} onClick={() => onChange(option.value)}>{option.label}</button>)}</span></Field>;
}
