# Oasis Companion

Oasis Companion is the Windows desktop companion for the `oasis-wiki` Codex Skill.
It provides a floating Oasis ball, tray controls, a settings UI, Agent detection,
MCP discovery and calls, multi-Agent Skill installation, and GitHub-backed updates.

## Install

Download the MSI from the GitHub Releases page and run it. The installer uses the
per-user `currentUser` mode and does not require administrator elevation.

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

The complete upstream Skill lives in `src-tauri/resources/skill/` and is copied
from `mislw/oasis-wiki`. It includes `SKILL.md`, Agent metadata, references, wiki
content, and MCP helper scripts.

## Repository Layout

- `src/`: React and TypeScript UI, including the settings tabs and floating ball.
- `src-tauri/src/`: Tauri and Rust runtime, configuration, Agent detection, MCP,
  Skill installation, tray, autostart, and updater modules.
- `src-tauri/resources/skill/`: read-only Skill resources bundled into the installer.
- `docs/`: release design and implementation plans.

This project is distributed as a Windows Companion application, not as a Codex
marketplace plugin. The Skill and the desktop companion are installed together by
the MSI, but the Skill itself remains independently usable by Codex.
