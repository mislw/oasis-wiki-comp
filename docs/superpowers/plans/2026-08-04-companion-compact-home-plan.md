# Oasis Companion Compact Home Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the default settings overview with a compact single-column control surface while preserving all existing detail pages and commands.

**Architecture:** Keep `Settings.tsx` as the stateful orchestration component. Add a small home-only render branch that reuses existing `settings`, `mcpStatus`, `skill`, `update`, `setRuntimeMode`, `connectMcpAuto`, `closeSettings`, and tab state. Scope new CSS to `.compact-home`; leave detail-page styles and behavior intact.

**Tech Stack:** React 19, TypeScript, Tauri invoke/events, Vite CSS.

---

### Task 1: Add compact home render

**Files:**
- Modify: `src/windows/Settings.tsx` around the main return branch (lines 497-930)

- [x] Add a `compact-home` branch before the existing tabs/nav return. Render header, mode segmented control, MCP status/action, footer summary, and gear button.
- [x] Wire existing handlers: `setRuntimeMode`, `connectMcpAuto`, `closeSettings`, and `setActiveTab`.
- [x] Keep the existing tabbed return intact below the branch so detail pages remain reachable.

### Task 2: Add compact home styling

**Files:**
- Modify: `src/index.css` under settings-window styles

- [x] Add fixed narrow dimensions, reduced padding, single-column spacing, status colors, and footer summary styles scoped to `.compact-home`.
- [x] Keep the full settings page scrollable and unchanged.
- [x] Add responsive fallback so the compact home remains usable under 340px width.

### Task 3: Verify build and layout

**Files:**
- Test: `npm run build`

- [x] Run the TypeScript/Vite build and confirm it passes without new diagnostics.
- [x] Inspect the compact layout mockup and verify the mode control, MCP primary action, settings gear, and footer summary fit the narrow surface; native Tauri sizing is synchronized to 380x390.
