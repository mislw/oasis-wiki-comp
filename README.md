# Oasis Companion

Oasis Companion is the Windows desktop companion for the `oasis-wiki` Codex Skill.
It provides a floating Oasis ball, tray controls, a settings UI, Agent detection,
MCP discovery and calls, multi-Agent Skill installation, and GitHub-backed updates.

## Install

Download the MSI from the GitHub Releases page and run it. The installer uses a
machine-wide location under `C:\\Program Files` and may request administrator
approval when installing or upgrading.

The bundled Skill is installed to:

```text
%USERPROFILE%\.codex\skills\oasis-wiki
```

Runtime settings are stored separately at:

```text
%USERPROFILE%\.oasis-companion\settings.json
```

After installation, launch Oasis Companion once and restart Codex. The companion
stays in the system tray and shows the floating ball while a supported Agent runs.

## Build From Source

Prerequisites: Node.js 18+, Rust with the MSVC toolchain, Microsoft C++ Build Tools,
and WebView2 Runtime.

```powershell
npm install
npm run build
npm run tauri build
```

The Windows MSI is generated under:

```text
src-tauri\target\release\bundle\msi\
```

## Bundled Skill

The canonical Skill source lives in `mislw/oasis-wiki`. Companion mirrors its
`oasis-wiki/` directory into both `src-tauri/resources/skill/` for the Windows
installer and `skills/oasis-wiki/` for the Codex plugin. Run
`scripts/sync-bundled-skill.ps1` after updating the sibling Skill checkout.

After a successful Skill update, Companion removes installer-owned backup and
temporary directories plus other directories whose `SKILL.md` declares the exact
name `oasis-wiki` in that Agent's registered Skill locations. The canonical target
is preserved, unrelated Skills are left untouched, and a failed activation restores
the previous canonical installation.

## Repository Layout

- `src/`: React and TypeScript UI, including the settings tabs and floating ball.
- `src-tauri/src/`: Tauri and Rust runtime, configuration, Agent detection, MCP,
  Skill installation, tray, autostart, and updater modules.
- `src-tauri/resources/skill/`: read-only Skill resources bundled into the installer.
- `skills/oasis-wiki/`: complete Skill discovered automatically when Codex installs the plugin.
- `docs/`: release design and implementation plans.

This repository is distributed both as a Codex plugin and as a Windows Companion
application. Installing the plugin downloads and discovers the complete Skill.
Installing the MSI bundles the same Skill for Companion-managed Agent targets; it
does not require or silently install the Codex plugin.
