# Single Repository Maintenance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `mislw/oasis-wiki-comp` the sole active source and release repository while preserving legacy update compatibility.

**Architecture:** Keep `skills/oasis-wiki` canonical inside the Companion repository and generate only the Tauri resource mirror. Migrate the exact legacy GitHub repository setting during config load, then archive the legacy repository after publishing a redirect notice.

**Tech Stack:** Rust, Tauri 2, React/TypeScript, Node test runner, PowerShell, GitHub CLI.

**Spec:** `docs/superpowers/specs/2026-09-28-single-repository-maintenance-design.md`

## Global Constraints

- Preserve custom GitHub update repositories.
- Preserve the old repository's source, tags, and releases.
- Do not include unrelated Companion window changes.
- Keep `skills/oasis-wiki` and `src-tauri/resources/skill` byte-identical after synchronization.

---

### Task 1: Single-Repository Source And Update Defaults

**Files:**
- Create: `tests/single-repository-maintenance.test.mjs`
- Modify: `scripts/sync-bundled-skill.ps1`
- Modify: `src-tauri/src/config/schema.rs`
- Modify: `src/windows/Settings.tsx`
- Modify: `README.md`

- [ ] Write and run the failing static test for the new canonical source and update repository.
- [ ] Change the sync script to mirror `skills/oasis-wiki` into `src-tauri/resources/skill`.
- [ ] Change the default repository and settings placeholder to `mislw/oasis-wiki-comp`.
- [ ] Document the single-repository workflow and rerun the test.

### Task 2: Existing Settings Migration

**Files:**
- Modify: `src-tauri/src/config/mod.rs`

- [ ] Write Rust tests for legacy migration and custom repository preservation.
- [ ] Run the tests and confirm the missing migration fails.
- [ ] Implement exact-value migration during config load and persist migrated settings.
- [ ] Run Rust and Node test suites.

### Task 3: Publish And Archive

**Files:**
- Modify in legacy repository: `README.md`

- [ ] Validate Skill mirror parity, frontend build, plugin manifest, and repository status.
- [ ] Commit and push the unified repository changes.
- [ ] Push the legacy README redirect.
- [ ] Archive `mislw/oasis-wiki` through the GitHub API.
- [ ] Verify the unified repository is active and the legacy repository is archived.
