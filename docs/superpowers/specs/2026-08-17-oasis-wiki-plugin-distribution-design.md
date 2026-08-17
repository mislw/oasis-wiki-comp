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

## Companion Skill Source Resolution

Companion resolves a trusted Skill source in this order:

1. Packaged Tauri resource: `<resource_dir>/skill`.
2. Installed plugin package: `<resource_dir>/skills/oasis-wiki`.
3. Source checkout fallback: `<CARGO_MANIFEST_DIR>/resources/skill`.

A candidate is valid only when it is a directory containing `SKILL.md`. The packaged resource remains authoritative for MSI installations, while the plugin and source-checkout candidates make Git/plugin development layouts usable without copying a standalone executable beside an unstaged `skill` directory.

If no candidate is valid, the error lists every attempted absolute path and explains that the user must run the packaged installer, install the complete plugin, or keep `src-tauri/resources/skill` in the source checkout. It must not suggest that an arbitrary untrusted download directory is acceptable.

## Reinstall UI

The reinstall confirmation describes the actual bundled Skill version and selected targets. It must not unconditionally claim that the bundled Skill may be a minimal stub. Missing or invalid Skill resources are reported by the resolver with the attempted paths instead of a speculative warning before every reinstall.

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
- Companion source resolution finds no valid `SKILL.md`: installation stops before modifying any Agent target and reports all attempted trusted paths.
- Plugin installation fails: Codex keeps the prior installed plugin; no standalone Skill downloader runs.
- Companion MSI is absent: the Skill remains usable, while desktop-only Companion features report that the Companion is unavailable.
- A user installs only the standalone Skill: the Skill works, but it does not silently install the plugin; documentation directs new users to the plugin entry.

## Validation

- Run the plugin manifest validator from the `plugin-creator` Skill.
- Assert the manifest points to `./skills/` and the `oasis-wiki` Skill is discoverable.
- Assert `skills/oasis-wiki/` matches `src-tauri/resources/skill/`, excluding explicitly generated or private-only files.
- Assert Companion source resolution prefers packaged resources, falls back to plugin/source layouts, rejects directories without `SKILL.md`, and reports all attempted paths.
- Assert the settings UI contains no unconditional minimal-stub warning.
- Run Skill hygiene and version consistency tests.
- Run an installation smoke test against an isolated Codex home or plugin cache when the local Codex plugin CLI is executable.
- Run a cloned-source smoke test where the executable resource directory has no staged `skill` folder but `src-tauri/resources/skill/SKILL.md` exists.
- Verify the published repository contains no build targets, credentials, private project files, or local caches.

## Non-Goals

- Automatically installing or elevating the Companion MSI.
- Running post-install shell scripts from a Skill.
- Automatically installing Cowart or unrelated marketplace plugins.
- Changing UGC project files or UI runtime behavior.
