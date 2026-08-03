# Oasis Companion Publish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将当前包含新版 UI、MCP 和多 Agent 支持的 Oasis Companion 与完整 oasis-wiki Skill 发布到 GitHub。

**Architecture:** 以 `oasis-companion` 当前源码为应用源，将完整上游 Skill 同步到 Companion 的 bundled resources；发布仓库包含源码、构建配置和安装说明，不提交 `node_modules`、`dist` 或 Rust `target`。构建生成 MSI，并通过 GitHub Release 提供安装包。

**Tech Stack:** Tauri 2, Rust, React 19, TypeScript, Vite, PowerShell, GitHub CLI.

---

### Task 1: 整理发布仓库

**Files:** `oasis-wiki-comp/`

- [x] 复制 `oasis-companion` 源码和必要文档到发布仓库。
- [x] 同步 `oasis-wiki-upstream/oasis-wiki` 到 `src-tauri/resources/skill`。
- [x] 排除构建缓存、依赖目录、截图和本地配置。

### Task 2: 验证应用和 Skill

**Files:** `oasis-wiki-comp/src-tauri/resources/skill/`

- [x] 运行 `npm run build` 验证前端类型和 Vite 构建。
- [x] 运行 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings` 和 `cargo test`。
- [x] 运行 `npm run tauri build` 生成 Windows MSI。

### Task 3: 提交并发布

**Files:** Git history and GitHub release

- [x] 检查差异，仅暂存发布仓库内容。
- [x] 创建发布分支、提交并推送到 `mislw/oasis-wiki-comp`。
- [x] 创建 `v0.1.0` GitHub Release，上传新 MSI，并在 README 说明安装方式。
