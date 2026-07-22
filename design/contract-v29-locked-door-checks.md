# Contract v29：锁门、开锁检定与破门

状态：协议 1.29 / contract-v29 active baseline

## 已完成边界

- `TerrainDefinition` 新增可选 `openCheckDifficulty`、`bashToTerrainId` 与 `bashCheckDifficulty`。开锁难度必须依附于可开启地形；破门目标与难度必须成对声明，源地形必须不可行走并阻光，目标地形必须可行走且透光。
- `ActorDefinition` 新增明确的 `doorSkill` 与 `bashPower` 基础能力。原创探索者分别声明 24 与 30，不借用前端显示值，也不把开锁能力隐藏在攻击/防御字段中。
- 原创内容新增 `demo.terrain.door-locked` 与 `demo.terrain.door-broken`。程序化层首次生成锁门；成功开锁后转换为普通开启门，之后可关成普通未锁门；成功破门后转换为不可关闭的破损门。
- `OpenDoor` 遇到带 `openCheckDifficulty` 的门时调用 `CheckKind::UnlockDoor`；协议新增 `BashDoor { direction }`，调用 `CheckKind::BashDoor`。两者都复用 `CheckContext` / `CheckResult` 和既有 5% 自动成功、5% 自动失败、整数 contest 规则。
- 固定检定顺序为 percentile roll，非自动结果再进行 ability roll。失败保持原 terrain ID；开锁成功按 `terrain.door-unlocked`、`terrain.door-opened` 顺序输出事件；破门成功输出 `terrain.door-bashed-open`。
- 所有成功、失败和 unavailable 命令继续使用标准行动成本。目标格被怪物或地面物品占据时不执行破门；当前不会把门命令隐式改成近战。
- 门仍无平行运行时状态。锁定、普通开启、普通关闭和破损都由稳定 terrain ID 表示，因此活动层、离层 `FloorState`、save v1、回放和 state hash Schema v14 自然覆盖。
- 前端增加 `B → 方向` 破门，与 `O → 方向`、`C → 方向` 共用两步方向输入和 Esc 取消；中英文 Fluent 文案与 TypeScript 命令测试同步更新。

## 协议、内容、存档与基线

- 协议升至 1.29；内容包升至 1.24.0，content hash 为 `2d2900d8052b0a600346d0b87cc3b3d5bb5138f851abbf2b95afa196bbbaaca2`，terrain 数量增至 8。
- save schema 继续为 v1，state hash 继续为 Schema v14。新增能力和难度属于内容定义，门结果继续落在既有 terrain ID 数组中。
- v28 存档继续可读。已经访问的旧程序化层不会被补成锁门；尚未访问的程序化层首次生成时使用 v29 的锁门内容。
- contract-v29 从 v28 迁移 65 个 exact fixtures，并新增“首次撞门失败、第二次自动成功、破损门不可关闭、存档往返”场景，共 66 个。

## 与原版 RFB v1.3.0.7 对比

原版 `src/cmd2.c` 的 `do_cmd_open_aux()` 从 feature `power` 读取锁强度，以玩家 `skills.dis` 为基础；盲目、无光、混乱或幻觉会把技能降为十分之一，再以 `skill - power * 4` 得到成功率并保留最小成功机会。失败允许命令重复。`do_cmd_bash_aux()` 以力量表派生 bash power，对门 power 进行百分比检定；成功后可能进入 `FF_BASH` 破损态或普通 `FF_OPEN` 开启态，并让玩家穿过门洞；失败还可能失去平衡并短暂麻痹。

相同点：

- 锁门具有内容声明的难度，玩家使用独立于普通攻击的能力进行开锁；
- 开锁和破门都是面向相邻方向、消耗行动的独立命令；
- 检定失败时门保持关闭，可以在后续回合再次尝试；
- 检定始终保留低概率自动成功，普通结果由玩家能力与门难度共同决定；
- 破门成功通过地形 feature/terrain 状态转换立即改变碰撞与视线，并随楼层保存。

主动差异：

- 重构把 `doorSkill`、`bashPower`、开锁难度和破门难度放入强类型内容字段，并通过统一 `CheckContext` / `CheckResult` 记录检定上下文；原版直接组合全局玩家状态、feature power 与 `randint0()`。
- 重构检定使用跨平台固定 RNG 与固定抽取顺序，并进入 replay checkpoint、state hash 和 exact contract；原版不提供协议级回放与哈希验证。
- 当前开锁成功固定转为普通开启门，破门成功固定转为不可关闭的破损门；原版破门成功还会再随机决定“破损”或“普通开启”，玻璃门等分支另有规则。
- 当前破门成功不会自动把玩家移动进门洞；原版会调用 `move_player()` 穿过门。
- 当前不实现盲目、无光、混乱、幻觉修正，不奖励开锁经验，不播放声音，不自动重复，也不实现撞门失败后的失衡/麻痹。
- 原版目标格有怪物时会转为近战攻击；重构保持命令类型稳定并返回 unavailable。
- 原版还覆盖卡死门、秘密门、玻璃门、箱子、特殊种族/职业和 easy-open；这些没有塞入本纵切。

本切片复刻的是“锁难度 + 角色能力 + 可重复失败 + 独立破门 + 地形结果持久化”的核心关系，并主动保持确定性协议边界。

## 下一步

权威相邻地形交互查询已由 [contract-v30](contract-v30-authoritative-terrain-interactions.md) 完成：核心按稳定顺序输出动作、方向、检定需求和占用原因，前端不再盲目提交门命令。下一纵切进入秘密门与搜索；陷阱、解除和挖掘随后继续拆分。
