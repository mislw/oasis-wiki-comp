# Single Repository Maintenance Design

## Goal

Make `mislw/oasis-wiki-comp` the only actively maintained repository for the Oasis Wiki Skill, Codex plugin, and Windows Companion.

## Repository Ownership

- `skills/oasis-wiki` is the canonical Skill source.
- `src-tauri/resources/skill` is a generated mirror used by the Companion installer.
- `scripts/sync-bundled-skill.ps1` mirrors the canonical in-repository Skill into the Tauri resource directory without requiring a sibling checkout.
- Releases, tags, plugin ZIP files, and future Companion installers are published only from `mislw/oasis-wiki-comp`.

## Update Compatibility

New settings default to `mislw/oasis-wiki-comp`. Existing settings that still contain the exact legacy repository `mislw/oasis-wiki` are migrated on load and persisted. Any other custom repository value is preserved.

The existing updater recursively locates `SKILL.md`, so GitHub source archives from the unified repository can install `skills/oasis-wiki` without changing archive extraction behavior.

## Legacy Repository

Before archiving `mislw/oasis-wiki`, update its README to identify `mislw/oasis-wiki-comp` as the active repository and direct users to the unified releases. Keep its source, tags, and releases intact so older Companion installations can still read the final legacy release.

## Verification

- Static packaging tests confirm the canonical source, default repository, UI placeholder, and local sync path.
- Rust tests confirm legacy settings migration and custom repository preservation.
- Companion Node tests, Rust tests, frontend build, plugin validation, and Skill tests pass.
- GitHub reports the legacy repository as archived only after the unified repository changes are pushed.
