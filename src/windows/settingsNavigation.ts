export type SettingsPage =
  | { kind: "overview" }
  | { kind: "agent"; agentId: string };

export function settingsOverviewPage(): SettingsPage {
  return { kind: "overview" };
}

export function selectSettingsPageForAgent(
  agentId: string,
  knownAgentIds: string[],
): SettingsPage {
  if (!knownAgentIds.includes(agentId)) {
    throw new Error(`unknown Agent target: ${agentId}`);
  }
  return { kind: "agent", agentId };
}
