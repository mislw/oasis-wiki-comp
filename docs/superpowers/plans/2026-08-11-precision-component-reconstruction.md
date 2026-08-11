# Precision Component Reconstruction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a tested, bundled Cowart workflow for extracting and reconstructing reusable UI components from approved flattened UI images.

**Architecture:** Markdown describes Stage 2A recognition and Stage 2B precision reconstruction. Python scripts create and validate an extraction plan, produce one job for each reusable target, recompose available transparent assets, and report review-required results. No script claims to perform AI image edits.

**Tech Stack:** Python standard library, `unittest`, Node.js built-in test runner, PowerShell.

---

### Task 1: Lock extraction-plan behavior with tests

**Files:**
- Create: `src-tauri/resources/skill/tests/test_precision_component_reconstruction.py`
- Create: `src-tauri/resources/skill/tests/cowart-ui-category.test.mjs`

- [x] **Step 1: Write failing tests** for reusable target consolidation, native bitmap rejection, unsafe skin rejection, one job per target, inactive-only statuses, and bundled-file presence.
- [x] **Step 2: Run the tests** and verify they fail because the Cowart scripts and assets do not exist.

### Task 2: Add plan creation and validation

**Files:**
- Create: `src-tauri/resources/skill/assets/cowart-ui/extraction-plan-template.json`
- Create: `src-tauri/resources/skill/scripts/cowart-ui/component-extractor/build_extraction_plan.py`
- Create: `src-tauri/resources/skill/scripts/cowart-ui/component-extractor/validate_extraction_plan.py`

- [x] **Step 1: Implement minimal plan grouping** by `target_component_id`.
- [x] **Step 2: Implement plan validation** for source metadata, modes, statuses, outputs, bounds, and reconstruction constraints.
- [x] **Step 3: Re-run Python tests** and confirm the covered validations pass.

### Task 3: Add reconstruction and review tools

**Files:**
- Create: `src-tauri/resources/skill/scripts/cowart-ui/component-extractor/build_reconstruction_jobs.py`
- Create: `src-tauri/resources/skill/scripts/cowart-ui/component-extractor/recompose_ui.py`
- Create: `src-tauri/resources/skill/scripts/cowart-ui/component-extractor/validate_reconstruction.py`

- [x] **Step 1: Generate one job per reusable target** without performing image generation.
- [x] **Step 2: Add deterministic preview composition and alpha-aware reconstruction validation.**
- [x] **Step 3: Run the Python suite** and confirm every workflow boundary passes.

### Task 4: Publish the workflow in the bundled Skill

**Files:**
- Create: `src-tauri/resources/skill/references/cowart-ui/precision-reconstruction.md`
- Create: `src-tauri/resources/skill/references/cowart-ui/component-extractor.md`
- Create: `src-tauri/resources/skill/references/cowart-ui-workflow.md`
- Modify: `src-tauri/resources/skill/SKILL.md`
- Modify: `src-tauri/resources/skill/AGENTS.md`

- [x] **Step 1: Document Stage 2A and Stage 2B** with native boundaries, multi-instance grouping, nine-slice assessment, and review gates.
- [x] **Step 2: Link the guide from both portable entry points.**
- [x] **Step 3: Run Node presence checks and Skill hygiene validation.**

### Task 5: Verify and publish

**Files:** Git history and bundled Skill resources.

- [x] **Step 1: Run Python, Node, hygiene, application build, and Rust checks where dependencies permit.**
- [x] **Step 2: Inspect the diff** for scope and generated artifacts.
- [ ] **Step 3: Commit and push** `codex/precision-component-reconstruction` to GitHub.
