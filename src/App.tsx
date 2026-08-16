import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import Ball from "./windows/Ball";
import Settings from "./windows/Settings";
import UIWorkbench from "./windows/UIWorkbench";
import UIWorkflow from "./windows/UIWorkflow";

// A single Vite entry serves both windows; route by the Tauri window label.
export default function App() {
  const previewWindow = new URLSearchParams(window.location.search).get("window");
  if (previewWindow === "ui-workbench") return <UIWorkbench />;
  if (previewWindow === "ui-workflow") return <UIWorkflow />;
  const label = getCurrentWebviewWindow().label;
  if (label === "ball") return <Ball />;
  if (label === "ui-workbench") return <UIWorkbench />;
  if (label === "ui-workflow") return <UIWorkflow />;
  return <Settings />;
}
