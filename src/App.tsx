import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import Ball from "./windows/Ball";
import Settings from "./windows/Settings";
import UIWorkbench from "./windows/UIWorkbench";

// A single Vite entry serves both windows; route by the Tauri window label.
export default function App() {
  const previewWindow = new URLSearchParams(window.location.search).get("window");
  if (previewWindow === "ui-workbench") return <UIWorkbench />;
  const label = getCurrentWebviewWindow().label;
  if (label === "ball") return <Ball />;
  if (label === "ui-workbench") return <UIWorkbench />;
  if (label.startsWith("settings-")) {
    return <Settings agentId={label.slice("settings-".length)} />;
  }
  return <Settings />;
}
