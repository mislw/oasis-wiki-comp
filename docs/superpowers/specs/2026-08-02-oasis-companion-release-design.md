# Oasis Companion 可发布化设计

## 目标

将现有 Windows `oasis-companion` 整理并发布到 `mislw/oasis-wiki-comp`，随安装包分发 `mislw/oasis-wiki` 的完整 Skill，并保证安装、更新、失败回滚和多 Agent 路径处理不会破坏用户已有 Skill。

本次同步以上游 `mislw/oasis-wiki` 的 `main` 分支提交 `b3a5997` 为初始基线。上游仓库继续作为 UGC 知识、references 和工具脚本的唯一事实来源；Companion 仓库不独立维护这些内容。

## 发布形态

仓库发布一个 Tauri 2 Windows Companion，而不是把桌面程序伪装成 Codex marketplace plugin。程序提供悬浮球、托盘、Agent 检测、运行时设置、Skill 安装和显式更新功能。

完整 Skill 作为只读构建资源放在 `src-tauri/resources/skill/`。仓库提供同步脚本，从一个已检出的 `mislw/oasis-wiki` 工作树复制 `oasis-wiki/`，记录上游 commit，并在构建和 CI 中检查资源完整性。同步只允许改变构建资源，不反向修改上游仓库。

## 仓库结构

- `src/`：React 设置页和悬浮球界面。
- `src-tauri/src/`：配置、Agent 注册、安装器、更新器、MCP 检测和桌面生命周期。
- `src-tauri/resources/skill/`：从上游同步的完整 `oasis-wiki` Skill。
- `scripts/sync-oasis-wiki.ps1`：显式同步上游工作树并记录修订。
- `scripts/check-bundled-skill.ps1`：验证 Skill 名称、入口、核心 references、脚本、UI 元数据和修订文件。
- `docs/`：Companion 架构、开发和发布说明。

不提交 `node_modules/`、`dist/`、Rust `target/`、安装包、用户配置、项目缓存、UGC 私有项目文件或 `.uasset` 备份。

## 上游同步模型

同步脚本接收 `-SourceRepository`，要求目录是干净的 Git 工作树并包含 `oasis-wiki/SKILL.md`。脚本读取 `HEAD` 完整 SHA，先复制到临时目录并执行完整性检查，成功后才替换 bundled Skill。

同步结果包含：

- 完整上游 `oasis-wiki/` 内容；
- `UPSTREAM_COMMIT`，保存 40 位 commit SHA；
- `VERSION`，保存用于旧版状态显示的短修订标识；
- 不向上游 `SKILL.md` frontmatter 注入 `version` 或 Companion 专属字段。

上游 Skill 的 `agents/openai.yaml`、references、wiki 和 scripts 必须原样保留。Companion 的运行时配置说明如果需要变更，应在上游先完成，再重新同步，防止两份 `SKILL.md` 漂移。

## 安装与回滚

安装仅接受注册表中的 target ID。未知 target 直接返回错误，绝不把 ID 当成路径。

每个目标独立执行：

1. 将来源复制到目标同级的唯一临时目录。
2. 验证临时副本的 `SKILL.md` 名称为 `oasis-wiki`，并验证完整资源和上游修订。
3. 将现有目标重命名为备份；重命名失败立即停止。
4. 将已验证的临时目录原子重命名为目标。
5. 若第 4 步失败，自动恢复备份。
6. 安装成功后保留最近三个备份并清理临时目录。

多目标安装返回每个目标的结果，不把部分成功伪装成整体成功。Codex 使用 `%USERPROFILE%\.codex\skills\oasis-wiki`；Claude Code 使用 `%USERPROFILE%\.claude\skills\oasis-wiki`；WorkBuddy 路径维持现有注册值，但必须通过独立发现测试后才在 UI 中默认启用。

## 版本与更新

Companion 应用版本、bundled Skill 修订、实际安装修订分开管理：

- 应用版本来自 Tauri/Cargo/package 元数据。
- bundled Skill 修订来自 `src-tauri/resources/skill/UPSTREAM_COMMIT`。
- installed Skill 修订来自目标目录的 `UPSTREAM_COMMIT`，缺失时显示 unknown，不谎报为当前内置版本。

GitHub 更新检查比较远端 revision 与 installed revision。更新下载设置响应体上限、ZIP 文件数上限、单文件和总解压大小上限；只接受名称为 `oasis-wiki` 且通过完整性验证的 Skill 根目录。下载、验证或安装失败时保持当前安装不变，并清理临时解压目录。

更新继续由用户显式触发，不做后台静默覆盖。HTTPS GitHub 下载提供传输安全；发布流程额外生成 SHA-256 清单，供可复现检查和后续签名扩展。

## 配置

运行时配置统一为 `%USERPROFILE%\.oasis-companion\settings.json`。README、技术设计、界面提示和 Skill 中的说明必须一致。

项目级 `.oasis-wiki/settings.json` 由 Agent 按 Skill 规则读取；Companion 只管理全局桌面设置和 MCP 连接参数，不扫描或改写 UGC 项目配置。安全锁定规则不能被项目配置覆盖。

## 错误处理

- 未知 target、错误 Skill 名称、缺失资源、脏上游工作树和无效 revision 都给出明确错误。
- 安装失败优先恢复旧目录；恢复失败时同时报告目标、备份和临时目录的绝对路径，便于人工恢复。
- 更新包拒绝路径穿越、过大文件、过多文件和多个含糊 Skill 根目录。
- 所有临时目录在成功和失败路径都尽力清理，但绝不为了清理而删除未验证的宽泛路径。

## 测试策略

Rust 单元/集成测试先覆盖失败路径，再改实现：

- 未知 target 被拒绝；
- 缺失或错误名称的 Skill 被拒绝；
- 无 `UPSTREAM_COMMIT` 时版本为 unknown；
- 已安装 revision 与 bundled revision 分开报告；
- 替换失败时恢复原目录；
- 部分目标成功时保留逐目标结果；
- ZIP 路径穿越、大小和文件数限制；
- 下载或验证失败不改变现有安装。

PowerShell 检查验证同步包包含 `SKILL.md`、`agents/openai.yaml`、核心 references、wiki 和必需 scripts。前端执行 TypeScript/Vite 构建；Rust 执行格式检查、Clippy 和全部测试。发布前还要在隔离临时目录完成一次真实同步与安装演练，确认不会覆盖本机 `%USERPROFILE%` 下的实际 Skill。

## 文档与发布

README 说明仓库定位、上游关系、开发依赖、同步命令、构建命令、安装路径、备份恢复和更新安全边界。Tauri bundle 目标统一为实际发布的 Windows 安装器类型，文档与产物目录保持一致。

实现完成后提交到 `main` 并推送 `mislw/oasis-wiki-comp`。由于远端当前为空，首个实现历史保留设计提交和后续按功能拆分的提交，不压成一个不可审查的大提交。

## 非目标

- 不修改或发布 RedCliff 等私有 UGC 项目内容。
- 不在 Companion 仓库独立演进 `oasis-wiki` 知识。
- 不在本阶段创建 Codex marketplace plugin 或个人 marketplace 条目。
- 不静默更新 Companion 或 Skill。
- 不实现与本次安全发布无关的 UI 重设计。
