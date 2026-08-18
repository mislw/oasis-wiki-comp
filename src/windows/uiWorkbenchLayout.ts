export type WorkbenchLayoutTree = {
  page_size: { width: number; height: number };
  nodes: Record<string, unknown>[];
};

export type PersistedLayoutState = {
  revision: number;
  fingerprint: string;
};

export type WorkbenchLayoutSaveState = {
  dirty: boolean;
  label: string;
  revision: number | null;
};

function stableValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, child]) => [key, stableValue(child)]),
    );
  }
  return value;
}

export function workbenchLayoutProjection(tree: WorkbenchLayoutTree) {
  return stableValue({
    page_size: tree.page_size,
    nodes: tree.nodes,
  }) as WorkbenchLayoutTree;
}

export function workbenchLayoutFingerprint(tree: WorkbenchLayoutTree) {
  return JSON.stringify(workbenchLayoutProjection(tree));
}

export function workbenchLayoutSaveState(
  currentFingerprint: string,
  persisted: PersistedLayoutState | null,
): WorkbenchLayoutSaveState {
  const dirty = persisted?.fingerprint !== currentFingerprint;
  return {
    dirty,
    label: dirty ? "未保存" : `已保存 v${persisted.revision}`,
    revision: persisted?.revision ?? null,
  };
}
