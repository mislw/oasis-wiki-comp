export type ChildOwnerCandidate = {
  category: string;
  node_kind?: string;
  extraction?: { mode?: string };
};

const CHILD_OWNER_CATEGORIES = new Set([
  "panel",
  "button",
  "tabs",
  "container",
  "group",
  "layout",
  "grid",
  "row",
  "card",
  "switcher",
  "artwork",
]);

export function canOwnChildren(node: ChildOwnerCandidate | null | undefined) {
  if (!node || node.node_kind === "native" || node.extraction?.mode === "native") return false;
  return node.node_kind === "composite"
    || node.extraction?.mode === "composite"
    || CHILD_OWNER_CATEGORIES.has(node.category);
}