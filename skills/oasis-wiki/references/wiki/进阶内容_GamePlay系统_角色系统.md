# 进阶内容/GamePlay系统/角色系统

> 官方知识库同步分类，共 7 篇文章

---

## 修改主角属性

> 文档路径: 进阶内容 > GamePlay系统 > 角色系统 > 修改主角属性

> 文档ID: 153 | [官网原文](https://developer.gp.qq.com/wikieditor/#/catalog/153)

> 更新: 2025-06-19 11:33:39

**涉及API/标识符:** `UGCPawnAttrSystem.SetSpeedScale`

#  修改主角属性

在游戏制作过程中，我们可以在 和平精英的角色 的基础上进行游戏开发，当然也可以对角色做相关的调整，下面会介绍如何在编辑器内调整角色的属性。

---

<br>

## 常见角色属性：

在和平中有很多角色相关的属性，常见属性如下:

- 血量 血量上限
- 信号值 信号值上限
- 能量值 能量值上限
- 基础移动速度 疾跑移动速度 游泳移动速度 趴下移动速度 蹲下移动速度 蹲下疾跑速度

此外还有很多其他的属性，例如 氧气值，背包容量等等。

---

<br>

## 修改属性的方式

在编辑器中，我们主要有四种方式修改角色的属性：

- 主角蓝图
- Lua
- 技能系统
- buff系统
  <br>

#### 主角蓝图

可以在PlayerPawn中的 Attr Config中修改角色属性，也可以在 PlayerPawn中的 AttrModifyComp 组件中的 Attr Modify Config中修改角色属性，还有一些属性在对应功能组件上（例如最大游泳速度在STCharacterMovement上）。
![企业微信截图_16867108643901.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_16867108643901.png)
![企业微信截图_16867108736868.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_16867108736868.png)
![企业微信截图_16867108766707.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_16867108766707.png)
常见属性及修改位置：

| 属性 | 组件 | 分类 | 参数 |
| ------ | ------ | ------ | ------ |
| 血量 | - | Attr Config | Health |
| 血量上限 | - | Attr Config | Health Max |
| 信号值 | - | Attr Config | Signal HP |
| 信号值上限 | - | Attr Config | Signal HPMax |
| 能量值 | - | Attr Config | Energy - 当前能量值 |
| 能量值上限 | - | Attr Config | Energy - 最大能量值 |
| 标准速度 | - | Attr Config | Default Speed Value |
| 站立疾跑速度(增量) | AttrModifyComp | Attr Modify Config | Config Attr Modify List - [Attr Modify Item Name]Sprint [Attr Name]Walk Speed |
| 蹲下速度(增量) | AttrModifyComp | Attr Modify Config | Config Attr Modify List - [Attr Modify Item Name]Crouch [Attr Name]Walk Speed |
| 蹲下疾跑速度(增量) | AttrModifyComp | Attr Modify Config | Config Attr Modify List - [Attr Modify Item Name]CrouchSprint [Attr Name]Walk Speed |
| 趴下速度(增量) | AttrModifyComp | Attr Modify Config | Config Attr Modify List - [Attr Modify Item Name]Prone [Attr Name]Walk Speed |
| 趴下疾跑速度(增量) | AttrModifyComp | Attr Modify Config | Config Attr Modify List - [Attr Modify Item Name]Crawl [Attr Name]Walk Speed |
| 游泳标准速度 | STCharacterMovement | Character Movement:Swiming | Max Swim Speed |

<br>

#### Lua

可以使用 PlayerPawn属性系统 UGCPawnAttrSystem 修改相关角色属性，例如：

```
function PlayerPawn:ReceiveBeginPlay()
    self.SuperClass.ReceiveBeginPlay(self);

    if self:HasAuthority() then
        UGCPawnAttrSystem.SetSpeedScale(self,3)
    end
end
```

上述代码会在设置角色的移动速度总系数为3，从而玩家的正常移动快很多。<br>
更多更详细的属性设置API，请查阅相关API文档。
<br>

#### 技能系统

可以使用 技能系统中 人物相关的 事件节点 修改 角色属性，例如：<br>

- 可以使用 修改最大移动速度(UAESkillAction_ChangeMovSpeed) 事件节点修改技能释放对象的最大移动速度
  <br>

#### Buff系统

可以使用 Buff系统中 人物相关的 buff事件 修改 角色属性，例如：

- 可以使用 AttrModify 动态属性修改器 buff事件 修改 角色属性。

---

<br>

## 实战演练

下面我们以 修改蓝图属性 和 Lua 两种方式 为例演示一下如何修改角色属性。
<br>

#### 修改角色蓝图属性

1.根据系统模板工程 Template_demolition 创建工程之后，双击打开 Asset\Blueprint\BurstPlayerPawn：<br>
![企业微信截图_1686711023569.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_1686711023569.png)
2.找到组件 AttrModifyComp，修改其Attr Modify Config分类下的Config Attr Modify List，将其中的Attr Modify Item Name 为 Prone，Attr Name 为 WalkSpeed 的element 中的 Modifer Value 改成 0：<br>
![企业微信截图_16867110385264.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_16867110385264.png)
3.对游戏进行调试，你会发现你的角色现在趴下移动速度 和 标准速度一样快。

#### Lua修改角色属性

1.根据系统模板工程 Template_demolition 创建工程之后，双击打开 Asset\Blueprint\BurstPlayerPawn：<br>
![企业微信截图_16867110535013.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_16867110535013.png)
2.点击 Lua按钮，打开BurstPlayerPawn对应的角色Lua脚本：<br>
![企业微信截图_16867110677216.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_16867110677216.png)
3.在角色脚本中增加如下代码，然后保存关闭脚本：

```
function BurstPlayerPawn:ReceiveBeginPlay()
    self.SuperClass.ReceiveBeginPlay(self);

    if self:HasAuthority() then
        UGCPawnAttrSystem.SetSpeedScale(self,3)
    end
end
```

该代码在开始游戏的时候，在服务器端，设置了移动速度总系数为3。<br>
4.对游戏进行调试，你会发现你的角色现在移动速度比平时快了很多。

---

## 物品掉落与保留

> 文档路径: 进阶内容 > GamePlay系统 > 角色系统 > 物品掉落与保留

> 文档ID: 20148 | [官网原文](https://developer.gp.qq.com/wikieditor/#/catalog/20148)

> 更新: 2025-06-25 18:10:05

**涉及API/标识符:** `DA_GameModeGeneral`, `IsSkipSpawnDeadTombBox`

# 物品掉落与保留

和平精英基于大逃杀玩法的机制设计，当玩家死亡时固定以死亡盒子的形式掉落背包物品，为了提高物品掉落策略的灵活性，在物资编辑器2.0的基础上对物品属性及掉落功能进行了扩展，允许开发者自定义玩家角色的物品掉落与保留规则。

<br>

## 设置死亡掉落类型

角色物品掉落特指玩家角色死亡时触发的掉落行为，因此需要先指定死亡掉落物品的形式。

点击编辑器菜单栏【玩法通用设置】按钮，在 ``DA_GameModeGeneral`` 数据资产蓝图中找到“Pawn”属性类目，该类别下定义了一个名为 ``玩家死亡掉落类型`` 的属性，该属性决定玩家角色死亡时是否允许掉落背包物品及掉落的形式，开发者需设置合适的掉落类型。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/UlDxKimage.png)

**生成死亡盒子**

默认的掉落类型，背包中能够掉落的物品都收集在盒子中，与和平精英的角色物品掉落方式保持一致。

![QQ2025624-212438-HD-ezgif.com-video-to-gif-converter.gif](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/V1cf2QQ2025624-212438-HD-ezgif.com-video-to-gif-converter.gif)

**散落**

能够掉落的物品将以可拾取物的形态分散掉落在角色死亡的位置附近。

![QQ2025624-211856-HD-ezgif.com-video-to-gif-converter.gif](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/4Do6LQQ2025624-211856-HD-ezgif.com-video-to-gif-converter.gif)

**不生成盒子也不散落**

设置为该类型将禁止角色掉落背包物品。

![QQ2025624-214634-HD-ezgif.com-video-to-gif-converter.gif](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/d0OlbQQ2025624-214634-HD-ezgif.com-video-to-gif-converter.gif)

<br>

## 配置物品掉落属性

各类物品模板属性的 [V2背包配置](https://developer.gp.qq.com/wikieditor/#/catalog/20101?autoJump=V2%E8%83%8C%E5%8C%85%E9%85%8D%E7%BD%AE) 类目中都具有 ``死亡物品掉落配置`` 属性，该属性决定了此类型的物品能否掉落或者保留，结合死亡掉落类型的组合配置可以实现不同的掉落与保留效果。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/u1MRvimage.png)

| |留在背包|掉落|直接删除|
|:-:|:-:|:-:|:-:|
|**生成死亡盒子**|保留|掉落|✖|
|**散落**|保留|掉落|✖|
|**不生成盒子也不散落**|保留|✖|✖|

<br>

## 注意事项

- 以上配置方式仅适用于 [物品编辑器](https://developer.gp.qq.com/wikieditor/#/catalog/20101) 创建的物品，且需要启用新的 [背包系统](https://developer.gp.qq.com/wikieditor/#/catalog/20104)；对于使用和平原生物品及经典背包的场景，仅支持控制是否生成死亡盒子，通过覆写 **UGCPlayerPawn** 的 ``IsSkipSpawnDeadTombBox`` 函数实现：
	```lua
	function UGCPlayerPawn:IsSkipSpawnDeadTombBox(EventInstigater)
    		return true
	end
	```
- 枪械的配件及子弹也属于独立的物品，因此需要配合枪械单独配置掉落属性，否则会出现保留或掉落效果不一致的现象

---

## 组队招募

> 文档路径: 进阶内容 > GamePlay系统 > 角色系统 > 组队招募

> 文档ID: 20485 | [官网原文](https://developer.gp.qq.com/wikieditor/#/catalog/20485)

> 新增: 2026-10-20 10:05:16

**涉及API/标识符:** `bEnableRecruit`, `BP_TeamRecruit_PlayerComponent`, `Construct`, `CreateWidgetAsync`, `DeclarationIndex`, `FRecruitInfo`, `GP_TeamRecruit`, `InvitUIPath`, `IsCaptain`, `IsFull`, `IsGlobalScope`, `MainPartyUIPath`, `MemberCount`, `MemberIconURLList`, `Members`, `MemberUIDList`, `OnForwardClicked`, `ReceiveBeginPlay`, `ReceiveEndPlay`, `RecruitID`, `RecruitmentUIPath`, `RuleUIPath`, `ScreeningUIPath`, `SetData`, `TargetIndex`, `TeamDeclaration`, `TeamDeclarations`, `TeamPlayers`, `TeamTarget`, `TeamTargets`, `true`, `UGC_PartyTemplate_Invitation_UIBP`, `UGC_PartyTemplate_Recruitment_UIBP`, `UGC_PartyTemplate_Rule_UIBP`, `UGC_PartyTemplate_TeamScreening_UIBP`, `UGC_PartyTemplate_TeamUp_UIBP`, `UGCGameSystem.GetLocalPlayerController`, `UGCMultiMode.RequestMatch`, `UGCPartyComponent`, `UI.UISlot.MainUISlot_High`

# 组队招募

该模板提供和平精英局内组队大厅能力，包含招募列表、发布招募、申请入队、我的队伍管理、入队邀请与队伍语音联动，可供其他开发者下载导入使用，接入到具体的玩法当中，使用方法见下文。

## 功能详解

### 流程

```lua
 1  PC->>CMPT: ReceiveBeginPlay()
 2  CMPT->>CMPT: 注册 UGCTeamSystem 队伍委托
 3  CMPT->>Backend: GetGamePartPlayerComponent("TeamRecruit")
 4  Backend-->>CMPT: 返回招募组件
 5  CMPT->>Backend: 注册招募委托（查询结果/发布成功/撤回过期）

 6  Note over PC: 玩家打开组队大厅
 7  PC->>CMPT: Open()
 8  CMPT->>UI: CreateWidgetAsync(TeamUp_UIBP) + AddToSlot(MainUISlot_High, 10)
 9  CMPT->>Backend: QueryRecruitList(范围, 目标)
10  Backend-->>CMPT: OnQueryRecruitListResultDelegate(招募列表)
11  CMPT->>CMPT: 转换数据 + 过滤自己的队伍 + 失效暂留
12  CMPT->>UI: RefreshTeamList(teams)
13  CMPT->>UI: SetMyTeam(myTeamData)

14  Note over UI: 玩家点击发布招募
15  UI->>CMPT: PublishRecruit(宣言, 目标, 范围)
16  CMPT->>Backend: TeamRecruit.PublishRecruit()
17  Backend-->>CMPT: OnPublishRecruitSuccessDelegate
18  CMPT->>UI: OnRecruitmentPublished()

19  Note over UI: 玩家申请加入某队伍
20  UI->>CMPT: JoinRecruitedTeam(RecruitID)
21  CMPT->>Backend: TeamRecruit.JoinRecruitedTeam()
22  Backend-->>CMPT: OnTeamIDChangedDelegate
23  CMPT->>CMPT: 轮询等待队伍同步稳定
24  CMPT->>UI: SetMyTeam()
25  CMPT->>CMPT: 单人变有队友 → Close() 关闭大厅
```

### 模版结构

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/x6f5timage.png)

**A. 说明按钮**
展示组队功能的规则说明，已经有默认展示文案
**B. 队伍筛选按钮**
按钮常驻展示当前生效筛选条件，展示格式：【队伍发布范围】-【队伍目标】
点击后弹出筛选弹窗
**C. 刷新按钮**
点击立即触发队伍列表刷新
以匹配触发瞬间的队伍人数为第一优先级排序，排序优先级：三人缺一人队伍 ＞ 二人缺二人队伍 ＞ 单人队伍
当队伍人数相同时，按照招募发布时间进行排序，招募发布时间越接近匹配触发时间的队伍，展示优先级越高
**D. 队伍信息展示**
【发布范围】：展示队伍的招募范围
【队伍目标】：展示队伍的队伍目标
【队伍宣言】：展示队伍的招募宣言
**E. 队伍成员展示**
展示队伍内玩家头像
玩法接入玩家档案模版时，点击玩家头像，弹出玩家档案弹窗
**F. 申请入队按钮**
点击后触发申请入队流程
**G. 发布招募按钮**
点击后弹出发布招募弹窗

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/96fp4image.png)

**B1. 发布范围 勾选项**
提供本局、全局两个互斥勾选项，默认选中【本局】
**B2. 队伍目标 勾选项**
界面最多展示6个可勾选文案选项，选项互斥，默认选中首条内容

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/UDQGBimage.png)

**G1. 发布范围 勾选项**
同B1，提供本局、全局两个互斥勾选项，默认选中【本局】
**G2. 队伍目标 勾选项**
界面最多展示6个可勾选文案选项，选项互斥，默认选中首条内容
**G3. 队伍宣言 勾选项**
界面最多展示6个可勾选文案选项，选项互斥，默认选中首条内容
**G4. 发布招募按钮**
点击后发布招募

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/rMbiximage.png)

**H. 我的队伍-队伍信息展示**
【发布范围】：展示队伍的招募范围
【队伍目标】：展示队伍的队伍目标
【队伍宣言】：展示队伍的招募宣言
**I. 我的队伍-队伍成员展示**
展示队伍内玩家头像
玩法接入玩家档案模版时，点击玩家头像，弹出玩家档案弹窗
**J. 我的队伍-退队按钮**
点击后退出当前小队
**K. 撤回招募按钮**
点击后撤回当前招募

## 功能使用

**1. 下载并导入模板**
打开 UGC 编辑器资源商店，找到组队模板，点击下载导入工程。
![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/N9XrKimage.png)

**2. 添加组件到 PlayerController**
`UGCPartyComponent` 是一个 Actor 组件，**必须挂载在 UGCPlayerController 蓝图上**（组件内部通过 `GetOwner()` 获取 PC 并区分端）。
![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/RFvnAimage.png)
>挂载错误（如挂到 GameState / Pawn）会导致功能完全不可用。

**3. 配置组件的 UI 路径**
在组件的细节面板配置 5 个路径属性，详见下方「组件属性配置」章节。
![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/HBkKZimage.png)

**4. 添加 TeamRecruit 的 GamePart**
在玩法的 GamePart 配置中启用 `GP_TeamRecruit`。
![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/79SCfimage.png)
为确保组件可用，还需在 GamePart 的 Config 蓝图中手动将 `BP_TeamRecruit_PlayerComponent` 挂载到 PlayerController 上，并勾选属性复制（Replicates）。
![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/p83wtimage.png)
招募功能还需要满足以下运行条件：
- `bEnableRecruit` 配置为 `true`。
- `TeamPlayers` 配置为 2、3 或 4。（需关闭阶段匹配）
- `TeamDeclarations` 配置队伍宣言，最多 6 条，每条最多 6 个中文字符。
- `TeamTargets` 配置队伍目标，最多 6 条，每条最多 4 个中文字符。
![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/TKjPXimage.png)

>未启用时招募功能整体不可用，日志会打印 获取失败 (GP_TeamRecruit 可能未配置?)。

**5. 配置招募文案**
在 `GP_TeamRecruit` 配置中填写：
|配置项|说明|
|-|-|
|`bEnableRecruit`|是否启用招募|
|`TeamTargets`|队伍目标文案，最多 6 条|
|`TeamDeclarations`|队伍目标文案，最多 6 条|

文案数量决定 UI 上显示的选项数量，未配置的选项会自动隐藏。序号从 1 开始。

**6. 启动游戏测试**
启动游戏后调用 `UGCPartyComponent:Open()` 即可打开组队大厅。

```lua
local PC = UGCGameSystem.GetLocalPlayerController()
local PartyComp = PC and PC:GetComponentByClass(UGCPartyComponent)
if PartyComp then PartyComp:Open() end
```
>PS：在 UGC PIE 测试环境中可能无法获取完整队伍数据，建议接入具体玩法后多人实测。

**7. 实现「前往」按钮**
模板未实现满员后的「前往」逻辑（`OnForwardClicked` 为 TODO），需按自己玩法补充，详见下方「注意事项」第 5 条。

### 组件属性配置

组件 `UGCPartyComponent` 的细节面板属性，5 个路径全部必填，缺省则对应界面打不开。

|属性|指向的蓝图|缺省后果|
|-|-|-|
|`MainPartyUIPath`|`UGC_PartyTemplate_TeamUp_UIBP`|组队大厅打不开|
|`ScreeningUIPath`|`UGC_PartyTemplate_TeamScreening_UIBP`|筛选弹窗打不开|
|`RecruitmentUIPath`|`UGC_PartyTemplate_Recruitment_UIBP`|招募弹窗打不开|
|`RuleUIPath`|`UGC_PartyTemplate_Rule_UIBP`|规则弹窗打不开|
|`InvitUIPath`|`UGC_PartyTemplate_Invitation_UIBP`|收到邀请无弹窗|

#### UI 挂载方式

所有界面统一挂到和平 UI 挂点槽位 `UI.UISlot.MainUISlot_High`，移除用 `RemoveFromParent()`。

|界面|ZOrder|
|-|-|
|组队大厅主界面|10|
|筛选 招募 规则弹窗|15|
|入队邀请弹窗|20|

### 数据结构说明

#### 我的队伍数据

组件通过 `SetMyTeam(myTeamData)` 下发给主界面。

|字段|类型|说明|
|-|-|-|
|`Members`|table[]|成员列表，队长在前|
|`MemberCount`|number|当前成员数|
|`IsCaptain`|boolean|本地玩家是否队长|
|`IsFull`|boolean|是否已满员|
|`IsGlobalScope`|boolean|招募范围（仅已发布招募时携带）|
|`TargetIndex`|number|队伍目标文案序号（仅已发布时携带）|
|`DeclarationIndex`|number|队伍宣言文案序号（仅已发布时携带）|

`Members` 成员结构：`{ UID, PicUrl, IsCaptain }`

#### 招募队伍数据

组件通过 `RefreshTeamList(teamDataList)` 下发给主界面。

|字段|类型|说明|
|-|-|-|
|`RecruitID`|string|招募 ID，申请入队时使用|
|`IsGlobalScope`|boolean|招募范围|
|`TargetIndex`|number|队伍目标序号|
|`DeclarationIndex`|number|队伍宣言序号|
|`Members`|table[]|成员列表，首位为队长|
|`MemberCount`|number|成员数|

#### 招募信息（内核原始结构）

GamePart 返回的 `FRecruitInfo`，组件内部会转换为上表的招募队伍数据。

|字段|类型|说明|
|-|-|-|
|`RecruitID`|string|招募 ID|
|`MemberUIDList`|number[]|成员 UID 列表|
|`MemberIconURLList`|string[]|成员头像 URL 列表|
|`TeamTarget`|string|队伍目标文案|
|`TeamDeclaration`|string|队伍宣言文案|
|`IsGlobalScope`|boolean|招募范围|

### 组件接口

#### 界面控制

|函数|说明|
|-|-|
|`Open()`|打开组队大厅，复用已有实例并触发 5 秒刷新冷却|
|`Close()`|关闭大厅（隐藏不销毁），并关闭筛选/招募弹窗|
|`OpenFilter()`|打开筛选弹窗（与大厅共用刷新冷却）|
|`CloseFilter()`|关闭筛选弹窗并销毁|
|`OpenRecruitment()`|打开发布招募弹窗（复用单例，保留冷却状态）|
|`CloseRecruitment()`|关闭招募弹窗（隐藏不销毁，保留冷却）|
|`OpenRule()`|打开规则说明弹窗|
|`OpenPlayerProfile(UID)`|打开玩家档案（需接入 PlayerProfileManager GamePart，点击自己会被拦截）|

#### 招募与队伍操作

|函数|返回值|说明|
|-|-|-|
|`PublishRecruit(DeclarationIndex, TargetIndex, IsGlobalScope)`|-|发布招募，序号 1~6，30 秒冷却|
|`RevokeRecruitment()`|-|撤回招募|
|`JoinRecruitedTeam(RecruitID)`|-|申请加入队伍，**5 秒冷却**|
|`RequestRefreshTeamList(filter)`|-|查询招募列表，**5 秒冷却**。`filter={IsGlobalScope, TargetIndex}`，传 nil 沿用上次条件|
|`IsRecruiting()`|boolean, goalIdx, declIdx, isGlobal|查询是否处于招募状态|
|`ExitTeam()`|-|退出队伍，会先复位招募按钮再轮询确认|
|`GetMaxTeamSize()`|number|获取玩法队伍人数上限，获取失败回退 4|

#### 冷却查询

|函数|返回值|说明|
|-|-|-|
|`CanPublishRecruit()`|number|发布冷却剩余秒数，0 表示可发布|
|`CanRefreshList()`|number|刷新冷却剩余秒数，0 表示可刷新|
|`CanJoinTeam()`|number|申请冷却剩余秒数，0 表示可申请|
|`GetRefreshCooldownRemain()`|number|同 `CanRefreshList()`|
|`GetPublishCooldownRemain()`|number|同 `CanPublishRecruit()`|
|`IsRefreshCooling()`|boolean|是否刷新冷却中|
|`IsApplyCooling()`|boolean|是否申请冷却中|

>冷却时长由服务端控制（发布 30 秒、刷新 5 秒、申请 5 秒），Lua 层只能查询，**无法修改**。组件采用每秒轮询内核的方式同步倒计时。

#### 文案读取

|函数|返回值|说明|
|-|-|-|
|`GetGoalText(idx)`|string|读取队伍目标文案（从 GP 配置，带缓存）|
|`GetDeclText(idx)`|string|读取队伍宣言文案（从 GP 配置，带缓存）|
|`GetScopeText(isGlobalScope)`|string|返回「全局」/「本局」|

#### UI 回调接口

UI回调接口整合

|函数|调用时机|
|-|-|
|`SetMyTeam(myTeamData)`|我的队伍数据刷新|
|`RefreshTeamList(teamDataList)`|招募列表查询完成|
|`SetFilterLabel(isGlobalScope, goalIdx)`|筛选条件变化|
|`SetRefreshCooldown(remain)`|刷新按钮冷却变化|
|`OnMemberJoined(memberData)`|新成员加入|
|`OnMemberAvatarUpdated(UID, PicUrl)`|头像兜底补拉成功|
|`OnRecruitmentPublished(scopeIdx, goalIdx, declIdx)`|发布招募成功|
|`OnRecruitmentRevoked()`|招募被撤回或过期|
|`ResetRecruitState()`|重置招募按钮状态|

筛选弹窗需实现 `SetPartyComponent(component)`、`SetConfirmCooldown(remain)`；招募弹窗需实现 `SetPartyComponent(component)`、`OnPublishCooldownTick(remain)`。

### 界面功能说明

#### 组队大厅主界面

顶部为「我的队伍」区域（永远显示，每人初始都有局内队伍，含最多 4 个成员头像）；下方为可加入的招募队伍列表，最多展示 5 条，按成员数降序排列，无队伍时显示空状态提示。

底部按钮区为状态机，**仅队长可见**，队员整体隐藏：
|条件|显示按钮|
|-|-|
|队伍已满员|前往|
|已发布招募|撤回招募|
|其他|发布招募|

另有刷新（带冷却倒计时）、筛选、规则说明、退出队伍按钮。

#### 筛选弹窗

可筛选「发布范围」（本局/全局）与「队伍目标」，两组各自互斥且必选一项。支持筛选记忆，重开弹窗自动恢复上次勾选。确定按钮与大厅刷新共用同一冷却。

#### 招募弹窗

可勾选「发布范围」「队伍目标」「队伍宣言」三组，各自互斥必选，默认勾选每组首项。发布按钮带 30 秒冷却倒计时，冷却中置灰。

#### 邀请弹窗

收到入队邀请时自动弹出，展示邀请者头像与昵称，可同意或拒绝。


## 注意事项

**1. 阶段匹配与队伍人数（重要限制）**
组队模板目前仅适用于「未开启阶段匹配」的玩法，请不要开启阶段匹配
![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/nXKTkimage.png)

|玩法配置|表现|
|-|-|
|未开启阶段匹配|人数上限正确，招募与满员判定正常|
|开启阶段匹配，各阶段人数**不同**|满员判定错误。例如实际匹配进 2 人阶段，模板仍按 4 人判定未满员，导致队伍已满却仍显示「发布招募」；反之则会提前误判满员而显示「前往」，招募流程走不下去|

建议：
- 若玩法不需要阶段匹配，保持关闭即可，模板开箱可用。
- 若必须开启阶段匹配且各阶段人数不同，需要自行扩展模板：在匹配成功后依据实际开局人数动态确定上限，而不是依赖 `GetTeamPlayersNumber()`。

**2. 局内邀请额外前置**

- 邀请同 DS 玩家：需开启聊天 + 开启本局频道或私聊频道
- 邀请大厅好友：需开启聊天 + 开启好友频道

**3. 组件为纯客户端组件**
服务端实例在 `ReceiveBeginPlay` 中直接返回，不做任何注册。所有 UI 与招募方法仅在客户端有效，请勿在服务端调用。

**4.「前往」按钮需自行实现**
模板的 `OnForwardClicked` 为 TODO 空实现，需按玩法自行补充（发起匹配、传送、开始玩法等）。
>若使用 `UGCMultiMode.RequestMatch`，注意第一个参数是 **SubModeID** 而非大厅 ModeID；服务器会校验发起者身份，非队长或队伍状态不符会返回 `not-leader`，客户端提示「你当前不是队长，不能点击匹配」。

**5. 邀请弹窗的数据时序**
`CreateWidgetAsync` 异步创建，`SetData` 可能先于 `Construct` 执行导致昵称被蓝图默认值覆盖。模板通过 `Construct` 后重新应用数据修复。重构该界面时**必须保留**此机制。

## 常见问题解答

**Q：招募列表一直为空？**
依次排查：① 日志是否出现 `GP_TeamRecruit` 可能未配置，是则检查 GamePart；② 是否配置了 `TeamTargets`/`TeamDeclarations` 文案；③ 是否真的没人发布招募（建议双客户端实测）；④ 是否被「自己的队伍」过滤（日志会打印 **过滤自己的队伍**）。

**Q：点击发布招募没反应？**
检查是否处于 30 秒发布冷却（`CanPublishRecruit() > 0`），以及是否配置了宣言/目标文案。

**Q：队伍只有我一个人却显示满员？**
见下方「注意事项」第 2 条，多为 `GetTeamPlayersNumber()` 返回值与配置不符，搜日志 `[ModeCheck]` 确认。

**Q：界面打不开？**
检查组件的 5 个 UI 路径属性是否全部配置，以及组件是否挂在 PlayerController 上。日志会打印对应路径为空的提示。

**Q：队员看不到队长发布的招募？**
队长视角用本地发布缓存，队员视角用内核 `IsRecruiting()` 查询。若队员看不到，说明内核侧招募状态尚未同步。

**Q：头像显示不出来？**
跨 DS 加入的成员需等待 PlayerState 生成，模板有快速重试与持续兜底补拉。日志关键字：**头像兜底补拉成功**。

**Q：切 DS 后功能异常？**
模板有自愈机制，检查日志是否出现 **获取成功**、**已加载**，**补注册**。若自定义扩展过组件，检查是否遗漏 `ReceiveEndPlay` 清理。

**Q：我的队伍成员头像只有 4 个槽位，能改吗？**
主界面提供 4 个头像槽位。若玩法队伍人数上限超过 4，需自行扩展 UI 槽位与头像刷新逻辑。

## 日志关键字速查

组件日志 TAG 为 `[UGCPartyComponent]`，界面为 `[TeamUp_UIBP]` 等。

|关键字|含义|
|-|-|
|`[ModeCheck] ModeID=x, GetTeamPlayersNumber=y`|模式 ID 与队伍人数，调试人数上限用|
|`获取失败 (GP_TeamRecruit 可能未配置?)`|GamePart 未启用|
|`GetGamePartConfig('TeamRecruit').TeamTargets 不可用`|招募文案未配置|
|`!!! 招募组件不可用，xxx失败`|招募组件获取失败|
|`过滤自己的队伍 RecruitID=xxx`|自己的队伍被过滤（正常）|
|`筛选条件变化，丢弃旧缓存`|切换筛选，缓存清空（正常）|
|`头像兜底补拉成功 UID=xxx`|跨 DS 头像补拉成功|
|`OnForwardClicked (TODO)`|前往按钮未实现，需自行补充|
|`not-leader`|服务器拒绝匹配请求（非队长或队伍状态不符）|

---

## 角色AI托管

> 文档路径: 进阶内容 > GamePlay系统 > 角色系统 > 角色AI托管

> 文档ID: 20466 | [官网原文](https://developer.gp.qq.com/wikieditor/#/catalog/20466)

> 新增: 2026-09-15 14:31:59

**涉及API/标识符:** `PlayerController`, `UE.IsValid`, `UGCGame.UGCPlayerAIHostComponent`, `UGCPlayerAIHostComponent`, `UGCPlayerPawn`

# 角色AI托管

## 概述

游戏中常见的自动战斗、自动寻路相关功能，会使得玩家操作的角色临时处于托管状态，并按一定预设好的行为逻辑自动行动，在这个过程中，玩家如果进行了任何操作，就又会恢复手动操作。在绿洲编辑器下，欲实现这样一套功能，可以使用新提供的 `UGCPlayerAIHostComponent` 来解决。角色AI托管依赖导航网格系统作为前提，具体配置方式请参考[导航网格](https://developer.gp.qq.com/wikieditor/#/catalog/20266)。

## 添加UGCPlayerAIHostComponent组件

在 `PlayerController` 蓝图中添加 `UGCPlayerAIHostComponent` 组件

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/WcoA6image.png)


给PlayerController挂载了该组件后，且调用接口激活了该组件的托管模式后，角色就会使用配置的行为树里的行为逻辑，进行托管操作了。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/LqKroimage.png)

|参数|说明|
|-|-|
|AIHostBTAsset|AI托管组件默认运行的行为树。托管时，会启用该行为树，则玩家角色会以该行为树的配置行为逻辑自动执行相应行为。也可以默认不配置，运行时动态传进去——用于控制不同需求下可能需要用不同的行为树的情况（比如自动战斗、自动寻路应该是俩不同的行为树）|
|没有操作自动托管|如果玩家在自由操作模式下，如果经过一段时间没有操作，是否自动进入托管状态|
|任一操作退出托管|玩家在托管状态下，如果进行了任一一个操作，会直接退出托管，恢复自由操作|
|进入托管挂机时间|自由操作模式下，没有操作自动进入托管的时间|
|踢出对局托管时间|玩家进入托管了多久后，会自动踢出对局|

## 制作玩家托管使用的行为树

制作玩家托管行为树的方案和制作怪物行为树的原理类似。但需要注意的是，并非所有节点都能在玩家托管行为树中使用，对于此种行为树，可参考现成提供好的一个自动战斗行为树资产。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/NlTPeimage.png)
![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/HlTz5image.png)

将刚才制作好的行为树添加到组件上

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/Ka1a7image.png)

## 通过技能开/关托管功能

在**技能编辑器**中的 自动战斗 模板下创建托管技能模板，并将创建好的[技能添加](https://developer.gp.qq.com/wikieditor/#/catalog/20091?autoJump=%E6%B7%BB%E5%8A%A0%E6%8A%80%E8%83%BD)到 `UGCPlayerPawn` 蓝图上。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/2prA0image.png)
![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/JaTXgimage.png)
![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/J0QsXimage.png)

完成技能配置后启动调试效果如下：

![2026-08-1917-45-16-ezgif.com-video-to-gif-converter.gif](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/WPD1r2026-08-1917-45-16-ezgif.com-video-to-gif-converter.gif)
## 手动调用API开/关托管功能

### 获取AI托管组件

``` Lua
    local Controller = OwnerActor:GetControllerSafety()
    if not UE.IsValid(Controller) then
        return nil
    end

    local AIHostCompClass = LoadClass("/Script/UGCGame.UGCPlayerAIHostComponent")
    if not AIHostCompClass then
        print('AIHostCompClass is Not Valid.............')
        return nil
    end

    local AIHostComp = Controller:GetComponentByClass(AIHostCompClass)
    if not UE.IsValid(AIHostComp) then
        print('AIHostComp is Not Valid.............')
        return nil
    end

return AIHostComp
```

### 开启托管并透传行为树

``` Lua
	--服务端调用
 local HostComp = self:GetAIHostComponent(self:GetOwnerActor())
 if UE.IsValid(HostComp) then
     --HostComp:ServerRequestStartAIHosting()
     HostComp:StopAIHosting()
     HostComp.AIHostBTAsset = self.AutoCombatBTAsset
     --HostComp.bAutoStartAIHost = true
     HostComp:StartAIHosting()
end
```
### 退出托管

``` Lua
--服务端调用
local HostComp = self:GetAIHostComponent(self:GetOwnerActor())
if UE.IsValid(HostComp) then
    print('test432 StopAIHosting')
    HostComp:StopAIHosting()
    --HostComp.bAutoStartAIHost = false
end
```

#### 没有操作自动托管

如果需要使用该功能，不建议直接在组件上开启该选项，因为一旦开启，由于组件是持续生效的，则会导致玩家在进入游戏后，只要没有操作一段时间就进入托管状态。（当然如果就是需要这种效果也可以这么使用。）通常还是建议，组件上默认关闭，开启了托管模式后，再打开该开关，关闭了托管模式，同时也将该开关关掉。

``` Lua
local HostComp = self:GetAIHostComponent(self:GetOwnerActor())
if UE.IsValid(HostComp) then
    HostComp.bAutoStartAIHost = true
end
```

---

## 角色Avatar复制

> 文档路径: 进阶内容 > GamePlay系统 > 角色系统 > 角色Avatar复制

> 文档ID: 20228 | [官网原文](https://developer.gp.qq.com/wikieditor/#/catalog/20228)

> 新增: 2026-10-16 10:26:02

**涉及API/标识符:** `ClientShowAvatar`, `PlayAnim`, `ServerShowAvatar`, `UE.LoadObject`, `UGCCharAvatarShowcaseActor`, `UGCGameSystem.GetLocalPlayerController`, `UGCGameSystem.GetUIDByPlayerController`, `UGCObjectUtility.FindObject`, `UGCPlayerController.lua`, `UnrealNetwork.CallUnrealRPC`

# 角色Avatar复制

编辑器已提供专用于玩家角色Avatar的展示Actor，该Actor属于静态物件，开发者可通过调用特定的API将玩家角色的Avatar复制到此物件中，以实现展示角色Avatar的效果。

![ScreenShot_2025-10-21_103538_182.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/TPMxnScreenShot_2025-10-21_103538_182.png)

<br>

## 添加Avatar展示Actor

在【模式】中搜索“UGCClientAvatarShowcaseActor_Base_BP”，将该Actor放置在场景中。

![Avater-加入avatar.drawio.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/2IT0cAvater-%E5%8A%A0%E5%85%A5avatar.drawio.png)

<br>

## 复制Avatar

Avatar展示Actor可用于展示角色Avatar，通过API调用实现，目前提供两种方法，适用于不同场景需求：

|方法|生效范围|适用场景|
|:-:|:-:|-|
|[ClientShowAvatar](https://developer.gp.qq.com/api/#/searchContent/UGCCharAvatarShowcaseActor?classDetailShow=true&path=class%2Fdetail%2FOthers%2FUGCCharAvatarShowcaseActor.json&isSelect=1&apiEnc=%5B%22%E7%B1%BB%22%5D&apiLabel=UGCCharAvatarShowcaseActor&autoJump=ClientShowAvatar)|客户端|适用于小场景展示，确保需展示的角色在当前客户端视野范围内且未被剔除|
|[ServerShowAvatar](https://developer.gp.qq.com/api/#/searchContent/UGCCharAvatarShowcaseActor?classDetailShow=true&path=class%2Fdetail%2FOthers%2FUGCCharAvatarShowcaseActor.json&isSelect=1&apiEnc=%5B%22%E7%B1%BB%22%5D&apiLabel=UGCCharAvatarShowcaseActor&autoJump=ServerShowAvatar)|服务器|适用于大场景展示，可确保所有客户端都能看到Avatar展示，需展示的角色不受视野限制|

以下为使用示例，界面设计如下：

![Avater-第 9 页.drawio.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/p42SMAvater-%E7%AC%AC%209%20%E9%A1%B5.drawio.png)

为按钮添加对应功能逻辑，点击“ClientShowAvatar”按键的执行代码如下:

``` lua
--Main.lua部分代码

function MainUI:ButtonClientShowAvatarOnClicked()
	ugcprint("客户端执行展示角色");
	--根据ID名称获取Actor
  local AvatarDisplayActor = UGCObjectUtility.FindObject("AvatarDisplayActor");
	--获取客户端当前的 PlayerController
  local PlayerController = UGCGameSystem.GetLocalPlayerController();
	--获取PlayerController对应的Uid
	local Uid = UGCGameSystem.GetUIDByPlayerController(PlayerController);
	--根据Uid展示对应玩家形象（客户端执行）
  AvatarDisplayActor:ClientShowAvatar(Uid);
	return nil;
end

```

点击“ServerShowAvatar”按键的执行代码如下:

``` lua
--Main.lua部分代码

function MainUI:ButtonServerShowAvatarOnClicked()
	ugcprint("服务器执行展示角色");
	--获取客户端当前的 PlayerController
  local PlayerController = UGCGameSystem.GetLocalPlayerController();
	--调用PlayerController的ServerRPC函数
  UnrealNetwork.CallUnrealRPC(PlayerController, PlayerController, "ShowAvatarByServer");
	return nil;
end

--UGCPlayerController.lua部分代码

--接收RPC调用
function UGCPlayerController:GetAvailableServerRPCs()
	return
  "ShowAvatarByServer"
end

function UGCPlayerController:ShowAvatarByServer()
  --根据ID名称获取Actor
  local AvatarDisplayActor = UGCObjectUtility.FindObject("AvatarDisplayActor");
  --获取PlayerController对应的Uid
	local uid = UGCGameSystem.GetUIDByPlayerController(self);
  --根据Uid展示对应玩家形象（服务器执行）
  AvatarDisplayActor:ServerShowAvatar(uid);
end

```

调试效果如下，点击“ClientShowAvatar”按键，Avatar展示Actor显示当前玩家的角色Avatar。

![avt显示客户端.gif](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/WxwA6avt%E6%98%BE%E7%A4%BA%E5%AE%A2%E6%88%B7%E7%AB%AF.gif)

如果在需展示的角色被剔除时再点击“ClientShowAvatar”按键，Avatar展示Actor不会显示Avatar。

![ScreenShot_2025-10-20_150428_271.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/5KKMfScreenShot_2025-10-20_150428_271.png)

点击“ServerShowAvatar”按键，Avatar展示Actor显示当前玩家的角色Avatar，无论角色是否被剔除。

![avt显示服务端.gif](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/8HZ4Lavt%E6%98%BE%E7%A4%BA%E6%9C%8D%E5%8A%A1%E7%AB%AF.gif)
![ScreenShot_2025-10-20_150445_218.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/UYpFNScreenShot_2025-10-20_150445_218.png)

<br>

## 播放角色动画

[PlayAnim](https://developer.gp.qq.com/api/#/searchContent/UGCCharAvatarShowcaseActor?classDetailShow=true&path=class%2Fdetail%2FOthers%2FUGCCharAvatarShowcaseActor.json&isSelect=1&apiEnc=%5B%22%E7%B1%BB%22%5D&apiLabel=UGCCharAvatarShowcaseActor&autoJump=PlayAnim) 方法可控制Avatar展示Actor循环播放或停止指定动画。以下为使用示例，界面设计如下：

![Avater-第 7 页 的副本.drawio.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/RvPHJAvater-%E7%AC%AC%207%20%E9%A1%B5%20%E7%9A%84%E5%89%AF%E6%9C%AC.drawio.png)

为按钮添加对应功能逻辑，部分代码如下：

``` lua
--Main.lua部分代码

function MainUI:ButtonPlayAnimOnClicked()
	ugcprint("播放动画");
	--根据路径加载动画资源（该路径对应动画序列“准备待机动作1”）
  local Animation = UE.LoadObject('/Game/Arts_Player/Characters/Animation/Shared_Anim/Loading/004/DianFengSai_P1_Idle.DianFengSai_P1_Idle');
	--根据ID名称获取Actor
	local AvatarDisplayActor = UGCObjectUtility.FindObject("AvatarDisplayActor");
	--循环播放动画
	AvatarDisplayActor:PlayAnim(Animation, true);
	return nil;
end

function MainUI:ButtonStopAnimOnClicked()
	ugcprint("停止动画");
	--根据路径加载动画资源（该路径对应动画序列“准备待机动作1”）
  local Animation = UE.LoadObject('/Game/Arts_Player/Characters/Animation/Shared_Anim/Loading/004/DianFengSai_P1_Idle.DianFengSai_P1_Idle');
	--根据ID名称获取Actor
	local AvatarDisplayActor = UGCObjectUtility.FindObject("AvatarDisplayActor");
	--停止播放动画
	AvatarDisplayActor:PlayAnim(Animation, false);
	return nil;
end

```

Avatar展示Actor显示Avatar后，点击“PlayAnim”按键，该Actor会循环播放指定动画。

![_20260928_153720-ezgif.com-video-to-gif-converter.gif](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/6dWAT_20260928_153720-ezgif.com-video-to-gif-converter.gif)

点击“StopAnim”按键，该Actor播放一次指定动画后停止。

![停止播放2.gif](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/7gpGn%E5%81%9C%E6%AD%A2%E6%92%AD%E6%94%BE2.gif)

---

## 角色出生点

> 文档路径: 进阶内容 > GamePlay系统 > 角色系统 > 角色出生点

> 文档ID: 362 | [官网原文](https://developer.gp.qq.com/wikieditor/#/catalog/362)

> 更新: 2025-06-19 11:33:39

**涉及API/标识符:** `BP_PlayerStartManager`, `BP_STPlayerStart`, `ComponentManager`, `DebugPlayerSettings`, `GetUGCModePlayerStart`, `PlayerBornPointID`, `PlayerStartManager`, `UGCGameMode`, `UGCGameSystem.GameState`

#  角色出生点

在绿洲启元编辑器中，开发者可以放置角色出生点来划定玩家们的出生位置，也可以通过配置不同的ID属性，以决定不同队伍在地图上的出生位置。

<br>

## 创建出生点

出生点是一种名为 ``BP_STPlayerStart`` 的蓝图，该蓝图有三种获取方式：模式面板、资源目录及蓝图创建。

**模式面板**

点击编辑器的菜单栏 ``窗口 -> 模式``，可以开启模式面板：

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/yDq1kimage.png)

在模式面板中搜索“PlayerStart”即可找到该蓝图：

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/H1EB8image.png)

> 另一个“PlayerStart”为UE原生的出生点蓝图，不建议使用

**资源目录**

在资源目录下也可以通过“PlayerStart”查找到该蓝图：

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/obdLeimage.png)

**蓝图创建**

在工程任意目录下右键点击 ``Blueprint Class``，搜索“PlayerStart”找到该蓝图，选中后即可创建出生点蓝图：

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/H3ztqimage.png)

以上任意方式获得出生点蓝图后，将该蓝图放置在关卡中合适的位置进行实例化，PlayerPawn会根据该出生点结合指定规则进行出生。

![企业微信截图_1686711110328.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_1686711110328.png)

<br>

## 配置出生规则

玩家角色的出生规则一般由开发者定义，如果关卡场景中存在多个出生点蓝图实例，且未实现出生逻辑，则默认会从多个出生点内随机一个点位进行出生；当玩法中存在多个阵营时，开发者需要实现出生规则以避免不同阵营的玩家出生在同一点位。

**1. 设置PlayerBornPointID**

出生点蓝图的 ``PlayerBornPointID`` 属性标识该出生点用于带有指定ID的玩家出生，常见的做法是将该属性值设置为队伍ID。

![企业微信截图_168671112293.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_168671112293.png)

**2. 创建及配置PlayerStartManager**

``PlayerStartManager`` 负责管理出生规则，具体的逻辑实现都写在该类文件中。

右键点击 ``Blueprint Class``，搜索“PlayerStartManager”，找到名为 ``BP_PlayerStartManager`` 的组件，选择创建并命名为“MyStartManager”。

![企业微信截图_16867111639124.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_16867111639124.png)

打开工程Blueprint目录下的 ``UGCGameMode`` 蓝图，在【细节面板】中找到``ComponentManager``属性，展开 ``GMComponentManager/ComponentConfigs`` 子属性，添加一个配置项，并设置以下参数：
- SubSystemId：ESSPlayerStartManager
- SubSystemClass：MyStartManager

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/MfDiVimage.png)

**3. 实现自定义出生规则**

双击打开MyStartManager蓝图，点击组件菜单栏的“Lua”按钮，打开MyStartManage类脚本文件：

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/2LoOyimage.png)

``GetUGCModePlayerStart`` 函数可以覆写出生规则，以下为示例：

```lua
--角色出生点管理器
local MyStartManager = {};

function MyStartManager:GetUGCModePlayerStart(Controller)

    --标记当前属于服务器还是客户端
    if UGCGameSystem.GameState:HasAuthority() == true then
        print("MMG_Lua MyStartManager:GetUGCModePlayerStart Server");
    else
        print("MMG_Lua MyStartManager:GetUGCModePlayerStart Client");
    end

    --获取当前控制器的玩家状态（PlayerState）
    local PlayerState = Controller.PlayerState;

    --PlayerState为空时，打印警告
    if PlayerState == nil then
        print( "Error: MyStartManager:GetUGCModePlayerStart PlayerState is nil!");
    end

    --根据玩家团队编号获取玩家出生点（PlayerStart）
    local SelectedPlayerStart = self:FindPlayerStartByBornPointID(PlayerState.TeamID, true);--当一个出生点已经被使用过一次，用于出生玩家时，自动寻找其他未被使用的出生点

    --打印当前玩家的队伍编号（TeamID）
    print(string.format("MMG_Lua MyStartManager:GetUGCModePlayerStart PlayerState.TeamID[%s]", PlayerState.TeamID));

    --若找到玩家出生点（PlayerStart），打印当前玩家出生点名称和对应BornID，否则输出错误信息
    if SelectedPlayerStart ~= nil then
        print(string.format("MyStartManager:GetUGCModePlayerStart SelectedPlayerStart[%s] BornID[%d] PlayerID[%s]",
        KismetSystemLibrary.GetObjectName(SelectedPlayerStart), SelectedPlayerStart.PlayerBornPointID, Controller.PlayerKey));
        --设置当前PlayerStart被占用
        SelectedPlayerStart:SetMarkOccupied();
        print(string.format( "PlayerStartOccupied?[%s]",SelectedPlayerStart:IsMarkOccupied()));
        return SelectedPlayerStart;
    else
        print("Error: MyStartManager:GetUGCModePlayerStart SelectedPlayerStart is nil!");
    end

    return nil;
end

return MyStartManager;
```

> 玩家复活时，也是通过GetUGCModePlayerStart函数决定复活点，因此需要在该函数内对初始出生和复活按需区分处理逻辑


<br>

## 运行调试

在 ``DebugPlayerSettings`` 面板中设置好队伍数量及每队的玩家数量后，点击“调试”按钮即可查看最终出生效果。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/gbLtVimage.png)

![企业微信截图_1686711975945.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_1686711975945.png)

---

## 队伍与阵营

> 文档路径: 进阶内容 > GamePlay系统 > 角色系统 > 队伍与阵营

> 文档ID: 20095 | [官网原文](https://developer.gp.qq.com/wikieditor/#/catalog/20095)

> 更新: 2026-03-13 10:25:36

**涉及API/标识符:** `Camp`, `ChangePlayerTeamID`, `DA_GameModeGeneral`, `PlayerBornPointID`, `SetCampForActor`, `SetCampForTeam`, `UGCCampSystem`, `UGCCampSystem.SetCampForActor`, `UGCCampSystem.SetCampForTeam`, `UGCGameSystem.GetAllPlayerPawn`, `UGCGameSystem.GetUGCResourcesFullPath`, `UGCGenericCharacterSystem.SpawnGenericCharacter`, `UGCObjectUtility.LoadClass`, `UGCPawnAttrSystem.GetPlayerKeyInt64`, `UGCPawnAttrSystem.GetTeamID`, `UGCPlayerPawn`, `UGCTeamSystem`, `UGCTeamSystem.ChangePlayerTeamID`

# 队伍与阵营

## 队伍系统

和平精英内置了一套完整的队伍系统，玩法可以按需求设置匹配时的队伍构成，也允许对处于局内的玩家重新分队，同队伍的玩家头顶会显示带有队内编号的UI，大 [小地图](https://developer.gp.qq.com/wikieditor/#/catalog/152) 上也会显示同队玩家的实时位置，如果 [启用了组队面板](https://developer.gp.qq.com/wikieditor/#/catalog/239) 还会显示同队玩家的状态。

![企业微信截图_17441867608036.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/l1lRW%E4%BC%81%E4%B8%9A%E5%BE%AE%E4%BF%A1%E6%88%AA%E5%9B%BE_17441867608036.png)

### 设置队伍

玩法可以设置匹配时的队伍数及每队人数，匹配系统将优先参考设置将满足人数条件的玩家拉入对局，具体设置可参考 [队伍设置](https://developer.gp.qq.com/wikieditor/#/catalog/127?autoJump=1.1+%E9%98%9F%E4%BC%8D%E8%AE%BE%E7%BD%AE:~:text=1.1-,%E9%98%9F%E4%BC%8D%E8%AE%BE%E7%BD%AE,-%E3%80%90%E9%98%9F%E4%BC%8D%E6%95%B0%E9%87%8F%E3%80%91%EF%BC%9A%E6%8C%87) 部分内容。
> 和平大厅组队人数最多4人，如果匹配设置单队人数过多，会导致拉入对局的玩家队伍组成存在不稳定的情况

---

### 动态分队

所有进入对局的玩家都会拥有各自的队伍编号，即使玩家以单人进入对局，系统也会为其分配一个默认的队伍编号；如果涉及二次分队，例如非对称类型的玩法，则需要调用 [``ChangePlayerTeamID``](https://developer.gp.qq.com/api/#/searchContent/UGCTeamSystem?classDetailShow=true&path=class%2Fdetail%2F%E5%92%8C%E5%B9%B3%E5%85%A8%E5%B1%80%E6%8E%A5%E5%8F%A3%2F%E7%A4%BE%E4%BA%A4%E7%B3%BB%E7%BB%9F%2FUGCTeamSystem.json&isSelect=1&apiEnc=%5B%22%E7%B1%BB%22%5D&apiLabel=UGCTeamSystem&autoJump=ChangePlayerTeamID) 进行动态调整。

代码示例：

``` lua
local Pawns = UGCGameSystem.GetAllPlayerPawn()

if Pawns[1] then
	local PlayerKey = UGCPawnAttrSystem.GetPlayerKeyInt64(Pawns[1])
	UGCTeamSystem.ChangePlayerTeamID(PlayerKey, NewTeamID)
end
```

---

### 大厅组队与玩法队伍

和平大厅的组队与玩法队伍系统是两套独立的队伍关系，队伍系统中的分队是临时性的队伍关系，当该对局结束时即结束队伍关系；和平大厅的队伍关系仅作为匹配成局时的分队参考，不影响玩法内的重新分队，且分队后也不影响和平大厅的队伍组成关系，即从玩法返回大厅时队伍关系仍然保留。

> [二次匹配](https://developer.gp.qq.com/wikieditor/#/catalog/376) 功能扩展了大厅队伍关系的能力，支持加入大厅队伍和同进同出子模式

![613b8712bfa4d139ca688a2280aa54ba.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/7yjVw613b8712bfa4d139ca688a2280aa54ba.png)

<br>

## 阵营系统

队伍系统虽然满足了组队需求，但是分配机制和角色定位较为固定，不同队伍之间固定为敌对关系，缺乏竞技对抗的深度，为此额外构建了阵营系统。

新的阵营系统提供了更灵活的敌对关系配置，同时配合新的 [怪物系统](https://developer.gp.qq.com/wikieditor/#/catalog/20144) 同步支持怪物的阵营分配，开发者仅需通过蓝图参数设置即可实现阵营的分配，也支持通过 [UGCCampSystem](https://developer.gp.qq.com/api/#/searchContent/UGCCampSystem?classDetailShow=true&path=class%2Fdetail%2F%E5%92%8C%E5%B9%B3%E5%85%A8%E5%B1%80%E6%8E%A5%E5%8F%A3%2F%E7%A4%BE%E4%BA%A4%E7%B3%BB%E7%BB%9F%2FUGCCampSystem.json&isSelect=1&apiEnc=%5B%22%E7%B1%BB%22%5D&apiLabel=UGCCampSystem) 提供的接口动态调整阵营。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/J8Lx3image.png)

---

### 阵营关系配置

工程已有一套默认的阵营配置表，该表入口位于 ``玩法通用设置``，在编辑器菜单栏点击【玩法通用设置】按钮,或在 ``Asset/Data`` 目录下找到通用配置数据资产蓝图 ``DA_GameModeGeneral``。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/UkHGaimage.png)
![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/Nt4mCimage.png)

> 如果数据资产蓝图被误删除，可通过点击【玩法通用设置】按钮重新生成该蓝图

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/AOJ4vimage.png)

双击打开 ``DA_GameModeGeneral``，在 ``Camp`` 属性项中添加需要配置的阵营信息。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/pyXnximage.png)

各阵营之间必须存在明确的对立/共存关系，例如：当阵营1与阵营2设置为中立关系，阵营2与阵营3设置为敌对关系时，阵营1与阵营3的关系仍需独立设置，因此必须确保每个阵营之间的关系（中立/敌对/结盟）都被明确配置，以保证游戏逻辑的正确性和可预测性。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/6e8f0image.png)

|属性名|属性说明|
|-|-|
|阵营设置|Camp ID：阵营的唯一ID<br>Camp Name：阵营的名称|
|阵营出生方式配置|支持按阵营设置出生点的逻辑，数组结构为 ``阵营id + 出生配置``的形式，其中0为占位阵营无法变更设置<br>- 阵营出生方式：指定出生点或者指定位置坐标<br>- 指定PlayerStartID：当 ``阵营出生方式``选为“指定PlayerStartID”时有效，对应 [``PlayerBornPointID``](https://developer.gp.qq.com/wikieditor/#/catalog/362?autoJump=%E9%85%8D%E7%BD%AE%E5%87%BA%E7%94%9F%E8%A7%84%E5%88%99)<br>- 当有多个PlayerStartID时是否随机选择：当 ``阵营出生方式``选为“指定PlayerStartID”时有效，如果关卡中放置了多个出生点，则会从这些出生点里随机选择一个<br>- 指定世界坐标：当 ``阵营出生方式``选为“指定世界坐标”时有效，填写具体的坐标值|
|阵营关系|Camp A/B ID：建立关系的阵营ID<br>Releation：具体阵营关系<br>- Same：同阵营（友方）<br>- Enemy：敌人<br>- Neutral：中立|
|默认阵营关系|所有配置的阵营，若未指定阵营关系，将使用此默认关系|
|不同队伍属于不同阵营|针对玩家角色的选项，如果勾选，则优先按队伍ID设置阵营，确保每个玩家都有初始阵营，但不保证阵营ID值一定等于队伍ID；否则，阵营ID默认全为1|

> 阵营ID相同默认为同阵营关系

---

### 设置怪物阵营

当怪物未主动配置阵营时，系统默认将其阵营ID设为0；如需调整怪物阵营，在怪物蓝图中，搜索“General Camp”，将 ``General Camp ID`` 属性设置为目标阵营ID，该怪物生成后将被分配在对应的阵营下。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/IwZvuimage.png)

如果使用了 [怪物刷新器](https://developer.gp.qq.com/wikieditor/#/catalog/20156) 生成怪物，会以 [怪物组表](https://developer.gp.qq.com/wikieditor/#/catalog/20156?autoJump=%E6%80%AA%E7%89%A9%E7%BB%84%E8%A1%A8) 里配置的阵营ID为准。

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/COOJbimage.png)

---

### 设置角色阵营

玩家角色阵营取决于 ``不同队伍属于不同阵营`` 的属性设置，若未勾选，系统默认将其阵营ID设为1；否则，将玩家队伍ID同步到阵营ID。

> ``UGCPlayerPawn`` 蓝图的 ``General Camp ID`` 属性设置无效

![image.png](https://cgugc-video-test-1258633575.cos.ap-shanghai.myqcloud.com/wiki_picture/dslerimage.png)

---

### 动态修改阵营

**玩家阵营**

如果希望在玩法进程中动态修改玩家的阵营，可以调用 [`SetCampForTeam`](https://developer.gp.qq.com/api/#/searchContent/UGCCampSystem?classDetailShow=true&path=class%2Fdetail%2F%E5%92%8C%E5%B9%B3%E5%85%A8%E5%B1%80%E6%8E%A5%E5%8F%A3%2F%E7%A4%BE%E4%BA%A4%E7%B3%BB%E7%BB%9F%2FUGCCampSystem.json&isSelect=1&apiEnc=%5B%22%E7%B1%BB%22%5D&apiLabel=UGCCampSystem&autoJump=SetCampForTeam) 进行修改，需要注意的是单个玩家无法单独设置阵营，仅能通过队伍所属阵营间接修改玩家阵营。

代码示例：

``` lua
local TargetCampID = 3
local PlayerTeamID = UGCPawnAttrSystem.GetTeamID(PlayerPawn)
UGCCampSystem.SetCampForTeam(PlayerTeamID, TargetCampID)
```

<br>

**怪物阵营**

如果希望在玩法进程中动态修改怪物的阵营，可以调用 [`SetCampForActor`](https://developer.gp.qq.com/api/#/searchContent/UGCCampSystem?classDetailShow=true&path=class%2Fdetail%2F%E5%92%8C%E5%B9%B3%E5%85%A8%E5%B1%80%E6%8E%A5%E5%8F%A3%2F%E7%A4%BE%E4%BA%A4%E7%B3%BB%E7%BB%9F%2FUGCCampSystem.json&isSelect=1&apiEnc=%5B%22%E7%B1%BB%22%5D&apiLabel=UGCCampSystem&autoJump=SetCampForActor) 进行修改。

代码示例：

``` lua
local TargetCampID = 3
local ClassPath = UGCObjectUtility.LoadClass(UGCGameSystem.GetUGCResourcesFullPath('Asset/Blueprint/Prefabs/Monsters/Boss.Boss_C'))
local BossActor = UGCGenericCharacterSystem.SpawnGenericCharacter(self, ClassPath, { X = 20460.0, Y = 29410.0, Z = 200 }, { Roll = 0, Pitch = 0, Yaw = 0 }, { X = 1, Y = 1, Z = 1 })

if BossActor then
		UGCCampSystem.SetCampForActor(BossActor, TargetCampID)
end
```

<br>

## API参考

|函数库/类名|函数类型|函数功能范围|
|-|-|-|
|[UGCTeamSystem](https://developer.gp.qq.com/api/#/searchContent/UGCTeamSystem?classDetailShow=true&path=class%2Fdetail%2F%E5%92%8C%E5%B9%B3%E5%85%A8%E5%B1%80%E6%8E%A5%E5%8F%A3%2F%E7%A4%BE%E4%BA%A4%E7%B3%BB%E7%BB%9F%2FUGCTeamSystem.json&isSelect=1&apiEnc=%5B%22%E7%B1%BB%22%5D&apiLabel=UGCTeamSystem)|静态函数|动态设置、获取玩家队伍ID，查询队友、队长等|
|[UGCCampSystem](https://developer.gp.qq.com/api/#/searchContent/UGCCampSystem?classDetailShow=true&path=class%2Fdetail%2F%E5%92%8C%E5%B9%B3%E5%85%A8%E5%B1%80%E6%8E%A5%E5%8F%A3%2F%E7%A4%BE%E4%BA%A4%E7%B3%BB%E7%BB%9F%2FUGCCampSystem.json&isSelect=1&apiEnc=%5B%22%E7%B1%BB%22%5D&apiLabel=UGCCampSystem)|静态函数|动态修改、获取阵营关系|

---
