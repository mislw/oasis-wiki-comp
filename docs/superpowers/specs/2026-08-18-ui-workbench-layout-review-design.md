# UI Workbench Layout Review Design

## Goal

Turn Oasis UI Workbench into a focused layout-review surface. The developer uses it to inspect and correct control positions, sizes, hierarchy, Z-order, node classification, and render mode, then saves a review snapshot. The Workbench stays open and does not create or focus a Codex task. Back in the original conversation, the developer can say `确认导入` or `按刚保存的位置导入`; the Agent then reads the latest applicable snapshot and continues through the existing editor-delivery safety gates.

## Scope

- Remove the top-level image, tree, asset, export, and direct-delivery actions from the visible Workbench UI.
- Remove the eight-stage workflow strip from the Workbench UI.
- Keep the complete page-navigation, control-tree, canvas, inspector, Assets, and Structure experience.
- Keep drag, resize, parent assignment, Z-order, Node Kind, Render Mode, add, duplicate, delete, restore, reset, zoom, grid, snapping, visibility, locking, and other existing layout-review operations.
- Add one primary `保存布局` action and a compact `未保存` / `已保存 vN` state.
- Persist a validated, versioned `layout-review.json` beside the page's existing `session.json` without modifying the source session.
- Extend the bundled `oasis-wiki` interaction guidance so the Agent can discover and consume a saved layout only after a chat confirmation.

## Non-Goals

- Re-enable or open the temporarily disabled native `UI 生图工具链`.
- Run `open_ui_workflow.py`, open a localhost workflow console, or focus Companion from ordinary UI-generation wording.
- Import new images, UI Trees, or asset files from the Workbench.
- Re-run image analysis from the Workbench.
- Export a replacement UI Tree from the Workbench.
- Deliver to WidgetBlueprint directly from the Workbench.
- Treat saving, selecting, or reading a layout snapshot as editor-write authorization.
- Infer a WidgetBlueprint path from a page title, display name, `/Game/`, or a directory.
- Modify RedCliff `.uasset`, WidgetBlueprint, Lua, DataTable, or other UGC project files as part of saving the layout.

## Workbench Interface

The Workbench keeps its existing application frame and editing layout:

1. Page list and page selection.
2. Control tree with search, collapse, visibility, locking, hierarchy, and selection.
3. Canvas with reference image, control overlays, drag, resize, pan, zoom, grid, snapping, visual mode, and move scope.
4. Inspector with bounds, parent, Z-order, Node Kind, Render Mode, text-related fields, extraction metadata, and reset/restore operations.
5. Assets and Structure gallery.

The top bar contains the Workbench identity, the current page/image label, existing layout-edit commands, the new `保存布局` button, and the compact save state. It does not contain:

- `导入图片`
- `重新自动识别`
- `导入 UI Tree`
- `导入资产`
- `导出 UI Tree`
- `确认并交付到编辑器`

The eight-stage workflow strip is not rendered. Its stored task data is not deleted, migrated, or mutated by this UI change.

The save state behaves as follows:

- Loading a page whose current editable tree matches its latest valid snapshot shows `已保存 vN`.
- Any layout-affecting edit after load or save shows `未保存`.
- A successful save shows `已保存 vN`, where `N` is the persisted revision returned by the backend.
- A failed save leaves the state as `未保存` and shows the concrete validation or filesystem error in the existing notice surface.
- Changing pages loads and evaluates the selected page independently; save state never leaks between pages.

## Snapshot Location And Ownership

Each registered Workbench page already owns a session directory containing `session.json`. The reviewed layout is stored at:

```text
<session_dir>/layout-review.json
```

`session.json` remains the immutable source artifact for this feature. `layout-review.json` is the mutable human-reviewed overlay and the only file updated by `保存布局`.

The backend resolves the session directory through the existing registered page catalog. The frontend supplies a `page_id` and the editable tree; it never supplies an arbitrary output path. This prevents saving outside the registered Workbench page.

## Snapshot Schema

The initial schema is `artifact_type: "ui_layout_review"` with `schema_version: 1`:

```json
{
  "artifact_type": "ui_layout_review",
  "schema_version": 1,
  "status": "pending_chat_confirmation",
  "page_id": "resource-exchange",
  "revision": 3,
  "saved_at": "2026-08-18T12:34:56.789Z",
  "source": {
    "session_file": "session.json",
    "session_sha256": "<lowercase sha256>",
    "workflow_task_id": "<optional persisted task id>"
  },
  "page_size": {
    "width": 1415,
    "height": 794
  },
  "nodes": [
    {
      "id": "button.confirm",
      "name": "ConfirmButton",
      "category": "button",
      "parent_id": "panel.main",
      "bounds": { "x": 820, "y": 665, "width": 220, "height": 72 },
      "z_index": 12,
      "node_kind": "interaction",
      "render_mode": "outline",
      "visible": true,
      "locked": false,
      "opacity": 1,
      "extraction": {
        "mode": "native",
        "target_component_id": "button.confirm"
      }
    }
  ],
  "change_summary": {
    "changed_node_count": 4,
    "added": [],
    "deleted": [],
    "moved": ["button.confirm"],
    "resized": [],
    "reparented": ["label.confirm"],
    "z_order_changed": ["button.confirm"],
    "classification_changed": []
  }
}
```

The persisted `nodes` array contains every complete editable node, not only changed nodes. It preserves the Workbench node contract, including identity, semantic category, optional text and display metadata, hierarchy, bounds, extraction data, visibility, locking, opacity, Z-order, Node Kind, Render Mode, visual-asset references, review state, reuse metadata, and interaction metadata when present. This allows add, duplicate, and delete operations to survive saving without overwriting `session.json`.

`change_summary` is diagnostic evidence for the user and Agent. It is derived by comparing the submitted layout to the original `session.json`; it is never used as the authoritative reconstruction input.

## Validation

The backend validates the complete submitted layout before writing:

- `page_id` must exist in the registered Workbench catalog.
- The registered session directory must still contain a readable `session.json` whose `page_id` matches the request.
- `page_size.width` and `page_size.height` must be finite and greater than zero.
- Every node ID must be non-empty and unique.
- Every `parent_id` must identify another submitted node.
- A node cannot parent itself, and the parent graph must contain no cycles.
- Bounds values must be finite; width and height must be greater than zero.
- `z_index` must be an integer within the supported signed 32-bit range.
- `node_kind` and `render_mode` must be members of the Workbench enums.
- Every submitted node must contain the minimum fields required by the Workbench node contract: `id`, `category`, `bounds`, and a valid `extraction` object.
- Visual-asset paths remain relative session asset references or recognized native asset references; arbitrary filesystem traversal is rejected.

Added and duplicated nodes are saved as complete nodes. Deleted nodes are absent from the authoritative snapshot and listed under `change_summary.deleted`. Children of a deleted node must already have been reparented by the Workbench before validation succeeds.

## Revision And Atomic Persistence

The backend reads an existing valid snapshot, verifies that its `page_id` and schema match, then assigns `revision + 1`. With no prior snapshot, the first revision is `1`.

Saving follows this sequence:

1. Resolve the page through the catalog and read `session.json`.
2. Validate the request and compute the change summary.
3. Compute the current `session.json` SHA-256.
4. Serialize the complete next snapshot to a temporary file in the same session directory.
5. Flush and sync the temporary file.
6. Atomically replace `layout-review.json`.
7. Return the persisted snapshot metadata to the frontend.

Validation or serialization failure leaves the previous snapshot untouched. Replacement failure reports the error and preserves whichever last complete file remains. A malformed existing snapshot is renamed with an `.invalid-<timestamp>` suffix before a new revision `1` is written, matching the repository's existing recovery posture while retaining evidence for diagnosis.

## Tauri Commands

Add two narrow commands:

```text
save_ui_workbench_layout(page_id, layout) -> LayoutReviewSaveResult
load_ui_workbench_layout_review(page_id) -> LayoutReview | null
```

`save_ui_workbench_layout` performs all validation, revisioning, change-summary generation, hashing, and atomic persistence. The result includes `page_id`, `revision`, `saved_at`, `status`, and the persisted content hash used by the frontend to establish `已保存 vN`.

`load_ui_workbench_layout_review` resolves the registered page, returns `null` when no snapshot exists, and rejects malformed or unsupported snapshots with a concrete error. Loading a snapshot never mutates `session.json`, the page catalog, workflow stages, or editor state.

The core validation and persistence functions live in a focused Rust module rather than the Tauri command wrapper so they can be unit tested with temporary directories.

## Frontend Dirty-State Model

After a page loads, the frontend creates a canonical layout projection containing only the snapshot-owned fields. It compares the projection hash with the last loaded or saved projection hash.

- Matching hashes: `已保存 vN` when a valid snapshot exists, otherwise `未保存`.
- Non-matching hashes: `未保存`.
- A save request snapshots the submitted projection; edits made while the request is in flight keep the UI dirty even if the older request succeeds.
- A page switch discards only the in-memory edits of the page being left, consistent with current page loading behavior. No implicit auto-save is introduced.

## Original-Conversation Discovery

Saving remains entirely local to the Workbench and does not submit a prompt, create a task, navigate Codex, or focus another window.

The bundled `oasis-wiki` guidance handles `确认导入`, `按刚保存的位置导入`, and equivalent explicit phrases as follows:

1. Identify the page already named or tracked in the current conversation.
2. Load that registered page's `layout-review.json` and require `status == "pending_chat_confirmation"`.
3. If the page is not known, inspect pending snapshots associated with the current project/workspace. Use the sole pending snapshot only when exactly one exists; otherwise ask the user to identify the page and do not select by newest timestamp alone.
4. Re-read `session.json`, verify its SHA-256 matches `source.session_sha256`, and reject the snapshot as stale if the source session changed after saving.
5. Report the page ID, revision, saved time, and change summary before any editor operation.
6. Use the full snapshot node list as the authoritative reviewed coordinates, dimensions, parent relationships, and Z-order for the delivery plan.

The Agent must not claim that Workbench automatically sent the result back to chat. It explicitly discovers and reads the persisted file after the user's confirmation phrase.

## Editor Delivery Gate

`确认导入` confirms only that the saved Workbench layout may be used as input to the next delivery step. It does not by itself authorize an editor mutation.

Before any WidgetBlueprint write, the Agent must still obtain or verify all existing delivery requirements:

- the exact project workspace matches the running editor project;
- an exact editor-returned `UGCWidgetBlueprint` or `WidgetBlueprint` `load_path` identifies one asset;
- the current UMG plan is frozen and incorporates the saved layout revision;
- read-only MCP preflight succeeds and remains current;
- a project-external backup location and rollback point exist;
- the user explicitly authorizes this specific editor write after the target and scope are stated.

If any requirement is absent, the Agent stops at planning or preflight and asks for the one missing decision. It must never infer the target from the Workbench page, `/Game/`, a display name, or a directory.

After a successful editor write, the normal readback, compile/save, editor visual inspection, interaction, PIE, and multiplayer acceptance states remain separate. The snapshot status is not changed to imply those states.

## Error Handling

- Unknown page: reject the save or load and keep the current UI state.
- Missing or mismatched `session.json`: reject and instruct the user to reopen or regenerate the registered page.
- Invalid hierarchy, bounds, enum, Z-order, or node set: reject with node-specific details; do not write a partial snapshot.
- Source hash mismatch during chat confirmation: mark the snapshot stale in reporting and require a new Workbench save.
- Multiple applicable pending snapshots: ask which page to use; never choose solely by recency.
- Unsupported snapshot schema: stop and report the supported schema version.
- Editor preflight or backup failure: keep the snapshot pending and perform no editor write.
- Workbench close with unsaved edits: preserve current behavior for the first release; no new close-confirmation dialog is added.

## Tests

### Rust Unit Tests

- First save creates revision `1` without changing `session.json`.
- Repeated valid saves increment revisions and preserve the latest complete snapshot.
- Duplicate IDs, missing parents, self-parenting, cycles, invalid bounds, invalid Z-order, unsupported enums, and mismatched node sets are rejected.
- A rejected save leaves the previous snapshot byte-for-byte unchanged.
- Atomic replacement produces valid JSON and cleans up the temporary file.
- A malformed prior snapshot is retained with an invalid suffix and recovery starts at revision `1`.
- Loading returns `null` for no snapshot and rejects unsupported or malformed snapshots.
- Catalog resolution prevents arbitrary path traversal.

### Frontend Tests

- Removed import, re-analysis, export, direct-delivery actions, hidden file inputs, delivery dialog, and eight-stage strip are not rendered.
- Page list, control tree, canvas, inspector, Assets, Structure, and all requested layout controls remain rendered and usable.
- An edit changes the state to `未保存`.
- A successful save shows `已保存 vN`.
- An edit made during an in-flight save remains `未保存` after the response.
- Save failure remains dirty and exposes the backend error.
- Page switches maintain independent revision and dirty state.
- Saving invokes only `save_ui_workbench_layout`; it does not invoke prompt submission, task creation, window focus, workflow opening, or editor-delivery commands.

### Skill And Integration Tests

- Skill guidance recognizes explicit saved-layout confirmation phrases.
- Guidance requires reading a real pending snapshot and verifying the source hash.
- Guidance refuses ambiguous multiple-snapshot selection.
- Guidance states that layout confirmation is not editor-write authorization.
- The native `UI 生图工具链` remains disabled and no `open_ui_workflow.py` path is reintroduced.
- Companion Skill copies and version metadata remain synchronized.

## Acceptance Criteria

- The Workbench opens directly into the focused layout-review interface with no top import/export/delivery group and no eight-stage strip.
- The complete requested layout-editing experience remains available.
- `保存布局` creates a validated `layout-review.json` with a monotonically increasing revision while leaving `session.json` unchanged.
- The UI accurately distinguishes `未保存` from `已保存 vN` across edits, saves, failures, and page changes.
- Saving does not create or open a Codex task and does not call editor delivery.
- In the original conversation, an explicit confirmation causes the Agent to read the correct non-stale snapshot and use its bounds, hierarchy, and Z-order in the delivery plan.
- No editor asset is modified until the existing exact-target, project-match, preflight, external-backup, frozen-plan, and explicit-write-confirmation gates are all satisfied.
- Automated tests, frontend build, Rust tests, packaged Skill hygiene, version consistency, installed build verification, and visible Workbench runtime checks pass before release.
