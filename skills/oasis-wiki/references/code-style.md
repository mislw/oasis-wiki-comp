# UGC Code Style

Use this reference whenever writing or reviewing Oasis/绿洲启元/和平精英 UGC Lua code, especially when adding config tables, member variables, methods, or `GlobalConfig` entries.

## Core Rules

0. Apply `predecessor-code-policy.md` before editing. Existing teammate/predecessor code is protected. In RedCliff, blocks identified by `git blame` as belonging to the protected predecessor are frozen by default: do not change even one line when an additive hook elsewhere can solve the task. Do not directly rewrite, restructure, wrap, rename, reorder, or refactor existing blocks for cleanliness. If a protected block truly must change, first prove no compatible external hook exists, explain the necessity, and limit the edit to the exact affected lines.
1. New config tables must include Chinese comments for every column.
2. English words in config table column names and variable names should be spelled out completely. Abbreviations already established in the codebase are acceptable (`ID`, `UI`, `Hp`, `Exp`, `UID`, `DS`, `BGM`); do not invent new abbreviations, and do not rename existing `Id`/`ID` spellings just to unify them.
3. Simple typed variables should use lightweight type prefixes:
   - `nLevel`: number
   - `szName`: string
   - `tbItemList`: table/list
   - `bOpened`: boolean
   Prefixes apply to plain data in member variables, parameters, and locals. Object/config/manager references use plain lowerCamel (`playerController`, `towerLevelConfig`, `virtualItemManager`); engine delegate parameters keep their engine names (`MobPawn`, `KillerController`).
4. Every member variable and every `GlobalConfig` config variable must have a Chinese comment.
5. Every method must have a Chinese comment explaining its purpose; the engine lifecycle overrides (`Construct`, `Destruct`, `ReceiveBeginPlay`, `ReceiveTick`, `Tick`) are exempt because their names are self-explanatory.
6. Source-code comments must describe behavior, data flow, ownership, or reasons only. Never write `前辈`、`前輩`, author names, or predecessor attribution into generated or edited code comments; keep any attribution only in the user-facing explanation.
7. Keep defensive code narrow. Only guard real boundary risks such as user input, missing config, RPC/network payloads, async UI lifecycle, destroyed actors, or optional data. For internal code with trusted required values, keep the flow direct; if an impossible invalid value appears and there is no clear recovery path, let it error so the real bug is exposed instead of hiding it behind noisy `if` branches. In particular, do not repeatedly wrap each block with checks like `if CauserActor and UE.IsValid(CauserActor) then` when the same actor/context is required by the whole calculation flow.
8. Use the protected predecessor's confirmed same-type implementation as the RedCliff style baseline: match data structures, naming, function placement, call order, comments, guard density, and minimal diff size. Bug fixes should usually be a few changed lines, ordinary feature hooks should stay close to the existing entry point, and larger diffs need a real feature reason rather than extra protection layers. Prefer direct project-style code over generic guard/wrapper scaffolding.

## How To Apply

- Put comments close to the variable, table field, or method they explain.
- For config tables, comment the meaning of each column, not only the table itself.
- For methods, explain what the method is responsible for and which side it runs on when relevant, such as server, client, UI, GameState, GameMode, PlayerController, Pawn, or Action.
- When adding behavior to an existing file, prefer a new helper, a new config entry, a new event/RPC hook, or a small appended branch over replacing the original function body.
- Add helpers only when they remove real repeated code or match an existing project pattern. Do not create helper layers just to make one small change look more systematic.
- Keep existing project naming when editing old code. Apply this style most strongly to new code, new config fields, new member variables, and newly added methods.
- Do not rename old fields only to satisfy style unless the user explicitly asks for cleanup, because renaming config keys, RPC names, event IDs, or save keys can break existing behavior.
- Avoid boilerplate nil/validity checks at every step when the value is a required invariant. Add a guard only when the code can make a useful decision after the guard, such as logging a clear config error, returning from a UI callback after a widget was closed, rejecting bad client input, or using a documented fallback.
- If a required object must be validated once for readability, validate it once near the boundary or function entry, then write the main logic without repeating the same `and UE.IsValid(...)` condition around every section. For calculations such as damage formulas, repeated checks around attack, tower, critical, and camp logic usually make the code harder to read without adding real recovery behavior.
- For file layout, RPC/event/UI organization, and logging format, follow the Structure Conventions below.

## Comment Style (adopted from the protected predecessor)

- Comments are written in Chinese; identifiers and API names inside comments stay in English (`GemFeature`, `PlayerProfileManager`).
- Declarations carry inline trailing comments: member variables, config-table fields, and GlobalConfig entries each get a `-- 中文说明` on the same line; align the trailing comments within one group.
- Standalone config values and file-local config tables get one or more comment lines directly above the declaration; long explanations may wrap to a second line.
- Methods get one comment line directly above `function Name:Method(...)`, describing the responsibility and, when it matters, the side or timing (`-- 客户端只在怪物波次进行中播放战斗背景音乐。`). Engine lifecycle overrides are exempt, as noted in Core Rule 5.
- Inside method bodies, comment only the business checks and non-obvious steps (`-- 防御塔是否解锁`); do not annotate every statement, and do not restate what the next line plainly does.
- Group related declarations with short comments (`-- 防御塔相关`) and split large regions with `---- 名称 ----`, `---- 名称 begin ----` / `---- 名称 end ----` markers.
- Persistent/archive fields add the ownership tag in the same trailing comment: `[CS]` when both sides hold or persist it, `[S]` when server-only. Do not invent other tag prefixes without a matching existing pattern.
- Comments record constraints and reasons (what must stay in sync, why a value exists), not narration of the code flow.
- Never write attribution (predecessor names, author names) or decorative banners into comments.

## Structure Conventions (RedCliff baseline)

### File Header And Imports

- The first line is the generated annotation `---@class ClassName_C:ParentClass_C`; UI files then list every bound widget/component with `---@field WidgetName WidgetType`, and the generated block ends with the exact marker `--Edit Below--`.
- Keep the generated header and marker intact; add imports and code only below `--Edit Below--`.
- Import order: official config bundle first (`require("Script.GameConfigs.CommonConfigPackage")`), then feature modules as `local X = require("Script....")`. Official extension packages load with `UGCGameSystem.UGCRequire("ExtendResource....")`, either once at file top or immediately before first use inside the function.

### Class Member Table

- Declare all members on a `local ClassName = { ... }` table above the functions: `tbUnlockedTower = {}, -- 已解锁的防御塔`.
- Persistent/archive fields sit in a `---- 存档数据 begin ----` section and carry an ownership tag in the comment: `[CS]` when both client and server hold or persist the field, `[S]` when server-only.
- Group members by feature with a `-- XXX相关` comment line and align the trailing comments within a group.
- UI state members also belong in this table (`nCurrentTab = 0, -- 当前选中一级分页`), not scattered as `self.x = ...` assignments in `Construct`.

### GlobalConfig Layout

- `Script/GameConfigs/GlobalConfig.lua` defines bare PascalCase globals (`MaxPlayerCount = 4`), not a `GlobalConfig` table; consumers reference the names directly.
- One Chinese comment line above each entry; multi-line explanations are allowed. Group related entries together.
- Expressions and compact list literals are normal: `CoopTicketFreeInterval = 2 * 60 * 60`, `CoopTicketBuyCostList = { 50, 100, 150 }`.
- Structured tables use keyed fields with an inline Chinese comment per field (`szParticlePath = "...", -- 燃烧资源`).

### EventDefine And Event Dispatch

- Start the file with `EventDefine = EventDefine or {}`; add events as `EventDefine.UpdateXxx = 125 -- 更新玩家战斗力`.
- Allocate separate numeric ranges per feature (for example 1xx castle/resources, 2xx stage waves, 3xx player attributes) and column-align the trailing comments.
- Name state-refresh events `UpdateXxx`, one-shot notifications `NotifyXxx`.
- UI listeners register in `Construct` with `UGCEventSystem:AddListener(EventDefine.Xxx, self.OnXxx, self)` and unregister in `Destruct` with the matching `RemoveListener`; handlers are named `OnXxx`. Dispatch with `UGCEventSystem:SendEvent(EventDefine.Xxx, ...)`.

### RPC Conventions

- Server RPC methods are named `ServerRPC_Xxx`, client RPC methods `ClientRPC_Xxx`; separate them with `---- 服务器RPC ----` and `---- 客户端RPC ----` section dividers.
- Register every server RPC in `GetAvailableServerRPCs()` as a `"ServerRPC_Xxx",` entry, appended in feature order.
- A server RPC logs player and key parameters on entry, then validates one rule at a time with a Chinese comment above each check; on failure it sends `ClientRPC_ShowTipsUI` (usually `Unreliable`) with the Chinese player-facing text, then returns. Main logic stays straight-line after validation.
- Clients call `UnrealNetwork.CallUnrealRPC(playerController, playerController, "ServerRPC_Xxx", ...)`; server-side pushes use the same helper with a `ClientRPC_Xxx` name.

### UI Callback Conventions

- Button handlers are named `Button_<WidgetName>_OnClicked(...)`; the first line logs `print("Class:Method")` (with `string.format` arguments when useful), then `UGCUIManager:PlayButtonOpenSound()` for normal buttons, then the logic.
- Bind in `Construct` (`self.Button_X.OnClicked:Add(self.Button_X_OnClicked, self)`), clear in `Destruct` (`self.Button_X.OnClicked:Clear()`).
- Generated or repeated slots are accessed dynamically: `self["Widget_Slot_" .. i]:SetActiveWidgetIndex(nIndex)`.
- Visibility uses `SetVisibility(ESlateVisibility.Visible/Collapsed)`; keep player-facing text in Chinese.

### Logging Conventions

- Lifecycle and entry logs: `print("Class:Method")`, or `print(string.format("Class:Method Key=%d", nKey))` when the parameters matter.
- Failure logs are English identifier plus reason: `print(string.format("Class:Method failed. RankID=%d", nRankID))`; resource load failures end in `LoadFailed`.
- Logs use English identifiers and formatted values; player-visible messages use Chinese text sent through tips UI/RPC, not through logs.

### Section Dividers And File Order

- Large files are divided with `---- 服务器RPC ----`, `---- 客户端RPC ----`, `---- 其他函数 ----`, and paired `---- XXX begin ----` / `---- XXX end ----` markers for feature blocks.
- File order: header and imports, file-local config tables, class member table, lifecycle (`Construct`/`Destruct`, `ReceiveBeginPlay` ...), server RPCs, client RPCs, then helpers.

## Examples

```lua
---@class UGCPlayerController_C:BP_PlayerController_TopDown_C
---@field GiftPackComponent GiftPackComponent_C
--Edit Below--
require("Script.GameConfigs.CommonConfigPackage")
local UGCGameMode = require("Script.Blueprint.UGCGameMode")

local UGCPlayerController = {
    ---- 存档数据 begin ----

    -- 防御塔相关
    tbUnlockedTower = {},               -- [CS]已解锁的防御塔
    tbTowerLevel = {},                  -- [CS]防御塔等级
    nLastExitTime = 0,                  -- [S]上次退出时间

    -- 关卡相关
    tbStageStarRecord = {},             -- [CS]各关卡当前最高评价星数 { StageID -> Star }
}

-- 上报最高通关关卡排行榜成绩
function UGCPlayerController:ReportStageClearRankScore(nStageID)
    print(string.format("UGCPlayerController:ReportStageClearRankScore StageID=%d", nStageID))
    if not UGCGameSystem.IsServer() then
        return
    end
    if playerController == nil or nStageID == nil or nStageID <= 0 then
        return
    end
    -- 主逻辑直写，不再重复包裹 UE.IsValid 等防御判断
end
```

```lua
-- Script/GameConfigs/GlobalConfig.lua：裸 PascalCase 全局配置，每条上方一行中文注释
-- 玩家人数上限
MaxPlayerCount = 4

-- 合作挑战门票免费领取间隔，单位：秒；每天0点重新开始计时
CoopTicketFreeInterval = 2 * 60 * 60

-- 合作挑战门票付费兑换的消耗，按当天第几次兑换依次取值
CoopTicketBuyCostList = { 50, 100, 150 }

-- 通用状态表现配置：资源路径与统一缩放；不包含英雄专属印记
local tbCommonStatusEffectConfig = {
    Burning = {
        szParticlePath = "/Game/Arts_Effect/ParticleSystems/Share/P_fire_burning_01.P_fire_burning_01", -- 燃烧资源
        nHeight = 0, -- 相对目标中心高度
    },
}
```

```lua
-- EventDefine.lua
EventDefine = EventDefine or {}

EventDefine.UpdatePlayerCombatPower = 125       -- 更新玩家战斗力
EventDefine.UpdateGemList = 113                 -- 更新玩家宝石列表

-- 服务端 RPC：逐条校验，失败用中文 Tips 返回
function UGCPlayerController:ServerRPC_UpgradeTower(nTowerID)
    print(string.format("UGCPlayerController:ServerRPC_UpgradeTower Player=%s TowerID=%d", self.PlayerState.PlayerName, nTowerID))

    -- 防御塔是否解锁
    if not self.tbUnlockedTower[nTowerID] then
        UnrealNetwork.CallUnrealRPC_Unreliable(self, self, "ClientRPC_ShowTipsUI", "升级防御塔失败，防御塔尚未解锁")
        return
    end
end

-- UI 按钮回调：首行日志 + 按钮音效
function King:Button_Tab_OnClicked(nTabIndex, bSkipSound)
    print(string.format("King:Button_Tab_OnClicked[%d]", nTabIndex))
    if not bSkipSound then
        UGCUIManager:PlayButtonOpenSound()
    end
end
```
