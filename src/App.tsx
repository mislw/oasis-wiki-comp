import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import Ball from "./windows/Ball";
import Settings from "./windows/Settings";

// A single Vite entry serves both windows; route by the Tauri window label.
export default function App() {
  const label = getCurrentWebviewWindow().label;
  if (label === "ball") return <Ball />;
  return <Settings />;
}
