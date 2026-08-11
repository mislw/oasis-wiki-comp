# Precision Component Reconstruction Design

## Goal

Add a reusable Cowart UI workflow to the bundled `oasis-wiki` Skill. The workflow turns an approved flattened UI image into reviewable, reusable UI assets without treating screenshot crops as real components.

## Scope

The workflow is bundled under `src-tauri/resources/skill/`. It adds a reference guide, an extraction-plan template, deterministic local scripts, and tests. It does not edit UGC projects, generate AI artwork itself, or automatically promote any reconstructed asset to an active component.

## Pipeline

1. Use the existing visual approval and UI-tree artifacts as input.
2. Classify each node as `native`, `extract_artwork`, `reconstruct_skin`, or `composite`.
3. Merge visually equivalent instances by `target_component_id` into one extraction-plan component.
4. Validate the plan before generating one AI-edit job per reusable target.
5. Require transparent PNG output for artwork and skins; leave native and composite nodes without bitmap output.
6. Recompose generated assets at every recorded instance position and report missing files, missing alpha, invalid status, and incomplete previews.
7. Leave all generated assets in `candidate` or `pending_review` until a developer performs the existing Stage 3 confirmation.

## Boundaries

- Text, numbers, prices, timers, progress, labels, and hit targets remain native controls.
- Skins containing baked content require `remove_content` unless their source is declared clean.
- Reconstructed assets are not proof of visual correctness; the preview requires developer review.
- Scripts plan, validate, and compose files only. Semantic removal, masking, alpha extraction, and occlusion repair are delegated to an image-edit model or developer.

## Packaging

The new files live inside the Companion's bundled Skill and are linked from both `SKILL.md` and `AGENTS.md`. The existing Skill hygiene check remains the packaging gate.

## Verification

Python `unittest` covers plan consolidation and validation boundaries. A Node test verifies every new Cowart file is present in the bundled resource tree. The skill hygiene check verifies the resulting bundle shape.
