# UI Workbench Layout Review Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Workbench import/export/direct-delivery UI with a validated `保存布局` flow whose snapshot is consumed only after explicit confirmation in the original conversation.

**Architecture:** A focused Rust module owns snapshot validation, revisioning, hashing, catalog-scoped path resolution, and atomic persistence. A small TypeScript helper owns canonical layout projections and dirty-state decisions, while `UIWorkbench.tsx` keeps all layout editing but removes import/export/workflow/delivery surfaces. Bundled Skill guidance defines saved-layout discovery and preserves the separate editor-write authorization gate.

**Tech Stack:** React 19, TypeScript 5.8, Node test runner, Tauri 2, Rust/serde/serde_json/sha2, Python unittest, PowerShell, WiX MSI.

**Spec:** `docs/superpowers/specs/2026-08-18-ui-workbench-layout-review-design.md`

## Global Constraints

- The native `UI 生图工具链` remains temporarily disabled; never run `open_ui_workflow.py` or open/focus the native workflow.
- `session.json` remains unchanged; only `<session_dir>/layout-review.json` is written.
- Saving or chat-confirming a layout never grants editor-write authorization.
- The Workbench retains complete layout editing, including add, duplicate, delete, drag, resize, hierarchy, Z-order, Node Kind, Render Mode, zoom, grid, Assets, and Structure.
- Editor delivery still requires exact WidgetBlueprint `load_path`, project match, read-only preflight, project-external backup, frozen plan, and explicit write confirmation.
- Use `apply_patch` for manual edits and preserve unrelated worktree changes.

---

### Task 1: Rust Layout Snapshot Domain

**Files:**
- Create: `src-tauri/src/ui_workbench_layout.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: inline `#[cfg(test)]` module in `src-tauri/src/ui_workbench_layout.rs`

**Interfaces:**
- Consumes: `ui_workbench_catalog::WorkbenchCatalog` and registered `WorkbenchPage.session_dir`.
- Produces: `LayoutReview`, `LayoutReviewRequest`, `LayoutReviewSaveResult`, `save_layout_review(catalog, page_id, request, now_ms)`, and `load_layout_review(catalog, page_id)`.

- [ ] **Step 1: Write failing Rust tests for first save and revision increment**

Create temporary registered sessions and assert first save writes revision `1`, second save writes revision `2`, the JSON status is `pending_chat_confirmation`, and `session.json` bytes do not change.

- [ ] **Step 2: Run the focused Rust tests and verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ui_workbench_layout -- --nocapture`

Expected: FAIL because `ui_workbench_layout` and its API do not exist.

- [ ] **Step 3: Implement schema, catalog resolution, hashing, and atomic save**

Define serde types for page size, bounds, full editable nodes, source metadata, change summary, and save result. Resolve paths only from the page catalog, hash `session.json` with SHA-256, write `layout-review.json.tmp`, sync it, and rename it to `layout-review.json`.

- [ ] **Step 4: Add failing validation and recovery tests**

Cover duplicate IDs, missing parents, self-parenting, cycles, non-finite/invalid bounds, invalid path traversal in visual assets, malformed prior snapshots, unsupported schemas, and rejected-save preservation of the previous bytes.

- [ ] **Step 5: Run validation tests and verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ui_workbench_layout -- --nocapture`

Expected: the new validation cases fail with missing checks.

- [ ] **Step 6: Implement minimal validation and malformed-snapshot recovery**

Return node-specific errors, rename malformed prior files to `layout-review.invalid-<timestamp>.json`, restart at revision `1`, and never partially replace the prior valid snapshot.

- [ ] **Step 7: Run focused Rust tests and verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ui_workbench_layout -- --nocapture`

Expected: PASS with zero failed tests.

### Task 2: Tauri Save And Load Commands

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/ui_workbench_layout.rs`

**Interfaces:**
- Consumes: Task 1 domain functions and `AppState.ui_workbench_catalog`.
- Produces: Tauri commands `save_ui_workbench_layout(page_id, layout)` and `load_ui_workbench_layout_review(page_id)`.

- [ ] **Step 1: Write a failing source registration test**

Add a Rust source-level or command registration assertion that both commands are exported and registered in `generate_handler!`.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ui_workbench_layout -- --nocapture`

Expected: FAIL because the commands are not registered.

- [ ] **Step 3: Implement command wrappers and registration**

Clone the catalog under its mutex, call the domain functions, and return serialized typed results. Do not mutate workflow stages, submit prompts, open windows, or call delivery code.

- [ ] **Step 4: Run Rust tests and verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ui_workbench_layout -- --nocapture`

Expected: PASS.

### Task 3: Frontend Projection And Dirty State

**Files:**
- Create: `src/windows/uiWorkbenchLayout.ts`
- Create: `tests/ui-workbench-layout.test.mjs`
- Modify: `package.json`

**Interfaces:**
- Consumes: editable `UITree` node/page data.
- Produces: `workbenchLayoutProjection(tree)`, `workbenchLayoutFingerprint(tree)`, and `workbenchLayoutSaveState(currentFingerprint, persisted)`.

- [ ] **Step 1: Write failing Node tests**

Assert the projection preserves complete editable nodes, stable key ordering makes equivalent trees match, layout edits change the fingerprint, and an in-flight save result does not clear dirty state when the current fingerprint differs from the submitted fingerprint.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `node --test tests/ui-workbench-layout.test.mjs`

Expected: FAIL because `uiWorkbenchLayout.ts` does not exist.

- [ ] **Step 3: Implement the minimal projection and save-state helpers**

Return JSON-safe data with deterministic recursive object-key ordering while retaining node array order. Keep request bookkeeping pure and independent of React/Tauri.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run: `node --test tests/ui-workbench-layout.test.mjs`

Expected: PASS.

- [ ] **Step 5: Add the test to `test:hierarchy`**

Update `package.json` so Task 4's aggregate run includes `tests/ui-workbench-layout.test.mjs`. Do not run the aggregate yet because its intentionally failing Workbench source assertions are the RED state for Task 4.

### Task 4: Focused Workbench UI And Save Flow

**Files:**
- Modify: `src/windows/UIWorkbench.tsx`
- Modify: `src/windows/UIWorkbench.css`
- Modify: `tests/ui-workbench-session.test.mjs`

**Interfaces:**
- Consumes: Task 2 Tauri commands and Task 3 projection helpers.
- Produces: visible `保存布局`, per-page `未保存` / `已保存 vN`, no direct delivery/import/export workflow UI.

- [ ] **Step 1: Replace obsolete source assertions with failing focused-layout assertions**

Assert removed labels, hidden file inputs, `workflowStageRows`, delivery dialog, `confirm_and_deliver_ui`, prompt submission, and eight-stage strip are absent. Assert `保存布局`, both Tauri snapshot commands, layout controls, page list, tree, canvas, inspector, Assets, and Structure remain.

- [ ] **Step 2: Run the Workbench Node tests and verify RED**

Run: `node --test tests/ui-workbench-session.test.mjs tests/ui-workbench-layout.test.mjs`

Expected: FAIL because the old top actions/workflow/delivery surfaces remain and save UI is absent.

- [ ] **Step 3: Remove obsolete UI state and handlers**

Delete file inputs and their refs/handlers, image re-analysis/import/export handlers, workflow-stage loading/rendering, delivery dialog/preflight/search/new-task code, opener usage, and associated CSS. Keep session/catalog loading and all layout-editing handlers.

- [ ] **Step 4: Implement load/save/dirty behavior**

On page load, call `load_ui_workbench_layout_review`; when valid, use its full nodes/page size as the editable tree and retain revision/fingerprint per page. `保存布局` submits the current projection and only clears dirty state when the response corresponds to the still-current fingerprint.

- [ ] **Step 5: Run focused Node tests and verify GREEN**

Run: `node --test tests/ui-workbench-session.test.mjs tests/ui-workbench-layout.test.mjs`

Expected: PASS.

- [ ] **Step 6: Run TypeScript build and aggregate frontend tests**

Run: `npm run build`

Run: `npm run test:hierarchy`

Expected: both PASS.

### Task 5: Original-Conversation Skill Discovery Rules

**Files:**
- Modify: `skills/oasis-wiki/references/oasis-ui-agent-interaction.md`
- Modify: `skills/oasis-wiki/references/cowart-ui/usage-guide.md`
- Modify: `skills/oasis-wiki/SKILL.md`
- Modify: `skills/oasis-wiki/AGENTS.md`
- Modify: `skills/oasis-wiki/agents/openai.yaml`
- Modify: matching files under `src-tauri/resources/skill/`
- Modify: `skills/oasis-wiki/tests/test_oasis_ui_agent_interaction.py`
- Modify: matching test under `src-tauri/resources/skill/tests/`

**Interfaces:**
- Consumes: schema `ui_layout_review` version `1` and status `pending_chat_confirmation`.
- Produces: explicit handling for `确认导入` / `按刚保存的位置导入`, stale hash checks, ambiguity refusal, and editor authorization separation.

- [ ] **Step 1: Write failing Python guidance tests**

Assert the guides mention `layout-review.json`, `pending_chat_confirmation`, `session_sha256`, sole-pending-snapshot selection, refusal to choose multiple snapshots by timestamp, and the exact-target/preflight/backup/frozen-plan/explicit-write gate.

- [ ] **Step 2: Run focused Python tests and verify RED**

Run: `python -m unittest discover -s skills/oasis-wiki/tests -p "test_oasis_ui_agent_interaction.py"`

Expected: FAIL because saved-layout confirmation guidance is absent.

- [ ] **Step 3: Implement the guidance in the canonical Skill copy and mirror it**

Document that Workbench save does not auto-return to chat, the Agent explicitly reads the file after confirmation, and native workflow opening remains disabled. Treat the current `skills/oasis-wiki` copy as the implementation baseline and apply the same patch to `src-tauri/resources/skill`; do not run `scripts/sync-bundled-skill.ps1` because its default independent source checkout is older than the current packaged Skill.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run: `python -m unittest discover -s skills/oasis-wiki/tests -p "test_oasis_ui_agent_interaction.py"`

Expected: PASS.

- [ ] **Step 5: Run full Skill hygiene/tests**

Run:

```powershell
python -m unittest discover -s skills/oasis-wiki/tests -p "test_*.py"
node --test skills/oasis-wiki/tests/cowart-ui-category.test.mjs
powershell -ExecutionPolicy Bypass -File skills/oasis-wiki/scripts/check-skill-hygiene.ps1
```

Expected: all PASS and no packaged-copy drift.

### Task 6: Release Version, Full Verification, Install, And Runtime Acceptance

**Files:**
- Modify: release/version files through `node scripts/set-release-version.mjs 1.260818.7`
- Generated build artifacts: ignored MSI/binaries only.

**Interfaces:**
- Consumes: all prior tasks.
- Produces: installable Companion `1.260818.7` with matching bundled/installed Skill and visible focused Workbench.

- [ ] **Step 1: Set the next release version**

Run: `node scripts/set-release-version.mjs 1.260818.7`

- [ ] **Step 2: Run full static verification**

Run frontend tests/build, all Rust tests, `cargo fmt --check`, version tests, Skill tests/hygiene, and `python scripts/check_companion_skill_versions.py` after installation.

- [ ] **Step 3: Build the Windows MSI**

Run: `npm run tauri build -- --config tauri.build.conf.json`

Expected: a `1.26.818+7` MSI is produced.

- [ ] **Step 4: Install and restart the exact built version**

Close the running Companion, install the new MSI non-interactively, start `C:\Program Files\Oasis Companion\oasis-companion.exe`, and verify its file/process version maps to `1.260818.7`.

- [ ] **Step 5: Run visible Workbench acceptance**

Open only the Workbench window through the supported Companion entry, verify the removed top actions/stage strip are absent, layout controls remain visible, edit a disposable registered session, save twice, and confirm `layout-review.json` revisions `1` then `2` while `session.json` remains byte-identical. Do not open the disabled native workflow and do not write an editor asset.

- [ ] **Step 6: Verify installed Skill/runtime match**

Run: `python scripts/check_companion_skill_versions.py`

Expected: exit `0` with `status=match` for the actual running process.

- [ ] **Step 7: Review diff and commit implementation**

Run: `git diff --check`, inspect `git diff`, confirm only intended source/docs/version files changed, then commit with a focused message. Do not push unless the user has asked to synchronize GitHub for this implementation.
