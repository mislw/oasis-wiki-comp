# UGC Pitfalls

Use this checklist before finalizing advice or code snippets.

## Server And Client Confusion

Symptoms:

- UI button prints but gameplay does not change.
- Code works in standalone but not multiplayer.
- Client sees a local value, server ignores it.

Check:

- Gameplay state, inventory, damage, team, score, spawning, and skill activation should be server-authoritative.
- UI should call a server RPC on PlayerController.
- Client RPCs should only update local UI/feedback.

## Server RPC Not Registered

Symptoms:

- `UnrealNetwork.CallUnrealRPC` appears to do nothing.
- Client-to-server call never reaches the function.

Check:

- Add the server function name to `UGCPlayerController:GetAvailableServerRPCs()`.
- Match the string exactly.
- Keep server RPC functions on the correct PlayerController class.

## Replicated Field Missing

Symptoms:

- Server value changes but client UI never updates.
- Reconnect loses expected display state.

Check:

- Add the field to `GetReplicatedProperties()`.
- For tables, use `UnrealNetwork.RepLazyProperty` if the project uses lazy replication.
- Call `ForceNetUpdate()` for template-style table updates when similar code does so.
- Avoid mutating nested tables without replication notification.

## Blueprint Path `_C` Mistakes

Symptoms:

- `UE.LoadClass` returns nil.
- Widget or actor class cannot be created.

Check:

- `UE.LoadClass` for Blueprint classes usually needs `Foo.Foo_C`.
- `UE.LoadObject` for non-class assets usually uses `Foo.Foo`.
- Root-relative UGC paths often need `UGCMapInfoLib.GetRootLongPackagePath()` or `UGCGameSystem.GetUGCResourcesFullPath(...)`.

## UGC Resource Path Missing Leading Slash

Symptoms:

- PIE shows the generic dialog `在 Lua 文件中发现了一些错误，请检查输出日志。` after a table row or UI config starts exercising new code.
- The feature works again when the triggering row is removed, which can make the row data or activity time look responsible.
- Client or PIE logs contain `Path starts with 'Asset', which is no longer supported` and `OriginalPath=Asset/...`.

Check:

- Arguments passed to `UGCGameSystem.GetUGCResourcesFullPath(...)` must start with `/Asset/`, not `Asset/`.
- Use `UGCGameSystem.GetUGCResourcesFullPath('/Asset/Data/Table/Foo.Foo')` for tables and `UGCGameSystem.GetUGCResourcesFullPath('/Asset/UI/Foo.Foo_C')` for WidgetBlueprint classes.
- Search every path reached by the new row; table, widget class, shop, object, texture, and other dependent lookups can fail separately.
- Treat the generic Lua dialog as a summary only. Confirm the cause from fresh `Clientlog`, `LuaLog`, or PIE output, then restart PIE and verify that no new path error is emitted.

## Missing Nil Checks

Symptoms:

- Runtime errors after login, reconnect, respawn, or phase transition.

Check:

- Nil-check PlayerController, PlayerPawn, PlayerState, GameState, widget class, widget object, and actor arrays.
- Player pawn can be nil during login/reconnect/respawn.
- UI can be nil after reconnect or if creation failed.

## Tick Does Too Much

Symptoms:

- Lag, repeated logs, inconsistent countdowns.

Check:

- Avoid scanning all actors or all players every frame unless necessary.
- Update countdown UI once per second.
- Cache actor lists when safe.
- Disable `bEnableActionTick` when an Action finishes.

## UI Direct Coupling

Symptoms:

- PlayerController reaches into UI fields and breaks when UI is recreated.
- Reconnect or settlement screen creates duplicate/invalid widgets.

Check:

- Prefer `ClientRPC -> UGCEventSystem:SendEvent -> UI listener`.
- UI widgets should register/unregister listeners.
- UIManager should own widget creation and viewport ordering.

## Listener Leaks

Symptoms:

- UI callback fires multiple times.
- Old widget still receives events.

Check:

- Pair `UGCEventSystem:AddListener` with `RemoveListener`.
- Remove listeners in `Destruct`, close, or end-play paths.
- Avoid creating the same singleton UI repeatedly without checking existing instance.

## Button Hidden In WBP Reappears At Runtime

Symptoms:

- A button or entry set to hidden in the Widget Designer still shows in PIE, or shows again after any UI refresh.
- Editing the Lua hide logic is undesirable because the refresh path is shared or in code you should not touch.

Check:

- Search Lua for the refresh path that enumerates controls and calls `SetVisibility(...Visible)`; a page or state updater can re-apply visibility every refresh and override a Designer-only hide.
- Collapse the smallest dedicated parent container that wraps only the target control (a `SizeBox` or slot with no other children) instead of hiding the control itself. Refresh code usually walks named controls, not wrapper slots, so the hide survives with no Lua edit and cannot be overwritten.
- A running PIE keeps the old widget tree and an open Designer tab keeps the old property view; verify after restarting PIE, and close/reopen the asset tab to see the change in the Designer.

## Removing A Dead Button

Symptoms:

- A top-level button opens nothing, prints an open-UI error, or points to a feature that no longer exists.

Check:

- Confirm it is actually dead before touching it: the target WidgetBlueprint or resource does not exist in the project and the UI is not registered in UIManager, so a click can only error.
- Remove the whole Designer subtree that exists only for the button (its wrapper slot included), compile, and save. Then verify the generated class no longer exposes the button and its child widgets.
- Clean the Lua side in one pass: the `Construct` binding, the `Destruct` clear, the click callback, and any generated class-field comments for the removed controls.
- Restart PIE for runtime and reopen the asset tab for Designer preview; either one alone is not evidence the widget is gone.

## World-Following UI Animation Drifts Or Jitters

Symptoms:

- A full-screen intro animation meant to appear near a world actor (boss arrival, elite spawn) jitters, lags behind the camera, or lands off-screen.
- The UI recomputes its viewport position every tick from an actor world location (`FindBossActor`, `SetPositionInViewport`, `SetAlignmentInViewport`).

Check:

- Prefer a fixed viewport presentation: anchor the animation at viewport alignment `(0.5, 0.5)` plus a small fixed offset, and play a pre-sampled frame sequence at a fixed frame rate from one timer. Presentation then ignores camera and actor movement.
- Keep the play trigger contract (who plays it, which id it logs) separate from the positioning code, so the positioning can be deleted later without touching callers.
- Verify after restarting PIE, not in an open Designer tab; viewport-position behavior only exists at runtime.

## Reconnect State Missing

Symptoms:

- Player reconnects without weapons, wrong UI, wrong team, or stale skill cooldown.

Check:

- Register respawn/reconnect/recovered delegates on server.
- Re-send phase, team, loadout, skill choice, score, and UI visibility.
- Make repeated ClientRPC events safe.

## Config Hidden In Logic

Symptoms:

- Hard to balance or change mode rules.
- Same item IDs/timings repeated in several files.

Check:

- Move item IDs, durations, skill cooldowns, text, and icon paths into `GlobalConfig.lua` or focused config files.
- Keep Actions focused on flow, not giant data tables.

## Duplicate Config Source After Merge

Symptoms:

- Two tables or fields can configure the same unlock level, cost, or rule; one was added locally while the other already existed on the latest origin branch.
- A merge conflict exposes parallel implementations, or one field becomes silently unused.

Check:

- Before adding a per-level list or mapping, search the entity's own table and the latest origin branch for a per-entity field that already controls it (for example an unlock level on the entity row).
- Prefer the per-entity field as the single source; keep level/aggregate tables for conditions, descriptions, and display-only text.
- If a duplicate must stay temporarily, mark it deprecated in its tooltip and do not leave it editable without a warning; remove it once the editor/MCP can rewrite the struct.
- After resolving a merge, grep for the losing field name. If no consumer remains, it is dead config, not a fallback.

## One Attribute Key, Two Numeric Semantics

Symptoms:

- A shared attribute dictionary stores one `ValueType` (absolute vs percent) per key, but the same key is delivered as absolute in one table and as percent in another.
- Nobody can say which source wins, and the dictionary values contradict the delivery rows.

Check:

- Numeric semantics belong to the delivery row (a gem row's own `ValueType`, equipment breakthrough fixed percent), not to a single dictionary flag; never consume the dictionary copy in logic.
- Before "fixing" one side to match the other, grep every consumer of both fields. If the dictionary field has no real reader, remove or downgrade it rather than inventing a truth it cannot express.
- Removing the duplicate follows the normal struct-field removal flow: edit the struct, resave the table data, and keep `ue_struct_custom.lua` in sync with the struct change.

## Table Or Struct Change Not Visible At Runtime

Symptoms:

- A planner updated a config table and the editor asset reads the new value, but the running game/PIE still shows the old value.
- A DataTable or `UserDefinedStruct` was reloaded or re-imported, yet an already-running PIE session behaves as before.
- Unlock counts, costs, drop rules, or table-driven UI text stay stale.

Check:

- Editor asset reload and PIE runtime are different caches. Saving or reloading the `.uasset` updates the editor, but a PIE session that started earlier keeps the DataTable values it loaded at startup; PIE does not hot-reload table content.
- Start a new PIE session after the table or struct change, then read the runtime value directly in client/DS Lua before judging the data.
- Confirm the disk asset is the committed version and the editor is not holding an unsaved override.
- Separate "not synchronized" from "bad data": after a fresh PIE confirms the new values, inspect the row content itself (for example a delimiter-separated count list with one missing value) before blaming the loading pipeline.

## Table-Driven Probability Change Unverified

Symptoms:

- A break/success/drop rate was moved into a table (often keyed by player count or level), but only the config was checked.
- Runtime cadence is unproven: how often the roll runs, when the first roll happens, and how many rolls fit in the designed window.

Check:

- Verify in PIE logs with the effective values printed from the runtime code: the rate actually read for the current row, the judgment interval, the first-judgment delay, and the judgment index that triggered the event.
- Start with the zero/smallest row as a baseline, then sample at least one populated row; a single-tier pass does not prove the table lookup.
- When the design defines a safe window or an expected judgment count, compute it from interval and first-roll delay and compare with the log, not just the final success/failure.

## Transform Buff Stuck Or Model Misplaced

Symptoms:

- After `UGCPersistEffectSystem.AddBuffByClass` applies a copied official transform Buff (`PEBuff_Transform_*`), the player can no longer move, turn, or act.
- DS or client log shows `PEBuff_Transforming.lua:15: attempt to index a nil value (global 'PersistEffectSkillStaticFunction')`.
- The transformed model is sunk into the ground (wheels or feet missing), faces backwards, or still carries leftovers from the source template (extra weapon, extra health, auto-ending after a fixed time).

Check:

- In editor 1.39 the global `PersistEffectSkillStaticFunction` used by the official `PEBuff_Transforming` template may be undefined while `UAEClosure_PersistEffectSkillStaticFunction` exists. The transition sub-buff then fails in `OnApply_BP` and leaves the pawn in the non-operable transforming state. Define the alias in a module that loads on both DS and client before the Buff is applied: `PersistEffectSkillStaticFunction = PersistEffectSkillStaticFunction or UAEClosure_PersistEffectSkillStaticFunction`. Re-check the log after each editor update; drop the alias once the official template no longer needs it.
- `变身-静态模型` needs a `StaticMesh`. A vehicle resource that only ships a `SkeletalMesh` must use the Preset (怪物骨骼) type. A Preset with `PresetTransformAnim = None` renders the reference pose; never keep another skeleton's ABP from the copied template.
- For the Preset type, `MeshRelativeTransform` is relative to the character `Mesh` component (offset below the capsule center and rotated by the character mesh yaw), not to the capsule. Measure at runtime: compare the transformed mesh component world Z with `CharacterMovement.CurrentFloor.HitResult.ImpactPoint.Z`, then fix Z and yaw in the Buff defaults.
- When copying a template such as `PEBuff_Transform_Chicken`, reset `WeaponID`, `EnableExtraHealth`/`ExtraHealth`, `ApplyTime` (`< 0` means permanent), the camera additive fields, and the collision box size for the new model.
- `CharacterMovement.MaxWalkSpeed` set in `ReceiveBeginPlay` can be overwritten by the game's own speed logic; read the runtime value before relying on it.

## Editor-Rewritten Tracked Files Before Commit

Symptoms:

- `git status` shows tracked Lua or asset files you never edited.
- The diff is only line-ending, whitespace, timestamp, or whole-file re-save churn from the UGC editor.
- A large group of assets was re-saved by one editor batch operation at an unrelated time.

Check:

- Inspect the diff before staging: `git diff --stat`, `git diff --ignore-cr-at-eol`, and a normal diff of the suspicious file. If only line endings or formatting changed, restore it and keep it out of feature commits.
- Restore only files that are clearly editor-generated noise. Never restore a teammate's or the user's real edits to make the worktree look clean.
- When many unrelated editor-resaved files exist, group commits by feature and keep editor churn in a separate commit or stash only after the owner confirms; do not mix them into a feature commit silently.
- After an editor version upgrade, a large batch of tracked assets can change automatically and cannot be byte-restored. Commit that churn as its own clearly labeled commit (for example "editor upgrade auto-changes") instead of mixing it into feature work; never hand-restore binary assets you cannot reconstruct.
- After push, compare the remote tree for the changed paths so a concurrent merge by another author is not mistaken for a lost change.

## Config Source Owned Outside The Repository

Symptoms:

- A table row is wrong in the `.uasset`, but its authoritative source is a planner CSV/Excel kept on another machine.
- Fixing the `.uasset` locally works until the next import silently overwrites it.

Check:

- Before a permanent table fix, determine the import source. If the source lives outside this checkout, report the exact table, row name, field, current value, expected value, and why it is wrong, and ask the owner to fix and re-import.
- A local hot-fix is acceptable only as a temporary PIE-verification aid: label it as temporary, keep it uncommitted or revert it afterward, and state clearly that the source still needs the fix.
- Do not claim the issue is resolved when only the local `.uasset` was patched.

## Wrong Player Lookup

Symptoms:

- Change affects the wrong player or nil target.

Check:

- Use `PlayerKey` for stable player-specific lookup.
- Prefer `UGCGameSystem.GetPlayerControllerByPlayerKey`, `GetPlayerPawnByPlayerKey`, and `GetPlayerStateByPlayerKey`.
- Be careful with local player controller helpers in server-side code.

## Unreliable RPC Used For Critical State

Symptoms:

- Occasional missing state, button stuck, wrong settlement result.

Check:

- Use unreliable RPC only for high-frequency visual/progress refresh where the next update can correct it.
- Use reliable RPC or replicated state for critical choices, phase changes, inventory grants, team assignments, and settlement.

## Damage Or Team Logic Runs Too Broadly

Symptoms:

- Friendly fire behaves wrong.
- Damage blocked for everyone.

Check:

- In `LuaModifyDamage`, nil-check instigator/victim PlayerState.
- Compare TeamID carefully.
- Return original `Damage` unless intentionally blocking or scaling it.

## Action Flow Dead End

Symptoms:

- Game gets stuck in prepare or round end.

Check:

- Every Action that starts a timer must eventually call `LuaQuickFireEvent` or intentionally end the flow.
- `Update` only runs when `bEnableActionTick` is true.
- Reset timer counters when re-entering the same Action.

## Unexpected Editor-Incompatible Or Generated Files

Trigger this check whenever `git status`, a pre-commit/pre-push review, release validation, project scan, or editor import check finds files that the UGC editor does not consume or that appear to be accidental tooling output. Examples include:

- `__pycache__/` and `*.pyc`: Python bytecode caches generated when Python modules run. They are not source files and the UGC editor does not use them.
- Temporary Python, shell, log, prompt, manifest, contact-sheet, render, or extraction files created by automation.
- Unexpected `.md` files inside the UGC project. Markdown is a text documentation format and may be legitimate; do not classify `README.md`, approved design documents, or maintained project documentation as garbage without inspecting their path and content.

Before proposing any action:

1. Report the exact path, extension, size, and whether it is a file or directory.
2. Inspect enough content or metadata to explain what it is and what this exact artifact appears to do. Do not execute an unknown file merely to identify it.
3. Run `git status --short` and focused `git log --diff-filter=A -- <path>` / `git show` checks to classify it as untracked, staged, committed but not pushed, or already present on a remote branch.
4. Check whether the file was intentionally referenced by project scripts, docs, tests, build steps, or editor assets before calling it removable.
5. Explain the finding in plain language and ask the user what to do. Do not delete, unstage, amend, revert, push, or add ignore rules before approval.

Offer actions according to state:

- Untracked: offer to delete it and, when appropriate, add a narrow `.gitignore` rule such as `__pycache__/` and `*.pyc`.
- Staged: offer to unstage and delete it, but require explicit approval for both actions.
- Committed but not pushed: report the commit and offer either a new cleanup commit or an explicitly authorized amend. Do not reset automatically.
- Already pushed: report the branch and commit, then ask whether to create and push a normal cleanup commit that deletes the artifact. This is the safe default.
- Published-history withdrawal: distinguish deleting the file in a new commit from removing the original commit from remote history. Never force-push or rewrite published history without a second, explicit authorization that names the exact branch and commits and acknowledges the collaboration risk.

Suggested prompt:

```text
I found <path>. It is <file type and purpose>. Git currently shows it as <untracked/staged/committed/pushed in commit>. The UGC editor does not use this file / its project purpose is not confirmed. Do you want me to (A) leave it, (B) delete it and add a narrow ignore rule, or (C) if already pushed, create a cleanup commit and push the deletion? Rewriting the remote history is a separate high-risk action and will not be performed without another explicit confirmation.
```

## Teaching Answer Checklist

Before answering with code, make sure the answer says:

- Which file and function/table to edit.
- Whether the code is server, client, UI, GameState, GameMode, PlayerController, Pawn, or Action.
- Whether an RPC must be registered.
- Whether replication must be updated.
- How to test in editor/game.
