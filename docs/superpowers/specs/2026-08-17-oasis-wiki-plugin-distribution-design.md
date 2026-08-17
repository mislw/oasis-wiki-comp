# Oasis Wiki Plugin Distribution Design

## Goal

Make `oasis-wiki-comp` the single supported Codex installation entry. When a user installs the plugin, Codex downloads the plugin package and discovers the bundled `oasis-wiki` Skill automatically. Users must not need to install the Skill separately.

## Constraints

- A standalone Skill has no trusted post-install hook and must not silently execute a downloader.
- Plugin installation may download only the declared plugin package; it must not install the Companion MSI or run arbitrary executables.
- The existing standalone Skill repository and Companion MSI remain supported secondary distribution paths.
- RedCliff project files and private project data are outside this change.

## Architecture

The `oasis-wiki-comp` repository becomes a valid Codex plugin root:

```text
.codex-plugin/plugin.json
skills/oasis-wiki/
  SKILL.md
  VERSION
  references/
  scripts/
  assets/
```

The plugin manifest declares `skills: "./skills/"`. Codex installs the repository as one plugin package and discovers `skills/oasis-wiki/SKILL.md` from that package. The Skill therefore arrives as part of plugin installation rather than downloading a second package after installation.

The existing `src-tauri/resources/skill/` bundle remains the Companion MSI copy. A synchronization command updates both `src-tauri/resources/skill/` and `skills/oasis-wiki/` from the same canonical Skill source.

## Marketplace Metadata

The plugin keeps the stable ID `oasis-wiki-comp` and the display name `Oasis Wiki`. Its marketplace entry uses the normal install action; installing that entry downloads the complete plugin package, including the Skill.

The marketplace entry must not claim that installing the plugin also installs the Windows Companion application. Companion installation remains an explicit MSI action because it requires Windows installation privileges.

## Version Contract

- The plugin manifest version, Companion canonical version, bundled Skill `VERSION`, plugin Skill `VERSION`, and standalone Skill `VERSION` must describe the same release.
- Plugin cache-buster metadata may append valid SemVer build metadata, but the canonical version remains `M.YYMMDD.N`.
- A version consistency test fails when any packaged copy drifts.

## Update Flow

1. Update the canonical `oasis-wiki` Skill source.
2. Run the synchronization command.
3. Validate the plugin manifest and both packaged Skill copies.
4. Run Skill, plugin, Companion, and version regression tests.
5. Publish the plugin repository and marketplace metadata.
6. A user installs or updates `oasis-wiki-comp`; Codex replaces the plugin package and discovers the matching Skill automatically.

## Failure Handling

- Missing `.codex-plugin/plugin.json` or `skills/oasis-wiki/SKILL.md`: plugin validation fails before publication.
- Skill copies differ: synchronization/version tests fail before publication.
- Plugin installation fails: Codex keeps the prior installed plugin; no standalone Skill downloader runs.
- Companion MSI is absent: the Skill remains usable, while desktop-only Companion features report that the Companion is unavailable.
- A user installs only the standalone Skill: the Skill works, but it does not silently install the plugin; documentation directs new users to the plugin entry.

## Validation

- Run the plugin manifest validator from the `plugin-creator` Skill.
- Assert the manifest points to `./skills/` and the `oasis-wiki` Skill is discoverable.
- Assert `skills/oasis-wiki/` matches `src-tauri/resources/skill/`, excluding explicitly generated or private-only files.
- Run Skill hygiene and version consistency tests.
- Run an installation smoke test against an isolated Codex home or plugin cache when the local Codex plugin CLI is executable.
- Verify the published repository contains no build targets, credentials, private project files, or local caches.

## Non-Goals

- Automatically installing or elevating the Companion MSI.
- Running post-install shell scripts from a Skill.
- Automatically installing Cowart or unrelated marketplace plugins.
- Changing UGC project files or UI runtime behavior.
