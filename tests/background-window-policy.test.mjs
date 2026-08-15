import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const detectionSource = await readFile(
  new URL("../src-tauri/src/detection/mod.rs", import.meta.url),
  "utf8",
);
const commandsSource = await readFile(
  new URL("../src-tauri/src/commands.rs", import.meta.url),
  "utf8",
);

test("background Agent detection never opens a settings window", () => {
  assert.doesNotMatch(detectionSource, /show_agent_settings/);
});

test("an explicit Agent settings command can still open the window", () => {
  assert.match(
    commandsSource,
    /crate::tray::show_agent_settings\(&app, &target_id\);/,
  );
});
