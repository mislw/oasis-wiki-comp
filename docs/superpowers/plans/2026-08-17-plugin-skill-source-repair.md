# Plugin Skill Source Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a cloned, packaged, or plugin-installed Oasis Companion reliably expose the complete `oasis-wiki` Skill without false stub warnings.

**Architecture:** Keep the standalone `mislw/oasis-wiki` repository canonical. Mirror it into both Companion's Tauri resource directory and the Codex plugin `skills/oasis-wiki` directory, then resolve only trusted local candidates containing `SKILL.md`. Validate the plugin, Skill-copy parity, UI warning policy, and synchronized release version before building and publishing `1.260817.5`.

**Tech Stack:** Rust/Tauri 2, React/TypeScript, Node test runner, PowerShell/robocopy, Codex plugin manifest, Python Skill checks.

---

### Task 1: Preserve Approved Design and Isolate Work

**Files:**
- Modify: `docs/superpowers/specs/2026-08-17-oasis-wiki-plugin-distribution-design.md`
- Create: `docs/superpowers/plans/2026-08-17-plugin-skill-source-repair.md`

- [ ] **Step 1: Confirm the design includes trusted source order**

Verify the spec names packaged `skill`, plugin `skills/oasis-wiki`, and source `resources/skill` candidates and requires `SKILL.md`.

- [ ] **Step 2: Commit only the two documentation files**

```powershell
git add docs/superpowers/specs/2026-08-17-oasis-wiki-plugin-distribution-design.md docs/superpowers/plans/2026-08-17-plugin-skill-source-repair.md
git commit -m "Document plugin Skill source repair"
```

- [ ] **Step 3: Create an isolated Companion worktree**

Use `superpowers:using-git-worktrees` and create a `codex/plugin-skill-source-repair` worktree from the documentation commit so the existing MCP and lockfile changes remain untouched.

### Task 2: Release the Canonical Skill Changes

**Files:**
- Modify: `../oasis-wiki/oasis-wiki/VERSION`
- Preserve existing approved changes: `../oasis-wiki/oasis-wiki/references/mcp-ui-widget.md`
- Preserve existing approved tests: `../oasis-wiki/oasis-wiki/tests/test_ui_workbench_companion_handoff.py`

- [ ] **Step 1: Run the focused existing Skill regression test**

```powershell
python -m unittest oasis-wiki.tests.test_ui_workbench_companion_handoff
```

Expected: the component reuse and Companion handoff tests pass before release metadata changes.

- [ ] **Step 2: Bump the canonical Skill version**

Set `oasis-wiki/VERSION` to:

```text
1.260817.5
```

- [ ] **Step 3: Run Skill hygiene and version tests**

```powershell
powershell -ExecutionPolicy Bypass -File oasis-wiki/scripts/check-skill-hygiene.ps1
python -m unittest discover -s oasis-wiki/tests -p "test_*.py"
```

Expected: exit code `0` with all tests passing.

- [ ] **Step 4: Commit and push the canonical Skill release**

```powershell
git add oasis-wiki/VERSION oasis-wiki/references/mcp-ui-widget.md oasis-wiki/tests/test_ui_workbench_companion_handoff.py
git commit -m "Release oasis-wiki 1.260817.5"
git push origin main
```

### Task 3: Add Failing Packaging and UI Tests

**Files:**
- Create: `tests/plugin-packaging.test.mjs`
- Modify: `tests/companion-versioning.test.mjs`
- Test: `src-tauri/src/skill/mod.rs`

- [ ] **Step 1: Write a failing plugin packaging test**

The test must load `.codex-plugin/plugin.json`, assert `name === "oasis-wiki-comp"`, `skills === "./skills/"`, `version === package.json.version`, require `skills/oasis-wiki/SKILL.md`, compare all non-cache files with `src-tauri/resources/skill`, and assert `Settings.tsx` does not contain `minimal stub` or `最小 stub`.

- [ ] **Step 2: Extend the version test**

Read `.codex-plugin/plugin.json` and `skills/oasis-wiki/VERSION`, then assert both equal the canonical package version.

- [ ] **Step 3: Write failing Rust resolver tests**

Add tests for a helper with this interface:

```rust
fn resolve_skill_source(candidates: &[PathBuf]) -> Result<PathBuf, String>
```

Cover first-valid precedence, rejection of a directory without `SKILL.md`, and an error containing every attempted path.

- [ ] **Step 4: Run tests and verify RED**

```powershell
node --test tests/plugin-packaging.test.mjs tests/companion-versioning.test.mjs
cargo test skill::tests::resolve_skill_source --manifest-path src-tauri/Cargo.toml
```

Expected: Node fails because the plugin manifest/copy does not exist; Rust fails because `resolve_skill_source` does not exist.

### Task 4: Implement Trusted Skill Source Resolution

**Files:**
- Modify: `src-tauri/src/skill/mod.rs`

- [ ] **Step 1: Add the minimal resolver**

Implement `resolve_skill_source` so it returns the first candidate that is a directory and contains a file named `SKILL.md`. On failure, return one error headed `bundled Skill resource not found; attempted trusted paths:` followed by every candidate.

- [ ] **Step 2: Build candidates in deterministic order**

Use:

```rust
let resource_dir = app.path().resource_dir()?;
let candidates = vec![
    resource_dir.join("skill"),
    resource_dir.join("skills").join("oasis-wiki"),
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("skill"),
];
```

Pass the selected path to the existing `install_skill_from_dir` flow without changing activation, cleanup, or rollback behavior.

- [ ] **Step 3: Run focused Rust tests and verify GREEN**

```powershell
cargo test skill::tests::resolve_skill_source --manifest-path src-tauri/Cargo.toml
```

Expected: all resolver tests pass.

### Task 5: Package the Plugin and Synchronize Skill Copies

**Files:**
- Create: `.codex-plugin/plugin.json`
- Create: `skills/oasis-wiki/**`
- Modify: `scripts/sync-bundled-skill.ps1`
- Modify: `README.md`

- [ ] **Step 1: Add the validated plugin manifest**

Use plugin name `oasis-wiki-comp`, version `1.260817.5`, `skills: "./skills/"`, repository metadata for `mislw/oasis-wiki-comp`, and the existing Companion icons.

- [ ] **Step 2: Mirror the canonical Skill to both targets**

Update `scripts/sync-bundled-skill.ps1` to mirror `../oasis-wiki/oasis-wiki` into:

```text
src-tauri/resources/skill
skills/oasis-wiki
```

Exclude `__pycache__` and `*.pyc`, but synchronize `VERSION` so every distributable copy stays identical.

- [ ] **Step 3: Run the synchronization script**

```powershell
powershell -ExecutionPolicy Bypass -File scripts/sync-bundled-skill.ps1 -SkillRepository ..\oasis-wiki
```

Expected: both targets contain `SKILL.md` and `VERSION` with no Python cache files.

- [ ] **Step 4: Update repository documentation**

Describe the repository as both a Companion application and Codex plugin, and document that plugin installation automatically discovers `skills/oasis-wiki` while MSI installation uses `src-tauri/resources/skill`.

- [ ] **Step 5: Run packaging tests and plugin validation**

```powershell
node --test tests/plugin-packaging.test.mjs
python C:\Users\ASUS\.codex\skills\.system\plugin-creator\scripts\validate_plugin.py .
```

Expected: packaging parity passes and the validator reports a valid plugin.

### Task 6: Remove the False Stub Warning and Bump Companion

**Files:**
- Modify: `src/windows/Settings.tsx`
- Modify: `package.json`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `tauri.build.conf.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/skill/mod.rs`
- Modify: `src-tauri/resources/skill/VERSION`
- Modify: `skills/oasis-wiki/VERSION`
- Modify: `.codex-plugin/plugin.json`

- [ ] **Step 1: Replace the speculative confirmation**

Use a confirmation that states the actual `EXPECTED_VERSION` and that selected Agent targets will be replaced. Do not mention a possible stub.

- [ ] **Step 2: Synchronize release versions**

Set canonical fields to `1.260817.5` and `tauri.build.conf.json` to `1.26.817+5`. Update the root package entry in `Cargo.lock` through Cargo rather than hand-editing dependency records.

- [ ] **Step 3: Run version and UI policy tests**

```powershell
node --test tests/companion-versioning.test.mjs tests/plugin-packaging.test.mjs
```

Expected: all canonical, MSI, plugin, bundled, and UI assertions pass.

### Task 7: Verify Source, Plugin, and Installer Paths

**Files:**
- No new production files.

- [ ] **Step 1: Run the complete focused suite**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm run test:hierarchy
npm run test:version
node --test tests/plugin-packaging.test.mjs
powershell -ExecutionPolicy Bypass -File src-tauri/resources/skill/scripts/check-skill-hygiene.ps1
python -m unittest discover -s src-tauri/resources/skill/tests -p "test_*.py"
python C:\Users\ASUS\.codex\skills\.system\plugin-creator\scripts\validate_plugin.py .
```

Expected: every command exits `0`.

- [ ] **Step 2: Verify cloned-source fallback**

Run the resolver test with a missing packaged candidate and a valid `src-tauri/resources/skill/SKILL.md`; confirm the source candidate is selected without modifying any Agent installation.

- [ ] **Step 3: Build the Windows installer**

```powershell
npm run tauri build -- --config tauri.build.conf.json
```

Expected: a `1.26.817+5` MSI is generated under `src-tauri/target/release/bundle/msi` and contains `resources/skill/SKILL.md`.

- [ ] **Step 4: Install/restart and run the version gate**

Install the generated MSI, restart Oasis Companion, then run:

```powershell
python src-tauri/resources/skill/scripts/check_companion_skill_versions.py
```

Expected: exit code `0` and `status=match` for the running Companion process.

### Task 8: Publish the Companion Plugin Release

**Files:**
- Commit only files named by this plan.

- [ ] **Step 1: Review the isolated diff**

```powershell
git status --short
git diff --check
git diff --stat
```

Confirm no `src-tauri/src/mcp/*`, `pnpm-lock.yaml`, or `pnpm-workspace.yaml` changes are included.

- [ ] **Step 2: Commit the implementation**

```powershell
git add .codex-plugin/plugin.json skills/oasis-wiki scripts/sync-bundled-skill.ps1 README.md tests/plugin-packaging.test.mjs tests/companion-versioning.test.mjs src-tauri/src/skill/mod.rs src/windows/Settings.tsx package.json src-tauri/tauri.conf.json tauri.build.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/resources/skill
git commit -m "Release Oasis plugin Skill distribution 1.260817.5"
```

- [ ] **Step 3: Push GitHub and verify remote state**

```powershell
git push origin main
git status -sb
git rev-list --left-right --count origin/main...HEAD
```

Expected: the branch is clean and the ahead/behind count is `0 0`.
