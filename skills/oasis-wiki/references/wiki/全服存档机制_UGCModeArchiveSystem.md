# 全服存档机制（UGCModeArchiveSystem）

> 手工维护文件，不参与 `scripts/sync_official_wiki.py` 快照刷新（未列入 `README.md` 的分类表，也不会被 sync 的删除逻辑清掉）。
> 用途: 沉淀官方灰度文档《【对外版本】全服存档机制 - 灰度专用》的接口与行为；官方 Wiki 尚无独立文章，`官方API参考手册.md` 也还没有 `UGCModeArchiveSystem` 条目，官方文章上线后以 Wiki 为准复核本页。
> 对应版本: 1.39.13.16220「新增全服存档功能，支持所有 DS 局共享的全局数值数据」（见 `版本日志.md` 的 1.39 Release Notes）。
> 来源: `【对外版本】全服存档机制 - 灰度专用.docx`，40224 字节，2026-11-20 10:53 收到，2026-11-20 收录，SHA-256 `ddf58aa745778e380b13b048b3f483a1b515abb20d12a8cfbb0b3af5d1053fd4`。

## 定位

- 玩法维度的数值存档：同一玩法下**所有对局共享同一份数据**，任一玩家、任一对局写入，其余对局可见（文档口径：理论立即生效，实际受网络状况与并发限制）。
- 与玩家存档的分工：玩家存档解决"我"的问题（个人等级、资产、进度），玩法存档解决"我们"的问题（全服进度、公共资源池、赛季数据）。
- 归属是**整个玩法**，不是玩家；数据在服务器侧，客户端只能通过同步 / RPC 拿到。

## 可实现的机制

| 机制 | 典型示例 | 依赖能力 |
|---|---|---|
| 全服共享目标 | 世界 BOSS 共享血量池跨局共同削血；全服共建建筑的分段进度 | modify 原子增减 + get 读实时进度 |
| 全服累计统计 | 全服累计击杀数、累计通关次数、今日全服死亡总数 | modify 增量计数 |
| 赛季积分与重置 | 赛季积分池、全服贡献值，赛季结束自动清零 | create 时设置 `ExpireDays` |
| 跨对局持久进度 | 全服累计上交 X 个道具解锁新区域；跨局推进的阶段状态 | 用整型 key 表示阶段与阈值 |
| 全服限量与配额 | 全服每日限量 N 份；共享资源池被逐次消耗 | modify 扣减 + 用回包最新值判断是否超额 |
| 全服状态统一切换 | 全服进入"恶魔时刻"后所有对局同步生效 | 数值标志位（0/1 或阶段号） |

## 能力边界（设计前先确认）

| 维度 | 限制 |
|---|---|
| 数据类型 | 仅 `int` / `float`，不支持字符串、数组、结构体 |
| 键数量 | 单玩法最多 1000 个 key |
| 调用位置 | 仅服务器脚本生效，客户端调用直接返回失败 |
| 调用频率 | 单玩法全部接口合计约 5000 次/秒 |
| 键名长度 | 非空且不超过 128 字符 |

复杂结构要拆成多个数值 key，或用数值编码表示状态阶段。

## 接口一览（全部异步，必须消费返回值）

| 我要做的事 | 接口 | 返回值 | 关键点 |
|---|---|---|---|
| 初始化整型 key | `AsyncCreateIntKey(BizName, InitValue, ExpireDays)` | `(ErrCode)` | 只在 key 不存在时创建；已存在返回 `KeyAlreadyExists` 且原值不变 |
| 初始化浮点 key | `AsyncCreateFloatKey(BizName, InitValue, ExpireDays)` | `(ErrCode)` | 一旦用 float 接口写入，该 key 永远是 float |
| 读当前值 | `AsyncGetKey(BizName)` | `(ErrCode, Value)` | 不存在返回 `NotExist`，`Value` 为 nil；读到的一定是最新已写入值 |
| 增减（计分/扣血/消耗） | `AsyncModifyIntValue(BizName, Delta)` / `AsyncModifyFloatValue(BizName, Delta)` | `(ErrCode, NewValue)` | 返回修改后的最新值；key 不存在时从 0 起算并隐式创建（占配额） |
| 删除 key | `AsyncDeleteKey(BizName)` | `(ErrCode)` | 只删已存在的 key，不存在返回 `NotExist`；删除可释放配额 |

参数：`BizName` 非空、≤128 字符、同一玩法内唯一；`ExpireDays` 非负数，默认 0 表示永不过期（秒/天口径为"天"）。

### 四个坑

1. **类型不可逆**：key 的 int/float 在创建时确定，用过 float 接口就永久 float，之后所有 int 接口返回 `TypeMismatch`，删除重建也无法改变。
2. **不消费就不发包**：接口返回的是未驱动的异步任务，只有被 `AsyncRun` / `AwaitSafe` / `WaitForSafe` / `AwaitAll` 消费才真正执行；只调用不消费既不发包也不报错。
3. **QPS 共享**：所有接口共用同一份频次配额，"每帧调用"这类写法必然触发限流。
4. **modify 的回包就是权威值**：`NewValue` 由后端原子累加返回，天然包含其他对局的并发修改；世界 BOSS 血量场景不需要再补一次查询。

## 四种异步写法

```lua
-- 1) 回调风格
UGCAsyncUtility.AsyncRun(UGCModeArchiveSystem.AsyncGetKey("TotalKills")):Then(function(Task)
    local ErrCode, Value = Task:Get()
    if ErrCode == EUGCModeArchiveError.Success then
        -- 使用 Value
    end
end)

-- 2) await 风格（推荐）：在自己的异步函数体内写同步代码
local LoadModeArchive = UGCAsyncUtility.AsyncDefine(function()
    local Ok, ErrCode, Value = UGCAsyncUtility.AwaitSafe(UGCModeArchiveSystem.AsyncGetKey("TotalKills"))
    if Ok and ErrCode == EUGCModeArchiveError.Success then
        return Value
    end
    return nil
end)
UGCAsyncUtility.AsyncRun(LoadModeArchive())

-- 3) 超时控制（不设超时引擎也会在 10 秒后放弃）
local Ok, ErrCode, Value = UGCAsyncUtility.WaitForSafe(UGCModeArchiveSystem.AsyncGetKey("TotalKills"), 5)

-- 4) 开局批量并发（AwaitAll）：总耗时约等于最慢的那个
local BatchLoad = UGCAsyncUtility.AsyncDefine(function()
    local Results = UGCAsyncUtility.AwaitAll({
        UGCModeArchiveSystem.AsyncGetKey("TotalKills"),
        UGCModeArchiveSystem.AsyncGetKey("TotalDeaths"),
        UGCModeArchiveSystem.AsyncGetKey("SeasonScore"),
    })
    local ErrCode1, TotalKills = table.unpack(Results[1])
    local ErrCode2, TotalDeaths = table.unpack(Results[2])
    local ErrCode3, SeasonScore = table.unpack(Results[3])
    return TotalKills, TotalDeaths, SeasonScore
end)
UGCAsyncUtility.AsyncRun(BatchLoad())
```

## 错误码（EUGCModeArchiveError）

本地错误（负值，请求未发出或本地兜底）：

| 常量 | 值 | 触发场景 | 处理建议 |
|---|---:|---|---|
| `Success` | 0 | 成功 | — |
| `LocalNotServer` | -1 | 在客户端调用 | 检查调用位置，接口仅服务器生效 |
| `LocalInvalidParam` | -2 | 参数校验失败（键名或数值非法） | 检查入参类型与范围 |
| `LocalNotSupported` | -3 | UGCMobile 环境 | 本接口仅限 UGC 端游 |
| `LocalTimeout` | -4 | 本地 10 秒兜底超时 | 可重试 |

后台错误（正值）：

| 常量 | 值 | 触发场景 | 处理建议 |
|---|---:|---|---|
| `InvalidParam` | 9935128 | 后台参数校验失败 | 检查入参 |
| `DbError` | 9935129 | 存储操作失败 | 可重试；连续出现联系平台 |
| `KeyLimitExceeded` | 9935130 | key 数量超上限（1000/玩法） | 删除不用的 key 或精简设计 |
| `RateLimited` | 9935131 | 触发频次限流 | 降频、合并请求、退避重试 |
| `NotExist` | 9935132 | get / delete 的 key 不存在 | 先创建，或忽略 |
| `KeyAlreadyExists` | 9935133 | create 时 key 已存在 | 属正常分支，非异常 |
| `TypeMismatch` | 9935135 | int / float 混用 | 该 key 已永久为 float，改用 float 接口 |

注意：9935134 在文档中未定义，别自己补常量名。

## 容量与限流

| 限制 | 值 | 超限表现 |
|---|---|---|
| 单玩法调用频次 | 约 5000 次/秒（所有接口合计） | `RateLimited` |
| 单玩法 key 数量 | 1000 个 | 新增返回 `KeyLimitExceeded`（删除可释放配额） |
| 单 key 键名长度 | 128 字符 | `LocalInvalidParam` |

计分类高频写入用 modify（原子累加，天然防并发覆盖），但所有接口共享配额："BOSS 每次受击都发一次 modify"这类写法要评估总量，必要时在玩法侧做本地节流合并（例如每 0.5 秒汇总一次伤害再写入）。

## 编辑器 PIE 联调

- PIE 下自动使用**本地模拟后端**，无需真实后台，行为与线上一致（类型标记、key 上限、过期、隐式创建、modify 返回最新值等）。
- 数据存在编辑器进程内存里，**重启 PIE 会清空**，只能验证逻辑，不能验证持久化。
- 数据按玩法隔离，同一编辑器内切换不同玩法工程互不串扰。

## 实践建议

推荐：

- 开局用 `AwaitAll` 一次批量拉取所需 key，不要循环单个查询。
- 需要"修改后的最新值"直接用 modify 的回包 `NewValue`，不要改完再查一次（多耗配额、多一次往返）。
- 计分类高频写入用 modify 增量；低频配置类读写用 create / get。
- key 命名带业务前缀（如 `Rank.TotalKills`、`Shop.DailyPool`），方便管理 1000 个配额。
- 所有调用都检查 `ErrCode`，至少区分成功、可重试（`RateLimited`、`LocalTimeout`）、不可重试（`TypeMismatch`、`LocalInvalidParam`）。

避免：

- 同一 key 在 float / int 接口之间混用。
- 每帧或高频循环调用。
- 只调用接口而不消费返回的异步任务。

## 常见问题（文档口径）

- **客户端能调用吗？** 不能，返回 `LocalNotServer` 且不发包。客户端需要数据，请服务器读取后通过同步或 RPC 传过去。
- **多个对局同时改同一 key 会互相覆盖吗？** 不会。modify 是增量叠加，任何到达顺序最终结果一致，各对局拿到的 `NewValue` 都是该次修改生效后的最新值。
- **能存字符串 / 数组 / 结构体吗？** 不能，只有 int / float。
- **能做排行榜吗？** 不能直接做。系统只存单个数值，不提供排序查询；排行榜要玩法侧自行维护（例如按分段分桶计数）。
- **调用了没反应怎么查？** ① 是否在服务器脚本；② 是否消费了异步任务；③ 参数是否被本地校验拦截（日志搜 `UGCModeArchiveSystem`）；④ 是否 10 秒兜底超时（`LocalTimeout`）。

## 文档未覆盖 / 落地时要自己定的点

- **超发回滚**：全服限量用"modify 扣减 + 回包判断是否超额"，文档没有原子预扣语义；判断为超额的调用要自己按负 Delta 回滚，并发下仍有短时超发。
- **赛季重置**：只有 `ExpireDays` 到期清零这一种手段，没有"立即重置赛季"的接口（要重置就 delete 再 create，注意 delete 期间读到 `NotExist`）。
- **强一致时机**：跨局可见性受网络与并发影响，不能当强一致读用；对时序敏感的判定（如世界 BOSS 击杀归属）应在服务端单点结算。
- **持久化验证**：PIE 的内存模拟验证不了持久化，必须跑真实灰度后台。
