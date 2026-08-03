import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { BallState } from "../types";

export default function Ball() {
  const [state, setState] = useState<BallState>("idle");

  useEffect(() => {
    invoke<BallState>("get_ball_state").then(setState).catch(() => {});
    const unlisten = listen<BallState>("ball://state", (event) => setState(event.payload));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <div
      className={`ball ball-${state}`}
      data-tauri-drag-region
      onClick={() => invoke("open_settings")}
      title="Oasis Companion - 点击打开设置"
    >
      <div className="ball-core" data-tauri-drag-region />
      {state === "error" && <span className="ball-badge ball-badge-err">!</span>}
    </div>
  );
}
