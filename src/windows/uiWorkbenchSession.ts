/** Loaded data and URL roots for an external UI Workbench session. */
export type LoadedWorkbenchSession = {
  raw: Record<string, unknown>;
  sourceImageUrl: string;
  assetBaseUrl: string;
};

/** Navigation metadata returned by the Companion page catalog. */
export type WorkbenchPageSummary = {
  page_id: string;
  title: string;
  control_count: number;
  updated_at_unix_ms?: number;
  thumbnail_data_url?: string | null;
  available: boolean;
};

/** Persisted page catalog returned by the Companion backend. */
export type WorkbenchCatalog = {
  selected_page_id: string | null;
  pages: WorkbenchPageSummary[];
};

/** Persisted session payload returned by the Companion backend. */
export type PersistedWorkbenchPagePayload = {
  page_id: string;
  title: string;
  control_count: number;
  source_image: string;
  session: Record<string, unknown>;
};

/** Loaded persisted page with its source image ready for browser display. */
export type LoadedPersistedWorkbenchPage = {
  pageId: string;
  title: string;
  controlCount: number;
  raw: Record<string, unknown>;
  sourceImageUrl: string;
};

/** Stable presentation fields for one page-navigation row. */
export type WorkbenchPageNavRow = {
  pageId: string;
  title: string;
  controlCountLabel: string;
  selected: boolean;
  available: boolean;
  thumbnailUrl: string | null;
};

type WorkbenchSessionConnection = {
  getPendingUrl: () => Promise<string | null>;
  subscribe: (handler: (url: string) => void) => Promise<() => void>;
  loadExternal: (url: string) => Promise<void>;
  loadFallback: () => Promise<void>;
};

type NativeWorkbenchNode = {
  id: string;
  category: string;
  display_text?: string;
  content_hint?: string;
  extraction?: { target_component_id?: string };
  visual_assets?: { native_preview?: string | null };
};

type WorkbenchLayerNode = {
  id: string;
  parent_id?: string;
};

/** Editable native text appearance aligned with the UMG TextBlock property groups. */
export type WorkbenchTextStyle = {
  font_size?: number;
  color?: string;
  outline_color?: string;
  outline_size?: number;
  shadow_color?: string;
  shadow_offset_x?: number;
  shadow_offset_y?: number;
  horizontal_alignment?: "left" | "center" | "right";
  vertical_alignment?: "top" | "middle" | "bottom";
  auto_wrap?: boolean;
};

type WorkbenchTextCss = {
  fontSize: string;
  color?: string;
  textShadow?: string;
  justifyContent?: "flex-start" | "center" | "flex-end";
  alignItems?: "flex-start" | "center" | "flex-end";
  textAlign?: "left" | "center" | "right";
  whiteSpace?: "normal" | "nowrap";
  overflowWrap?: "anywhere";
};

const LEGACY_NATIVE_DISPLAY_TEXT: Record<string, string> = {
  "header.title": "城防",
  "text.tab.defence_tower": "防御塔",
  "text.tab.wall": "城墙",
  "text.plan_label": "方案",
  "text.plan.1": "1",
  "text.plan.2": "2",
  "text.plan.3": "3",
  "badge.stage.1": "1",
  "badge.stage.2": "2",
  "badge.stage.3": "3",
  "badge.stage.4": "4",
  "badge.stage.5": "5",
  "text.manage_building": "管理建筑",
  "text.tower_name": "箭塔",
  "badge.tag.ranged": "远程",
  "badge.tag.single": "单体",
  "badge.tag.physical": "物理",
  "text.tower_level": "等级 1",
  "text.tower_exp": "0/100",
  "stat.attack": "攻击 120",
  "stat.health": "生命 1200",
  "stat.attack_speed": "攻速 1.0",
  "stat.critical": "暴击 5%",
  "stat.range": "范围 4",
  "stat.damage": "伤害 120",
  "text.effects_title": "升级效果",
  "badge.effect.1": "1",
  "badge.effect.2": "2",
  "badge.effect.3": "3",
  "badge.effect.4": "4",
  "text.effect.1": "攻击提升",
  "text.effect.2": "射程提升",
  "text.effect.3": "攻速提升",
  "text.effect.4": "解锁强化",
  "value.effect.1": "+10%",
  "value.effect.2": "+1",
  "value.effect.3": "+5%",
  "value.effect.4": "可用",
  "status.effect.1": "已解锁",
  "status.effect.2": "已解锁",
  "status.effect.3": "未解锁",
  "status.effect.4": "未解锁",
  "counter.resource.attack": "20",
  "counter.resource.range": "15",
  "counter.resource.gold": "300",
  "owned.resource.attack": "拥有 120",
  "owned.resource.range": "拥有 80",
  "owned.resource.gold": "拥有 1800",
  "text.upgrade": "升级",
  "text.currency.title": "货币兑换",
  "text.currency.element_title": "元素兑换",
  "text.currency.name_element_01": "一堆元素",
  "counter.currency.amount_element_01": "17000",
  "counter.currency.discount_element_01": "70%",
  "text.currency.name_element_02": "一袋元素",
  "counter.currency.amount_element_02": "82500",
  "counter.currency.discount_element_02": "65%",
  "text.currency.name_element_03": "一箱元素",
  "counter.currency.amount_element_03": "150000",
  "counter.currency.discount_element_03": "50%",
  "text.currency.jade_title": "龙玉兑换",
  "text.currency.name_jade_01": "一些龙玉",
  "counter.currency.amount_jade_01": "60",
  "text.currency.name_jade_02": "一堆龙玉",
  "counter.currency.amount_jade_02": "300",
  "text.currency.name_jade_03": "小袋龙玉",
  "counter.currency.amount_jade_03": "980",
};

const CLOSE_BUTTON_TEXT_STYLE: WorkbenchTextStyle = {
  font_size: 30,
  color: "#fff3cf",
  outline_color: "#6b3515",
  outline_size: 2,
  horizontal_alignment: "center",
  vertical_alignment: "middle",
};

function hasComponentToken(value: string | undefined, token: string): boolean {
  return typeof value === "string"
    && value.toLowerCase().split(/[^a-z0-9]+/).includes(token);
}

/** Return the native close-glyph defaults for recognized close-button nodes. */
export function nativeWorkbenchCloseTextStyle(node: NativeWorkbenchNode): WorkbenchTextStyle | undefined {
  if (!(["button", "hit_target"] as string[]).includes(node.category)) return undefined;
  const targetId = node.extraction?.target_component_id;
  return hasComponentToken(node.id, "close") || hasComponentToken(targetId, "close")
    ? { ...CLOSE_BUTTON_TEXT_STYLE }
    : undefined;
}

/** Resolve native preview text from session data before legacy built-in samples. */
export function nativeWorkbenchDisplayText(node: NativeWorkbenchNode): string {
  const explicit = typeof node.display_text === "string" ? node.display_text.trim() : "";
  if (explicit) return explicit;
  const hint = typeof node.content_hint === "string" ? node.content_hint.trim() : "";
  if (hint) return hint;
  const legacy = LEGACY_NATIVE_DISPLAY_TEXT[node.id]
    ?? LEGACY_NATIVE_DISPLAY_TEXT[node.extraction?.target_component_id ?? ""];
  if (legacy) return legacy;
  if (nativeWorkbenchCloseTextStyle(node)) return node.visual_assets?.native_preview ? "" : "×";
  return node.category === "counter" ? "0" : "";
}

function designTextValue(value: number): number {
  return Math.round(value * 1000) / 1000;
}

/** Convert UMG-like text settings into design-space canvas preview CSS. */
export function nativeWorkbenchTextCss(
  style: WorkbenchTextStyle | undefined,
  defaultFontSize: number,
): WorkbenchTextCss {
  const result: WorkbenchTextCss = {
    fontSize: `${style?.font_size === undefined ? defaultFontSize : designTextValue(style.font_size)}px`,
  };
  if (!style) return result;
  if (style.color) result.color = style.color;

  const hasOutline = style.outline_size !== undefined;
  const hasShadow = style.shadow_color !== undefined
    || style.shadow_offset_x !== undefined
    || style.shadow_offset_y !== undefined;
  if (hasOutline || hasShadow) {
    const shadows: string[] = [];
    const outline = designTextValue(Math.max(0, style.outline_size ?? 0));
    if (outline > 0) {
      const color = style.outline_color ?? "#5b351e";
      shadows.push(
        `-${outline}px -${outline}px 0 ${color}`,
        `0 -${outline}px 0 ${color}`,
        `${outline}px -${outline}px 0 ${color}`,
        `-${outline}px 0 0 ${color}`,
        `${outline}px 0 0 ${color}`,
        `-${outline}px ${outline}px 0 ${color}`,
        `0 ${outline}px 0 ${color}`,
        `${outline}px ${outline}px 0 ${color}`,
      );
    }
    if (hasShadow) {
      const x = designTextValue(style.shadow_offset_x ?? 0);
      const y = designTextValue(style.shadow_offset_y ?? 0);
      const blur = Math.max(1, outline);
      shadows.push(`${x}px ${y}px ${blur}px ${style.shadow_color ?? "#000000"}`);
    }
    result.textShadow = shadows.join(", ") || "none";
  }

  if (style.horizontal_alignment) {
    result.justifyContent = style.horizontal_alignment === "left"
      ? "flex-start"
      : style.horizontal_alignment === "right" ? "flex-end" : "center";
    result.textAlign = style.horizontal_alignment;
  }
  if (style.vertical_alignment) {
    result.alignItems = style.vertical_alignment === "top"
      ? "flex-start"
      : style.vertical_alignment === "bottom" ? "flex-end" : "center";
  }
  if (style.auto_wrap !== undefined) {
    result.whiteSpace = style.auto_wrap ? "normal" : "nowrap";
    if (style.auto_wrap) result.overflowWrap = "anywhere";
  }
  return result;
}

/** Return a selected layer's parent IDs from its direct parent through the root. */
export function workbenchLayerAncestorIds(
  nodes: WorkbenchLayerNode[],
  selectedId: string,
): string[] {
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const ancestors: string[] = [];
  const visited = new Set([selectedId]);
  let parentId = byId.get(selectedId)?.parent_id;
  while (parentId && byId.has(parentId) && !visited.has(parentId)) {
    ancestors.push(parentId);
    visited.add(parentId);
    parentId = byId.get(parentId)?.parent_id;
  }
  return ancestors;
}

/** Validate and normalize a loopback HTTP workbench base URL. */
export function normalizeSessionBaseUrl(value: string): string {
  const url = new URL(value);
  if (url.protocol !== "http:") throw new Error("workbench URL must use http");
  if (url.hostname !== "localhost" && url.hostname !== "127.0.0.1") {
    throw new Error("workbench URL must use a loopback host");
  }
  if (!url.port) throw new Error("workbench URL must include an explicit port");
  if (url.username || url.password) throw new Error("workbench URL must not include credentials");
  if (url.search || url.hash) throw new Error("workbench URL must not include a query or fragment");
  if (!url.pathname.endsWith("/")) url.pathname = `${url.pathname}/`;
  return url.href;
}

/** Resolve a session asset without allowing relative traversal above the session root. */
export function resolveSessionAssetUrl(baseUrl: string, assetPath: string): string {
  if (/^(?:data:|blob:|https?:\/\/)/i.test(assetPath)) return assetPath;
  const base = new URL(normalizeSessionBaseUrl(baseUrl));
  const resolved = new URL(assetPath, base);
  if (resolved.origin !== base.origin || !resolved.pathname.startsWith(base.pathname)) {
    throw new Error(`asset path resolves outside the workbench session: ${assetPath}`);
  }
  return resolved.href;
}

/** Fetch a generated workbench session from its loopback server. */
export async function loadWorkbenchSession(
  baseUrl: string,
  fetchImpl: typeof fetch = fetch,
): Promise<LoadedWorkbenchSession> {
  const assetBaseUrl = normalizeSessionBaseUrl(baseUrl);
  const response = await fetchImpl(new URL("session.json", assetBaseUrl), { cache: "no-store" });
  if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
  const raw: unknown = await response.json();
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error("session.json must contain an object");
  }
  const sourceImage = (raw as Record<string, unknown>).source_image;
  if (typeof sourceImage !== "string" || !sourceImage.trim()) {
    throw new Error("session.json must contain a non-empty source_image");
  }
  return {
    raw: raw as Record<string, unknown>,
    sourceImageUrl: resolveSessionAssetUrl(assetBaseUrl, sourceImage),
    assetBaseUrl,
  };
}

/** Retry a loopback session briefly so the server watchdog can replace a crashed worker. */
export async function loadWorkbenchSessionWithRetry(
  baseUrl: string,
  options: {
    attempts?: number;
    delayMs?: number;
    fetchImpl?: typeof fetch;
    wait?: (delayMs: number) => Promise<void>;
  } = {},
): Promise<LoadedWorkbenchSession> {
  const attempts = Math.max(1, Math.trunc(options.attempts ?? 4));
  const delayMs = Math.max(0, options.delayMs ?? 250);
  const wait = options.wait ?? ((delay) => new Promise((resolve) => setTimeout(resolve, delay)));
  let lastError: unknown;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      return await loadWorkbenchSession(baseUrl, options.fetchImpl);
    } catch (error) {
      lastError = error;
      if (attempt < attempts) await wait(delayMs);
    }
  }
  throw lastError;
}

/** Connect initial and forwarded workbench sessions to one loader. */
export async function connectWorkbenchSessions({
  getPendingUrl,
  subscribe,
  loadExternal,
  loadFallback,
}: WorkbenchSessionConnection): Promise<() => void> {
  let eventSeen = false;
  let latestEventUrl: string | null = null;
  const unsubscribe = await subscribe((url) => {
    eventSeen = true;
    latestEventUrl = url;
    void loadExternal(url);
  });
  const pendingUrl = await getPendingUrl();
  if (pendingUrl && pendingUrl !== latestEventUrl) {
    await loadExternal(pendingUrl);
  } else if (!eventSeen) {
    await loadFallback();
  }
  return unsubscribe;
}

/** Choose the persisted selection when available, otherwise the first readable page. */
export function preferredWorkbenchPage(catalog: WorkbenchCatalog): string | null {
  const selected = catalog.pages.find(
    (page) => page.page_id === catalog.selected_page_id && page.available,
  );
  return selected?.page_id ?? catalog.pages.find((page) => page.available)?.page_id ?? null;
}

/** Convert catalog metadata into fixed navigation-row presentation data. */
export function workbenchPageNavRows(catalog: WorkbenchCatalog): WorkbenchPageNavRow[] {
  return catalog.pages.map((page) => ({
    pageId: page.page_id,
    title: page.title,
    controlCountLabel: `${page.control_count} 控件`,
    selected: page.page_id === catalog.selected_page_id,
    available: page.available,
    thumbnailUrl: page.thumbnail_data_url ?? null,
  }));
}

/** Collect relative reusable-asset paths from one persisted session. */
export function collectPersistedAssetPaths(raw: Record<string, unknown>): string[] {
  const nodes = Array.isArray(raw.controls)
    ? raw.controls
    : Array.isArray(raw.nodes)
      ? raw.nodes
      : [];
  const paths = new Set<string>();
  for (const value of nodes) {
    if (!value || typeof value !== "object" || Array.isArray(value)) continue;
    const assets = (value as Record<string, unknown>).visual_assets;
    if (!assets || typeof assets !== "object" || Array.isArray(assets)) continue;
    for (const key of ["clean_layer", "clean_asset", "assembly_preview", "native_preview"] as const) {
      const path = (assets as Record<string, unknown>)[key];
      if (typeof path !== "string" || !path || path === "__source__") continue;
      if (/^(?:[a-z]+:|[\\/])|(?:^|[\\/])\.\.(?:[\\/]|$)/i.test(path)) continue;
      paths.add(path);
    }
  }
  return [...paths];
}

/** Read every available reusable asset while leaving missing assets unresolved. */
export async function loadPersistedAssetUrls(
  pageId: string,
  raw: Record<string, unknown>,
  readAsset: (pageId: string, assetPath: string) => Promise<string>,
): Promise<Map<string, string>> {
  const entries = await Promise.all(
    collectPersistedAssetPaths(raw).map(async (path) => {
      try {
        return [path, await readAsset(pageId, path)] as const;
      } catch {
        return null;
      }
    }),
  );
  return new Map(entries.filter((entry): entry is readonly [string, string] => entry !== null));
}

/** Load one persisted page without retaining data from the previously selected page. */
export async function loadPersistedWorkbenchPage(
  pageId: string,
  backend: {
    loadPage: (pageId: string) => Promise<PersistedWorkbenchPagePayload>;
    readAsset: (pageId: string, assetPath: string) => Promise<string>;
  },
): Promise<LoadedPersistedWorkbenchPage> {
  const loaded = await backend.loadPage(pageId);
  if (loaded.page_id !== pageId) throw new Error("loaded page ID does not match the requested page");
  const sourceImageUrl = await backend.readAsset(pageId, loaded.source_image);
  return {
    pageId: loaded.page_id,
    title: loaded.title,
    controlCount: loaded.control_count,
    raw: loaded.session,
    sourceImageUrl,
  };
}
